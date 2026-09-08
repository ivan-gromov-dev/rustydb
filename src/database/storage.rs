use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::aof::{Aof, AofError};
use crate::command::{Command, SetCondition};
use crate::config::MemoryConfig;
use crate::executor::execute_with_snapshot;
use crate::logging::{self, LogLevel};
use crate::output::CommandOutput;
use crate::snapshot::{self, SnapshotError};
use crate::storage::InMemoryStore;

pub(crate) struct Database {
    store: InMemoryStore,
    snapshot_path: Option<PathBuf>,
    aof: Option<Aof>,
    metrics: DatabaseMetrics,
    key_versions: HashMap<Vec<u8>, u64>,
    next_key_version: u64,
    transaction_records: Option<Vec<Vec<Vec<u8>>>>,
}

#[derive(Default)]
struct DatabaseMetrics {
    connected_clients: u64,
    total_connections: u64,
    commands_processed: u64,
    keyspace_hits: u64,
    keyspace_misses: u64,
    persistence_successes: u64,
    persistence_failures: u64,
}

impl Database {
    pub(crate) fn try_blocking_pop(&mut self, keys: &[Vec<u8>], right: bool) -> CommandOutput {
        let key = match self.store.first_nonempty_list(keys) {
            Ok(Some(key)) => key,
            Ok(None) => return CommandOutput::Nil,
            Err(error) => return CommandOutput::Error(error.to_string()),
        };
        let command = if right {
            Command::RPop { key: key.clone() }
        } else {
            Command::LPop { key: key.clone() }
        };
        match self.execute(command) {
            CommandOutput::Value(value) => CommandOutput::KeyList(vec![key, value]),
            output => output,
        }
    }

    pub(crate) fn try_blocking_move(
        &mut self,
        source: Vec<u8>,
        destination: Vec<u8>,
        source_end: crate::command::ListEnd,
        destination_end: crate::command::ListEnd,
    ) -> CommandOutput {
        match self
            .store
            .first_nonempty_list(std::slice::from_ref(&source))
        {
            Ok(Some(_)) => {}
            Ok(None) => return CommandOutput::Nil,
            Err(error) => return CommandOutput::Error(error.to_string()),
        }
        self.execute(Command::LMove {
            source,
            destination,
            source_end,
            destination_end,
        })
    }

    pub(crate) fn active_expire(&mut self, limit: usize) -> usize {
        self.store.active_expire(limit)
    }

    pub(crate) fn execute(&mut self, command: Command) -> CommandOutput {
        let command_name = command.name();
        let output = self.execute_inner(command);
        let status = if output.is_error() { "error" } else { "ok" };
        let level = if output.is_error() {
            LogLevel::Error
        } else {
            LogLevel::Info
        };
        logging::event(
            level,
            "command_completed",
            &[("command", command_name), ("status", status)],
        );
        output
    }

