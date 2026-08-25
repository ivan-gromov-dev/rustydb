use std::fmt;

#[derive(Clone, Debug, PartialEq)]
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
    Save,
    AofRewrite,
    Info,
    Help,
    Exit,
}

impl Command {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Set { .. } => "SET",
            Self::MSet { .. } => "MSET",
            Self::SetNx { .. } => "SETNX",
            Self::Get { .. } => "GET",
            Self::MGet { .. } => "MGET",
            Self::GetSet { .. } => "GETSET",
            Self::GetDel { .. } => "GETDEL",
            Self::Append { .. } => "APPEND",
            Self::Increment { .. } => "INCR",
            Self::IncrementBy { .. } => "INCRBY",
            Self::Decrement { .. } => "DECR",
            Self::DecrementBy { .. } => "DECRBY",
            Self::IncrementByFloat { .. } => "INCRBYFLOAT",
            Self::Exists { .. } => "EXISTS",
            Self::Delete { .. } => "DEL",
            Self::Rename { .. } => "RENAME",
            Self::Expire { .. } => "EXPIRE",
            Self::PExpire { .. } => "PEXPIRE",
            Self::Ttl { .. } => "TTL",
            Self::PTtl { .. } => "PTTL",
            Self::Persist { .. } => "PERSIST",
            Self::StrLen { .. } => "STRLEN",
            Self::GetRange { .. } => "GETRANGE",
            Self::SetRange { .. } => "SETRANGE",
            Self::LPush { .. } => "LPUSH",
            Self::RPush { .. } => "RPUSH",
            Self::LLen { .. } => "LLEN",
            Self::LPop { .. } => "LPOP",
            Self::RPop { .. } => "RPOP",
            Self::LRange { .. } => "LRANGE",
            Self::SAdd { .. } => "SADD",
            Self::SRem { .. } => "SREM",
            Self::SIsMember { .. } => "SISMEMBER",
            Self::SMembers { .. } => "SMEMBERS",
            Self::SCard { .. } => "SCARD",
            Self::Keys => "KEYS",
            Self::Len => "LEN",
            Self::Clear => "CLEAR",
            Self::Save => "SAVE",
            Self::AofRewrite => "AOFREWRITE",
            Self::Info => "INFO",
            Self::Help => "HELP",
            Self::Exit => "EXIT",
        }
    }

    pub(crate) fn lookup_size(&self) -> Option<usize> {
        match self {
            Self::Get { .. } => Some(1),
            Self::MGet { keys } | Self::Exists { keys } => Some(keys.len()),
            _ => None,
        }
    }

    pub(crate) fn aof_arguments(&self) -> Option<Vec<Vec<u8>>> {
        let number = |value: i64| value.to_string().into_bytes();
        let unsigned = |value: u64| value.to_string().into_bytes();
        let offset = |value: usize| value.to_string().into_bytes();
        let mut arguments = match self {
            Self::Set { key, value } => vec![b"SET".to_vec(), key.clone(), value.clone()],
            Self::MSet { entries } => {
                let mut values = vec![b"MSET".to_vec()];
                for (key, value) in entries {
                    values.push(key.clone());
                    values.push(value.clone());
                }
                values
            }
            Self::SetNx { key, value } => vec![b"SETNX".to_vec(), key.clone(), value.clone()],
            Self::GetSet { key, value } => vec![b"GETSET".to_vec(), key.clone(), value.clone()],
            Self::GetDel { key } => vec![b"GETDEL".to_vec(), key.clone()],
            Self::Append { key, append_value } => {
                vec![b"APPEND".to_vec(), key.clone(), append_value.clone()]
            }
            Self::Increment { key } => vec![b"INCR".to_vec(), key.clone()],
            Self::IncrementBy { key, amount } => {
                vec![b"INCRBY".to_vec(), key.clone(), number(*amount)]
            }
            Self::Decrement { key } => vec![b"DECR".to_vec(), key.clone()],
            Self::DecrementBy { key, amount } => {
                vec![b"DECRBY".to_vec(), key.clone(), number(*amount)]
            }
            Self::IncrementByFloat { key, amount } => vec![
                b"INCRBYFLOAT".to_vec(),
                key.clone(),
                amount.to_string().into_bytes(),
            ],
            Self::Delete { keys } => with_keys(b"DEL", keys),
            Self::Rename { old_key, new_key } => {
                vec![b"RENAME".to_vec(), old_key.clone(), new_key.clone()]
            }
            Self::Expire { key, seconds } => {
                vec![b"EXPIRE".to_vec(), key.clone(), unsigned(*seconds)]
            }
            Self::PExpire { key, milliseconds } => {
                vec![b"PEXPIRE".to_vec(), key.clone(), unsigned(*milliseconds)]
            }
            Self::Persist { key } => vec![b"PERSIST".to_vec(), key.clone()],
            Self::SetRange {
                key,
                offset: at,
                value,
            } => vec![
                b"SETRANGE".to_vec(),
                key.clone(),
                offset(*at),
                value.clone(),
            ],
            Self::LPush { key, value } => vec![b"LPUSH".to_vec(), key.clone(), value.clone()],
            Self::RPush { key, value } => vec![b"RPUSH".to_vec(), key.clone(), value.clone()],
            Self::LPop { key } => vec![b"LPOP".to_vec(), key.clone()],
            Self::RPop { key } => vec![b"RPOP".to_vec(), key.clone()],
            Self::SAdd { key, member } => vec![b"SADD".to_vec(), key.clone(), member.clone()],
            Self::SRem { key, member } => vec![b"SREM".to_vec(), key.clone(), member.clone()],
            Self::Clear => vec![b"CLEAR".to_vec()],
            Self::Get { .. }
            | Self::MGet { .. }
            | Self::Exists { .. }
            | Self::Ttl { .. }
            | Self::PTtl { .. }
            | Self::StrLen { .. }
            | Self::GetRange { .. }
            | Self::LLen { .. }
            | Self::LRange { .. }
            | Self::SIsMember { .. }
            | Self::SMembers { .. }
            | Self::SCard { .. }
            | Self::Keys
            | Self::Len
            | Self::Save
            | Self::AofRewrite
            | Self::Info
            | Self::Help
            | Self::Exit => return None,
        };
        arguments.shrink_to_fit();
        Some(arguments)
    }
}

fn with_keys(name: &[u8], keys: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut arguments = Vec::with_capacity(keys.len() + 1);
    arguments.push(name.to_vec());
    arguments.extend(keys.iter().cloned());
    arguments
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
