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

impl RequestError {
    pub(crate) fn response_message(&self) -> String {
        match self {
            Self::InvalidCommand(CommandError::UnsupportedProtocol(_)) => {
                format!("NOPROTO {self}")
            }
            _ => format!("ERR {self}"),
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

    for (index, element) in elements.iter().enumerate() {
        if !matches!(element, RespFrame::BulkString(_)) {
            return Err(RequestError::ExpectedBulkString { index });
        }
    }
    let arguments = elements
        .into_iter()
        .filter_map(|element| match element {
            RespFrame::BulkString(value) => Some(value),
            _ => None,
        })
        .collect();

    Command::from_owned_bytes(arguments).map_err(RequestError::InvalidCommand)
}
