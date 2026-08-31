use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtocolVersion {
    Resp2,
    Resp3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetCondition {
    IfAbsent,
    IfPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetExpiration {
    Seconds(u64),
    Milliseconds(u64),
    UnixSeconds(u64),
    UnixMilliseconds(u64),
    KeepTtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GetExExpiration {
    Seconds(u64),
    Milliseconds(u64),
    UnixSeconds(u64),
    UnixMilliseconds(u64),
    Persist,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpireCondition {
    NoExpiration,
    HasExpiration,
    Greater,
    Less,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClientInfoAttribute {
    LibraryName,
    LibraryVersion,
}

impl ProtocolVersion {
    pub(crate) fn number(self) -> u8 {
        match self {
            Self::Resp2 => 2,
            Self::Resp3 => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Command {
    Set {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    SetAdvanced {
        key: Vec<u8>,
        value: Vec<u8>,
        condition: Option<SetCondition>,
        return_old: bool,
        expiration: Option<SetExpiration>,
    },
    MSet {
        entries: Vec<(Vec<u8>, Vec<u8>)>,
    },
    MSetNx {
        entries: Vec<(Vec<u8>, Vec<u8>)>,
    },
    SetNx {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Get {
        key: Vec<u8>,
    },
    GetEx {
        key: Vec<u8>,
        expiration: Option<GetExExpiration>,
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
    Type {
        key: Vec<u8>,
    },
    Touch {
        keys: Vec<Vec<u8>>,
    },
    Unlink {
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
    ExpireAdvanced {
        key: Vec<u8>,
        seconds: u64,
        condition: ExpireCondition,
    },
    PExpireAdvanced {
        key: Vec<u8>,
        milliseconds: u64,
        condition: ExpireCondition,
    },
    PExpire {
        key: Vec<u8>,
        milliseconds: u64,
    },
    ExpireAt {
        key: Vec<u8>,
        unix_seconds: u64,
        condition: Option<ExpireCondition>,
    },
    PExpireAt {
        key: Vec<u8>,
        unix_milliseconds: u64,
        condition: Option<ExpireCondition>,
    },
    Ttl {
        key: Vec<u8>,
    },
    PTtl {
        key: Vec<u8>,
    },
    ExpireTime {
        key: Vec<u8>,
    },
    PExpireTime {
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
    LPushMany {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
    },
    LPushX {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
    },
    RPush {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    RPushMany {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
    },
    RPushX {
        key: Vec<u8>,
        values: Vec<Vec<u8>>,
    },
    LLen {
        key: Vec<u8>,
    },
    LPop {
        key: Vec<u8>,
    },
    LPopCount {
        key: Vec<u8>,
        count: usize,
    },
    RPop {
        key: Vec<u8>,
    },
    RPopCount {
        key: Vec<u8>,
        count: usize,
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
    SAddMany {
        key: Vec<u8>,
        members: Vec<Vec<u8>>,
    },
    SRem {
        key: Vec<u8>,
        member: Vec<u8>,
    },
    SRemMany {
        key: Vec<u8>,
        members: Vec<Vec<u8>>,
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
    HSet {
        key: Vec<u8>,
        entries: Vec<(Vec<u8>, Vec<u8>)>,
    },
    HSetNx {
        key: Vec<u8>,
        field: Vec<u8>,
        value: Vec<u8>,
    },
    HGet {
        key: Vec<u8>,
        field: Vec<u8>,
    },
    HMGet {
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
    },
    HGetAll {
        key: Vec<u8>,
    },
    HDel {
        key: Vec<u8>,
        fields: Vec<Vec<u8>>,
    },
    HExists {
        key: Vec<u8>,
        field: Vec<u8>,
    },
    HLen {
        key: Vec<u8>,
    },
    HKeys {
        key: Vec<u8>,
    },
    HVals {
        key: Vec<u8>,
    },
    HIncrementBy {
        key: Vec<u8>,
        field: Vec<u8>,
        amount: i64,
    },
    HIncrementByFloat {
        key: Vec<u8>,
        field: Vec<u8>,
        amount: f64,
    },
    HScan {
        key: Vec<u8>,
        cursor: usize,
        pattern: Option<Vec<u8>>,
        count: usize,
    },
    Ping {
        message: Option<Vec<u8>>,
    },
    Echo {
        message: Vec<u8>,
    },
    Hello {
        protocol: Option<ProtocolVersion>,
    },
    ClientId,
    ClientSetName {
        name: Vec<u8>,
    },
    ClientGetName,
    ClientSetInfo {
        attribute: ClientInfoAttribute,
        value: Vec<u8>,
    },
    MetadataList,
    MetadataInfo {
        names: Vec<Vec<u8>>,
    },
    MetadataCount,
    Select,
    DbSize,
    FlushDb,
    FlushAll,
    Keys {
        pattern: Vec<u8>,
    },
    Scan {
        cursor: usize,
        pattern: Option<Vec<u8>>,
        count: usize,
        type_name: Option<Vec<u8>>,
    },
    RandomKey,
    Copy {
        source: Vec<u8>,
        destination: Vec<u8>,
        replace: bool,
    },
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
            Self::Set { .. } | Self::SetAdvanced { .. } => "SET",
            Self::MSet { .. } => "MSET",
            Self::MSetNx { .. } => "MSETNX",
            Self::SetNx { .. } => "SETNX",
            Self::Get { .. } => "GET",
            Self::GetEx { .. } => "GETEX",
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
            Self::Type { .. } => "TYPE",
            Self::Touch { .. } => "TOUCH",
            Self::Unlink { .. } => "UNLINK",
            Self::Rename { .. } => "RENAME",
            Self::Expire { .. } => "EXPIRE",
            Self::ExpireAdvanced { .. } => "EXPIRE",
            Self::PExpire { .. } => "PEXPIRE",
            Self::PExpireAdvanced { .. } => "PEXPIRE",
            Self::ExpireAt { .. } => "EXPIREAT",
            Self::PExpireAt { .. } => "PEXPIREAT",
            Self::Ttl { .. } => "TTL",
            Self::PTtl { .. } => "PTTL",
            Self::ExpireTime { .. } => "EXPIRETIME",
            Self::PExpireTime { .. } => "PEXPIRETIME",
            Self::Persist { .. } => "PERSIST",
            Self::StrLen { .. } => "STRLEN",
            Self::GetRange { .. } => "GETRANGE",
            Self::SetRange { .. } => "SETRANGE",
            Self::LPush { .. } | Self::LPushMany { .. } => "LPUSH",
            Self::LPushX { .. } => "LPUSHX",
            Self::RPush { .. } | Self::RPushMany { .. } => "RPUSH",
            Self::RPushX { .. } => "RPUSHX",
            Self::LLen { .. } => "LLEN",
            Self::LPop { .. } | Self::LPopCount { .. } => "LPOP",
            Self::RPop { .. } | Self::RPopCount { .. } => "RPOP",
            Self::LRange { .. } => "LRANGE",
            Self::SAdd { .. } | Self::SAddMany { .. } => "SADD",
            Self::SRem { .. } | Self::SRemMany { .. } => "SREM",
            Self::SIsMember { .. } => "SISMEMBER",
            Self::SMembers { .. } => "SMEMBERS",
            Self::SCard { .. } => "SCARD",
            Self::HSet { .. } => "HSET",
            Self::HSetNx { .. } => "HSETNX",
            Self::HGet { .. } => "HGET",
            Self::HMGet { .. } => "HMGET",
            Self::HGetAll { .. } => "HGETALL",
            Self::HDel { .. } => "HDEL",
            Self::HExists { .. } => "HEXISTS",
            Self::HLen { .. } => "HLEN",
            Self::HKeys { .. } => "HKEYS",
            Self::HVals { .. } => "HVALS",
            Self::HIncrementBy { .. } => "HINCRBY",
            Self::HIncrementByFloat { .. } => "HINCRBYFLOAT",
            Self::HScan { .. } => "HSCAN",
            Self::Ping { .. } => "PING",
            Self::Echo { .. } => "ECHO",
            Self::Hello { .. } => "HELLO",
            Self::ClientId
            | Self::ClientSetName { .. }
            | Self::ClientGetName
            | Self::ClientSetInfo { .. } => "CLIENT",
            Self::MetadataList | Self::MetadataInfo { .. } | Self::MetadataCount => "COMMAND",
            Self::Select => "SELECT",
            Self::DbSize => "DBSIZE",
            Self::FlushDb => "FLUSHDB",
            Self::FlushAll => "FLUSHALL",
            Self::Keys { .. } => "KEYS",
            Self::Scan { .. } => "SCAN",
            Self::RandomKey => "RANDOMKEY",
            Self::Copy { .. } => "COPY",
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
            Self::SetAdvanced {
                key,
                value,
                expiration,
                ..
            } => {
                let mut values = vec![b"SET".to_vec(), key.clone(), value.clone()];
                match expiration {
                    Some(SetExpiration::Seconds(value)) => {
                        values.extend([b"EX".to_vec(), unsigned(*value)]);
                    }
                    Some(SetExpiration::Milliseconds(value)) => {
                        values.extend([b"PX".to_vec(), unsigned(*value)]);
                    }
                    Some(SetExpiration::UnixSeconds(value)) => {
                        values.extend([b"EXAT".to_vec(), unsigned(*value)]);
                    }
                    Some(SetExpiration::UnixMilliseconds(value)) => {
                        values.extend([b"PXAT".to_vec(), unsigned(*value)]);
                    }
                    Some(SetExpiration::KeepTtl) => values.push(b"KEEPTTL".to_vec()),
                    None => {}
                }
                values
            }
            Self::MSet { entries } => {
                let mut values = vec![b"MSET".to_vec()];
                for (key, value) in entries {
                    values.push(key.clone());
                    values.push(value.clone());
                }
                values
            }
            Self::MSetNx { entries } => {
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
            Self::GetEx { key, expiration } => {
                let mut values = vec![b"GETEX".to_vec(), key.clone()];
                match expiration {
                    Some(GetExExpiration::Seconds(value)) => {
                        values.extend([b"EX".to_vec(), unsigned(*value)]);
                    }
                    Some(GetExExpiration::Milliseconds(value)) => {
                        values.extend([b"PX".to_vec(), unsigned(*value)]);
                    }
                    Some(GetExExpiration::UnixSeconds(value)) => {
                        values.extend([b"EXAT".to_vec(), unsigned(*value)]);
                    }
                    Some(GetExExpiration::UnixMilliseconds(value)) => {
                        values.extend([b"PXAT".to_vec(), unsigned(*value)]);
                    }
                    Some(GetExExpiration::Persist) => values.push(b"PERSIST".to_vec()),
                    None => return None,
                }
                values
            }
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
            Self::Unlink { keys } => with_keys(b"DEL", keys),
            Self::Copy {
                source,
                destination,
                ..
            } => vec![
                b"COPY".to_vec(),
                source.clone(),
                destination.clone(),
                b"REPLACE".to_vec(),
            ],
            Self::Rename { old_key, new_key } => {
                vec![b"RENAME".to_vec(), old_key.clone(), new_key.clone()]
            }
            Self::Expire { key, seconds } => {
                vec![b"EXPIRE".to_vec(), key.clone(), unsigned(*seconds)]
            }
            Self::ExpireAdvanced { key, seconds, .. } => {
                vec![b"EXPIRE".to_vec(), key.clone(), unsigned(*seconds)]
            }
            Self::PExpire { key, milliseconds } => {
                vec![b"PEXPIRE".to_vec(), key.clone(), unsigned(*milliseconds)]
            }
            Self::PExpireAdvanced {
                key, milliseconds, ..
            } => vec![b"PEXPIRE".to_vec(), key.clone(), unsigned(*milliseconds)],
            Self::ExpireAt {
                key, unix_seconds, ..
            } => vec![b"EXPIREAT".to_vec(), key.clone(), unsigned(*unix_seconds)],
            Self::PExpireAt {
                key,
                unix_milliseconds,
                ..
            } => vec![
                b"PEXPIREAT".to_vec(),
                key.clone(),
                unsigned(*unix_milliseconds),
            ],
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
            Self::LPushMany { key, values } => with_values(b"LPUSH", key, values),
            Self::LPushX { key, values } => with_values(b"LPUSHX", key, values),
            Self::RPush { key, value } => vec![b"RPUSH".to_vec(), key.clone(), value.clone()],
            Self::RPushMany { key, values } => with_values(b"RPUSH", key, values),
            Self::RPushX { key, values } => with_values(b"RPUSHX", key, values),
            Self::LPop { key } => vec![b"LPOP".to_vec(), key.clone()],
            Self::LPopCount { key, count } => {
                vec![b"LPOP".to_vec(), key.clone(), offset(*count)]
            }
            Self::RPop { key } => vec![b"RPOP".to_vec(), key.clone()],
            Self::RPopCount { key, count } => {
                vec![b"RPOP".to_vec(), key.clone(), offset(*count)]
            }
            Self::SAdd { key, member } => vec![b"SADD".to_vec(), key.clone(), member.clone()],
            Self::SAddMany { key, members } => with_values(b"SADD", key, members),
            Self::SRem { key, member } => vec![b"SREM".to_vec(), key.clone(), member.clone()],
            Self::SRemMany { key, members } => with_values(b"SREM", key, members),
            Self::HSet { key, entries } => {
                let mut values = vec![b"HSET".to_vec(), key.clone()];
                for (field, value) in entries {
                    values.extend([field.clone(), value.clone()]);
                }
                values
            }
            Self::HSetNx { key, field, value } => {
                vec![
                    b"HSETNX".to_vec(),
                    key.clone(),
                    field.clone(),
                    value.clone(),
                ]
            }
            Self::HDel { key, fields } => {
                let mut values = vec![b"HDEL".to_vec(), key.clone()];
                values.extend(fields.iter().cloned());
                values
            }
            Self::HIncrementBy { key, field, amount } => vec![
                b"HINCRBY".to_vec(),
                key.clone(),
                field.clone(),
                number(*amount),
            ],
            Self::HIncrementByFloat { key, field, amount } => vec![
                b"HINCRBYFLOAT".to_vec(),
                key.clone(),
                field.clone(),
                amount.to_string().into_bytes(),
            ],
            Self::Clear => vec![b"CLEAR".to_vec()],
            Self::FlushDb => vec![b"FLUSHDB".to_vec()],
            Self::FlushAll => vec![b"FLUSHALL".to_vec()],
            Self::Get { .. }
            | Self::MGet { .. }
            | Self::Exists { .. }
            | Self::Type { .. }
            | Self::Touch { .. }
            | Self::Ttl { .. }
            | Self::PTtl { .. }
            | Self::ExpireTime { .. }
            | Self::PExpireTime { .. }
            | Self::StrLen { .. }
            | Self::GetRange { .. }
            | Self::LLen { .. }
            | Self::LRange { .. }
            | Self::SIsMember { .. }
            | Self::SMembers { .. }
            | Self::SCard { .. }
            | Self::HGet { .. }
            | Self::HMGet { .. }
            | Self::HGetAll { .. }
            | Self::HExists { .. }
            | Self::HLen { .. }
            | Self::HKeys { .. }
            | Self::HVals { .. }
            | Self::HScan { .. }
            | Self::Ping { .. }
            | Self::Echo { .. }
            | Self::Hello { .. }
            | Self::ClientId
            | Self::ClientSetName { .. }
            | Self::ClientGetName
            | Self::ClientSetInfo { .. }
            | Self::MetadataList
            | Self::MetadataInfo { .. }
            | Self::MetadataCount
            | Self::Select
            | Self::DbSize
            | Self::Keys { .. }
            | Self::Scan { .. }
            | Self::RandomKey
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

fn with_values(name: &[u8], key: &[u8], values: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut arguments = Vec::with_capacity(values.len() + 2);
    arguments.push(name.to_vec());
    arguments.push(key.to_vec());
    arguments.extend(values.iter().cloned());
    arguments
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CommandError {
    EmptyInput,
    InvalidArguments(&'static str),
    UnknownCommand(String),
    InvalidInteger(String),
    InvalidFloat(String),
    UnsupportedProtocol(i64),
    InvalidClientMetadata,
    UnsupportedDatabase(i64),
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
            Self::UnsupportedProtocol(protocol) => {
                write!(formatter, "unsupported protocol version: {protocol}")
            }
            Self::InvalidClientMetadata => write!(
                formatter,
                "client metadata cannot contain spaces, newlines or special characters"
            ),
            Self::UnsupportedDatabase(index) => {
                write!(formatter, "DB index is out of range: {index}")
            }
        }
    }
}
