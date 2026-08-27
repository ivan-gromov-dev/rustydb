mod metadata;
mod parser;
mod types;

pub(crate) use metadata::{COMMANDS, CommandMetadata, command_metadata};
pub(crate) use types::{ClientInfoAttribute, Command, CommandError, ProtocolVersion};

#[cfg(test)]
mod tests;
