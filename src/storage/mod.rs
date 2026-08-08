mod in_memory;
mod indexing;
mod stored_value;

pub(crate) use in_memory::InMemoryStore;

#[cfg(test)]
mod tests;
