use std::fmt;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Set {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    MSet {
        entries: Vec<(Vec<u8>, Vec<u8>)>,
    },
    SetNx {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Get {
        key: Vec<u8>,
    },
    MGet {
        keys: Vec<Vec<u8>>,
    },
    GetSet {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    GetDel {
        key: Vec<u8>,
    },
    Append {
        key: Vec<u8>,
        append_value: Vec<u8>,
    },
    Increment {
        key: Vec<u8>,
    },
    IncrementBy {
        key: Vec<u8>,
        amount: i64,
    },
    Decrement {
        key: Vec<u8>,
    },
    DecrementBy {
        key: Vec<u8>,
        amount: i64,
    },
    IncrementByFloat {
        key: Vec<u8>,
        amount: f64,
    },
    Exists {
        keys: Vec<Vec<u8>>,
    },
    Delete {
        keys: Vec<Vec<u8>>,
    },
    Rename {
        old_key: Vec<u8>,
        new_key: Vec<u8>,
    },
    Expire {
        key: Vec<u8>,
        seconds: u64,
    },
    PExpire {
        key: Vec<u8>,
        milliseconds: u64,
    },
    Ttl {
        key: Vec<u8>,
    },
    PTtl {
        key: Vec<u8>,
    },
    Persist {
        key: Vec<u8>,
    },
    StrLen {
        key: Vec<u8>,
    },
    GetRange {
        key: Vec<u8>,
        start: i64,
        end: i64,
    },
    SetRange {
        key: Vec<u8>,
        offset: usize,
        value: Vec<u8>,
    },
    LPush {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    RPush {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    LLen {
        key: Vec<u8>,
    },
    LPop {
        key: Vec<u8>,
    },
    RPop {
        key: Vec<u8>,
    },
    LRange {
        key: Vec<u8>,
        start: i64,
        end: i64,
    },
    SAdd {
        key: Vec<u8>,
        member: Vec<u8>,
    },
    SRem {
        key: Vec<u8>,
        member: Vec<u8>,
    },
    SIsMember {
        key: Vec<u8>,
        member: Vec<u8>,
    },
    SMembers {
        key: Vec<u8>,
    },
    SCard {
        key: Vec<u8>,
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
