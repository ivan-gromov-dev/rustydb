use std::path::{Path, PathBuf};

use crate::aof::{Aof, AofError};
use crate::command::Command;
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
    pub(crate) fn execute(&mut self, command: Command) -> CommandOutput {
        let aof_arguments = command.aof_arguments();
        let output = execute_with_snapshot(command, &mut self.store, self.snapshot_path.as_deref());
        if !output.is_error()
            && let (Some(aof), Some(arguments)) = (&mut self.aof, aof_arguments)
            && let Err(error) = aof.append(&arguments)
        {
            return CommandOutput::Error(format!("AOF append failed: {error}"));
        }
        output
    }

    pub(crate) fn open(snapshot_path: impl AsRef<Path>) -> Result<Self, SnapshotError> {
        let snapshot_path = snapshot_path.as_ref().to_owned();
        let mut store = InMemoryStore::new();
        snapshot::load(&snapshot_path, &mut store)?;

        Ok(Self {
            store,
            snapshot_path: Some(snapshot_path),
            aof: None,
        })
    }

    pub(crate) fn open_aof(path: impl AsRef<Path>) -> Result<Self, AofError> {
        let (aof, commands) = Aof::open(path.as_ref())?;
        let mut store = InMemoryStore::new();
        for command in commands {
            let output = execute_with_snapshot(command, &mut store, None);
            if output.is_error() {
                return Err(AofError::InvalidCommand(format!(
                    "replay failed: {output:?}"
                )));
            }
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
