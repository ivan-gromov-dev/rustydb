mod clock;
mod in_memory;
mod indexing;
mod snapshot;
mod stored_value;
mod value;

pub(crate) use in_memory::InMemoryStore;
pub(crate) use snapshot::{SnapshotDataError, SnapshotEntry, SnapshotValue};

#[cfg(test)]
mod tests;
