use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::aof::{Aof, AofError};
use crate::command::Command;
use crate::config::MemoryConfig;
use crate::executor::execute_with_snapshot;
use crate::output::CommandOutput;
use crate::snapshot::{self, SnapshotError};
use crate::storage::InMemoryStore;

pub(crate) struct Database {
    store: InMemoryStore,
    snapshot_path: Option<PathBuf>,
    aof: Option<Aof>,
}

impl Database {
    pub(crate) fn active_expire(&mut self, limit: usize) -> usize {
        self.store.active_expire(limit)
    }

    pub(crate) fn execute(&mut self, command: Command) -> CommandOutput {
        if command == Command::AofRewrite {
            return self.rewrite_aof();
        }
        let aof_arguments = command.aof_arguments();
        let output = execute_with_snapshot(command, &mut self.store, self.snapshot_path.as_deref());
        let evicted_keys = self.store.take_evicted_keys();
        if !output.is_error()
            && let Some(aof) = &mut self.aof
        {
            if let Some(arguments) = aof_arguments
                && let Err(error) = aof.append(&arguments)
            {
                return CommandOutput::Error(format!("AOF append failed: {error}"));
            }
            for key in evicted_keys {
                if let Err(error) = aof.append(&[b"DEL".to_vec(), key]) {
                    return CommandOutput::Error(format!("AOF append failed: {error}"));
                }
            }
        }
        output
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
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
