use std::fmt;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Set {
        key: String,
        value: String,
    },
    MSet {
        entries: Vec<(String, String)>,
    },
    SetNx {
        key: String,
        value: String,
    },
    Get {
        key: String,
    },
    MGet {
        keys: Vec<String>,
    },
    GetSet {
        key: String,
        value: String,
    },
    GetDel {
        key: String,
    },
    Append {
        key: String,
        append_value: String,
    },
    Increment {
        key: String,
    },
    IncrementBy {
        key: String,
        amount: i64,
    },
    Decrement {
        key: String,
    },
    DecrementBy {
        key: String,
        amount: i64,
    },
    IncrementByFloat {
        key: String,
        amount: f64,
    },
    Exists {
        keys: Vec<String>,
    },
    Delete {
        keys: Vec<String>,
    },
    Rename {
        old_key: String,
        new_key: String,
    },
    Expire {
        key: String,
        seconds: u64,
    },
    PExpire {
        key: String,
        milliseconds: u64,
    },
    Ttl {
        key: String,
    },
    PTtl {
        key: String,
    },
    Persist {
        key: String,
    },
    StrLen {
        key: String,
    },
    GetRange {
        key: String,
        start: i64,
        end: i64,
    },
    SetRange {
        key: String,
        offset: usize,
        value: String,
    },
    LPush {
        key: String,
        value: String,
    },
    RPush {
        key: String,
        value: String,
    },
    LLen {
        key: String,
    },
    LPop {
        key: String,
    },
    RPop {
        key: String,
    },
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
    InvalidFloat(String),
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
            Self::InvalidFloat(usage) => {
                write!(formatter, "invalid float: {usage}")
            }
        }
    }
}
