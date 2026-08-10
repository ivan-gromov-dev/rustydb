use crate::command::{Command, CommandError};
use crate::output::CommandOutput;

#[derive(Debug, PartialEq)]
pub(crate) enum ParsedLine {
    Empty,
    Command(Command),
    Error(CommandOutput),
}

pub(crate) fn parse_line(input: &str) -> ParsedLine {
    match Command::parse(input) {
        Ok(command) => ParsedLine::Command(command),
        Err(CommandError::EmptyInput) => ParsedLine::Empty,
        Err(error) => ParsedLine::Error(CommandOutput::Error(error.to_string())),
    }
}

#[cfg(test)]
mod tests;