    fn execute_inner(&mut self, command: Command) -> CommandOutput {
        self.metrics.commands_processed = self.metrics.commands_processed.saturating_add(1);
        if let Command::Watch { keys } = command {
            let versions = keys
                .into_iter()
                .map(|key| {
                    let exists = self.store.exists(&key);
                    let version = *self.key_versions.entry(key.clone()).or_insert(0);
                    (key, version, exists)
                })
                .collect();
            return CommandOutput::WatchVersions(versions);
        }
        if let Command::Transaction { commands, watched } = command {
            let changed = watched.into_iter().any(|(key, version, existed)| {
                self.key_versions.get(&key).copied().unwrap_or(0) != version
                    || self.store.exists(&key) != existed
            });
            if changed {
                return CommandOutput::NullArray;
            }
            let collects_aof = self.aof.is_some() && self.transaction_records.is_none();
            if collects_aof {
                self.transaction_records = Some(Vec::new());
            }
            let outputs = commands
                .into_iter()
                .map(|command| self.execute_inner(command))
                .collect();
            if collects_aof {
                let records = self.transaction_records.take().unwrap_or_default();
                if !records.is_empty() {
                    if let Some(aof) = &mut self.aof {
                        if let Err(error) = aof.append_transaction(&records) {
                            self.metrics.persistence_failures =
                                self.metrics.persistence_failures.saturating_add(1);
                            return CommandOutput::Error(format!(
                                "AOF transaction append failed: {error}"
                            ));
                        }
                        self.metrics.persistence_successes =
                            self.metrics.persistence_successes.saturating_add(1);
                    }
                }
            }
            return CommandOutput::Transaction(outputs);
        }
        if command == Command::Info {
            return CommandOutput::Value(self.info().into_bytes());
        }
        if command == Command::AofRewrite {
            let output = self.rewrite_aof();
            self.record_persistence(&output);
            return output;
        }
        let lookup = command.lookup_size();
        let records_snapshot_save = command == Command::Save;
        let aof_requires_integer_one = matches!(
            command,
            Command::ExpireAdvanced { .. }
                | Command::PExpireAdvanced { .. }
                | Command::ExpireAt {
                    condition: Some(_),
                    ..
                }
                | Command::PExpireAt {
                    condition: Some(_),
                    ..
                }
                | Command::Copy { .. }
                | Command::SMove { .. }
        );
        let aof_should_append = match &command {
            Command::SetAdvanced { key, condition, .. } => match condition {
                Some(SetCondition::IfAbsent) => !self.store.exists(key),
                Some(SetCondition::IfPresent) => self.store.exists(key),
                None => true,
            },
            Command::MSetNx { entries } => entries.iter().all(|(key, _)| !self.store.exists(key)),
            _ => true,
        };
        let aof_arguments = command.aof_arguments();
        let mutation_keys = aof_arguments.as_deref().map(command_keys);
        let output = execute_with_snapshot(command, &mut self.store, self.snapshot_path.as_deref());
        if let Some(total) = lookup {
            let hits = match &output {
                CommandOutput::Value(_) => 1,
                CommandOutput::Nil => 0,
                CommandOutput::OptionalValues(values) => {
                    values.iter().filter(|value| value.is_some()).count()
                }
                CommandOutput::Integer(value) => usize::try_from(*value).unwrap_or(0).min(total),
                _ => 0,
            };
            self.metrics.keyspace_hits = self.metrics.keyspace_hits.saturating_add(hits as u64);
            self.metrics.keyspace_misses = self
                .metrics
                .keyspace_misses
                .saturating_add((total - hits) as u64);
        }
        let evicted_keys = self.store.take_evicted_keys();
        if !output.is_error() {
            if let Some(keys) = mutation_keys {
                if keys.is_empty() {
                    self.bump_all_key_versions();
                } else {
                    for key in keys {
                        self.bump_key_version(key);
                    }
                }
            }
        }
        for key in &evicted_keys {
            self.bump_key_version(key.clone());
        }
        if !output.is_error()
            && (!aof_requires_integer_one || output == CommandOutput::Integer(1))
            && aof_should_append
            && self.aof.is_some()
        {
            let mut records = Vec::new();
            if let Some(arguments) = aof_arguments {
                records.push(arguments);
            }
            for key in evicted_keys {
                records.push(vec![b"DEL".to_vec(), key]);
            }
            if let Some(transaction_records) = &mut self.transaction_records {
                transaction_records.extend(records);
            } else if let Some(aof) = &mut self.aof {
                for arguments in records {
                    if let Err(error) = aof.append(&arguments) {
                        self.metrics.persistence_failures =
                            self.metrics.persistence_failures.saturating_add(1);
                        return CommandOutput::Error(format!("AOF append failed: {error}"));
                    }
                    self.metrics.persistence_successes =
                        self.metrics.persistence_successes.saturating_add(1);
                }
            }
        }
        if records_snapshot_save {
            self.record_persistence(&output);
        }
        output
    }

    pub(crate) fn client_connected(&mut self) {
        self.metrics.connected_clients = self.metrics.connected_clients.saturating_add(1);
        self.metrics.total_connections = self.metrics.total_connections.saturating_add(1);
    }

    pub(crate) fn client_disconnected(&mut self) {
        self.metrics.connected_clients = self.metrics.connected_clients.saturating_sub(1);
    }

    fn record_persistence(&mut self, output: &CommandOutput) {
        if output.is_error() {
            self.metrics.persistence_failures = self.metrics.persistence_failures.saturating_add(1);
        } else {
            self.metrics.persistence_successes =
                self.metrics.persistence_successes.saturating_add(1);
        }
    }

    fn info(&self) -> String {
        let reclamation = self.store.reclamation_metrics();
        format!(
            "connected_clients:{}\ntotal_connections:{}\ncommands_processed:{}\nkeyspace_hits:{}\nkeyspace_misses:{}\nexpired_keys:{}\nevicted_keys:{}\npersistence_successes:{}\npersistence_failures:{}\n",
            self.metrics.connected_clients,
            self.metrics.total_connections,
            self.metrics.commands_processed,
            self.metrics.keyspace_hits,
            self.metrics.keyspace_misses,
            reclamation.expirations,
            reclamation.evictions,
            self.metrics.persistence_successes,
            self.metrics.persistence_failures,
        )
    }

    fn rewrite_aof(&mut self) -> CommandOutput {
        let Some(aof) = &mut self.aof else {
            return CommandOutput::Error("AOF is not configured".to_owned());
        };
        let wall_now = SystemTime::now();
        let entries = match self.store.snapshot_entries(wall_now) {
            Ok(entries) => entries,
            Err(error) => return CommandOutput::Error(format!("AOF rewrite failed: {error}")),
        };
        match aof.rewrite(&entries, wall_now) {
            Ok(()) => CommandOutput::Ok,
            Err(error) => CommandOutput::Error(format!("AOF rewrite failed: {error}")),
        }
    }

