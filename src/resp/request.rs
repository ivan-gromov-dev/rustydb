use std::fmt;

use crate::command::{Command, CommandError};

use super::frame::RespFrame;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RequestError {
    ExpectedArray,
    EmptyArray,
    ExpectedBulkString { index: usize },
    InvalidCommand(CommandError),
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedArray => write!(formatter, "expected a RESP array command"),
            Self::EmptyArray => write!(formatter, "command array cannot be empty"),
            Self::ExpectedBulkString { index } => {
                write!(formatter, "command argument {index} must be a bulk string")
            }
            Self::InvalidCommand(error) => error.fmt(formatter),
        }
    }
}

pub(crate) fn command_from_frame(frame: RespFrame) -> Result<Command, RequestError> {
    let RespFrame::Array(elements) = frame else {
        return Err(RequestError::ExpectedArray);
    };

    if elements.is_empty() {
        return Err(RequestError::EmptyArray);
    }

    let arguments = elements
        .iter()
        .enumerate()
        .map(|(index, element)| match element {
            RespFrame::BulkString(value) => Ok(value.as_slice()),
            _ => Err(RequestError::ExpectedBulkString { index }),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Command::from_bytes(&arguments).map_err(RequestError::InvalidCommand)
}
