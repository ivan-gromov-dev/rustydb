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
        if !output.is_error()
            && aof_should_append
            && let Some(aof) = &mut self.aof
        {
            if let Some(arguments) = aof_arguments.as_ref()
                && let Err(error) = aof.append(arguments)
            {
                self.metrics.persistence_failures =
                    self.metrics.persistence_failures.saturating_add(1);
                return CommandOutput::Error(format!("AOF append failed: {error}"));
            } else if aof_arguments.is_some() {
                self.metrics.persistence_successes =
                    self.metrics.persistence_successes.saturating_add(1);
            }
            for key in evicted_keys {
                if let Err(error) = aof.append(&[b"DEL".to_vec(), key]) {
                    self.metrics.persistence_failures =
                        self.metrics.persistence_failures.saturating_add(1);
                    return CommandOutput::Error(format!("AOF append failed: {error}"));
                }
                self.metrics.persistence_successes =
                    self.metrics.persistence_successes.saturating_add(1);
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
            let output = execute_with_snapshot(command, &mut store, None);
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
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