    #[cfg(test)]
    pub(crate) fn open(snapshot_path: impl AsRef<Path>) -> Result<Self, SnapshotError> {
        Self::open_with_config(snapshot_path, MemoryConfig::default())
    }

    pub(crate) fn open_with_config(
        snapshot_path: impl AsRef<Path>,
        memory_config: MemoryConfig,
    ) -> Result<Self, SnapshotError> {
        let snapshot_path = snapshot_path.as_ref().to_owned();
        let mut store = InMemoryStore::with_max_keys(memory_config.max_keys());
        snapshot::load(&snapshot_path, &mut store)?;
        store.enforce_key_limit();

        Ok(Self {
            store,
            snapshot_path: Some(snapshot_path),
            aof: None,
            metrics: DatabaseMetrics::default(),
            key_versions: HashMap::new(),
            next_key_version: 1,
            transaction_records: None,
        })
    }

    pub(crate) fn open_aof_with_config(
        path: impl AsRef<Path>,
        memory_config: MemoryConfig,
    ) -> Result<Self, AofError> {
        let (mut aof, commands) = Aof::open(path.as_ref())?;
        let mut store = InMemoryStore::with_max_keys(memory_config.max_keys());
        let mut replay_evictions = Vec::new();
        for command in commands {
            let output = execute_replay(command, &mut store);
            if output.is_error() {
                return Err(AofError::InvalidCommand(format!(
                    "replay failed: {output:?}"
                )));
            }
            replay_evictions.extend(store.take_evicted_keys());
        }
        replay_evictions.sort();
        replay_evictions.dedup();
        for key in replay_evictions
            .into_iter()
            .filter(|key| !store.contains_stored_key(key))
        {
            aof.append(&[b"DEL".to_vec(), key])?;
        }
        Ok(Self {
            store,
            snapshot_path: None,
            aof: Some(aof),
            metrics: DatabaseMetrics::default(),
            key_versions: HashMap::new(),
            next_key_version: 1,
            transaction_records: None,
        })
    }

    pub(crate) fn save(&mut self) -> Result<(), SnapshotError> {
        let path = self
            .snapshot_path
            .as_deref()
            .ok_or(SnapshotError::NotConfigured)?;
        snapshot::save(path, &mut self.store)
    }

    fn new() -> Self {
        Database {
            store: InMemoryStore::new(),
            snapshot_path: None,
            aof: None,
            metrics: DatabaseMetrics::default(),
            key_versions: HashMap::new(),
            next_key_version: 1,
            transaction_records: None,
        }
    }

    fn bump_key_version(&mut self, key: Vec<u8>) {
        let version = self.next_key_version;
        self.next_key_version = self.next_key_version.saturating_add(1);
        self.key_versions.insert(key, version);
    }

    fn bump_all_key_versions(&mut self) {
        let version = self.next_key_version;
        self.next_key_version = self.next_key_version.saturating_add(1);
        for current in self.key_versions.values_mut() {
            *current = version;
        }
    }
}

fn execute_replay(command: Command, store: &mut InMemoryStore) -> CommandOutput {
    if let Command::Transaction { commands, .. } = command {
        return CommandOutput::Transaction(
            commands
                .into_iter()
                .map(|command| execute_replay(command, store))
                .collect(),
        );
    }
    execute_with_snapshot(command, store, None)
}

fn command_keys(arguments: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let Some(name) = arguments.first() else {
        return Vec::new();
    };
    if name.eq_ignore_ascii_case(b"COPY") {
        return arguments.get(2).cloned().into_iter().collect();
    }
    if name.eq_ignore_ascii_case(b"SDIFFSTORE")
        || name.eq_ignore_ascii_case(b"SINTERSTORE")
        || name.eq_ignore_ascii_case(b"SUNIONSTORE")
    {
        return arguments.get(1).cloned().into_iter().collect();
    }
    let Some(metadata) = crate::command::command_metadata(name) else {
        return Vec::new();
    };
    if metadata.first_key == 0 {
        return Vec::new();
    }
    let Ok(first) = usize::try_from(metadata.first_key) else {
        return Vec::new();
    };
    let last = if metadata.last_key < 0 {
        arguments
            .len()
            .saturating_sub(metadata.last_key.unsigned_abs() as usize)
    } else {
        usize::try_from(metadata.last_key).unwrap_or(first)
    };
    let step = usize::try_from(metadata.key_step).unwrap_or(1).max(1);
    (first..=last)
        .step_by(step)
        .filter_map(|index| arguments.get(index).cloned())
        .collect()
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
