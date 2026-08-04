use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Set { key: String, value: String },
    Mset { entries: Vec<(String, String)> },
    Get { key: String },
    MGet { keys: Vec<String> },
    Append { key: String, append_value: String },
    Increment { key: String },
    IncrementBy { key: String, inc_value: i64 },
    Exists { key: String },
    Delete { key: String },
    Rename { old_key: String, new_key: String },
    Keys,
    Len,
    Clear,
    Help,
    Exit,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CommandError {
    EmptyInput,
    InvalidArguments(&'static str),
    UnknownCommand(String),
    InvalidInteger(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(formatter, "empty command"),
            Self::InvalidArguments(usage) => {
                write!(formatter, "usage: {usage}")
            }
            Self::UnknownCommand(command) => {
                write!(formatter, "unknown command: {command}")
            }
            Self::InvalidInteger(usage) => {
                write!(formatter, "invalid integer: {usage}")
            }
        }
    }
}
