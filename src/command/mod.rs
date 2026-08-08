mod arguments;
mod parser;
mod types;

pub(crate) use types::{Command, CommandError};

#[cfg(test)]
mod tests;
