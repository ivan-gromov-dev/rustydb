use std::path::{Path, PathBuf};

use crate::command::Command;
use crate::executor::execute_with_snapshot;
use crate::output::CommandOutput;
use crate::snapshot::{self, SnapshotError};
use crate::storage::InMemoryStore;

pub(crate) struct Database {
    store: InMemoryStore,
    snapshot_path: Option<PathBuf>,
}

impl Database {
    pub(crate) fn execute(&mut self, command: Command) -> CommandOutput {
        execute_with_snapshot(command, &mut self.store, self.snapshot_path.as_deref())
    }

    pub(crate) fn open(snapshot_path: impl AsRef<Path>) -> Result<Self, SnapshotError> {
        let snapshot_path = snapshot_path.as_ref().to_owned();
        let mut store = InMemoryStore::new();
        snapshot::load(&snapshot_path, &mut store)?;

        Ok(Self {
            store,
            snapshot_path: Some(snapshot_path),
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
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
