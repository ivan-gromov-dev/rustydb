mod clock;
mod in_memory;
mod indexing;
mod stored_value;
mod value;

pub(crate) use in_memory::InMemoryStore;

#[cfg(test)]
mod tests;
