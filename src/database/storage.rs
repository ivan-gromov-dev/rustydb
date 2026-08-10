use crate::command::Command;
use crate::executor::execute;
use crate::output::CommandOutput;
use crate::storage::InMemoryStore;

pub(crate) struct Database {
    store: InMemoryStore,
}

impl Database {
    pub(crate) fn execute(&mut self, command: Command) -> CommandOutput {
        execute(command, &mut self.store)
    }

    fn new() -> Self {
        Database {
            store: InMemoryStore::new(),
        }
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
