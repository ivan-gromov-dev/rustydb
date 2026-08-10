use crate::command::{Command, CommandError};
use crate::database::Database;
use crate::output::CommandOutput;

pub(crate) fn process_line(database: &mut Database, input: &str) -> Option<CommandOutput> {
    match Command::parse(input) {
        Ok(command) => Some(database.execute(command)),
        Err(CommandError::EmptyInput) => None,
        Err(error) => Some(CommandOutput::Error(error.to_string())),
    }
}

#[cfg(test)]
mod tests;
