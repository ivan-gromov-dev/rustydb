mod metadata;
mod parser;
mod types;

pub(crate) use metadata::{COMMANDS, CommandMetadata, command_metadata};
pub(crate) use types::{
    ClientInfoAttribute, Command, CommandError, ExpireCondition, GetExExpiration, ProtocolVersion,
    SetCondition, SetExpiration,
};

#[cfg(test)]
mod tests;
