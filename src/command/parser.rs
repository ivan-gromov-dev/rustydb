use super::types::{
    ClientInfoAttribute, Command, CommandError, ExpireCondition, GetExExpiration, ProtocolVersion,
    SetCondition, SetExpiration,
};

type KeyValueEntries = Vec<(Vec<u8>, Vec<u8>)>;

impl Command {
    pub(crate) fn parse(input: &str) -> Result<Self, CommandError> {
        let input = input.trim();
        let Some(command) = input.split_whitespace().next() else {
            return Err(CommandError::EmptyInput);
        };

        let command = command.to_ascii_uppercase();
        let tail_after = match command.as_str() {
            "SET" => {
                let tokens: Vec<_> = input.split_whitespace().collect();
                let has_options = tokens.get(3).is_some_and(|option| {
                    ["NX", "XX", "GET", "EX", "PX", "EXAT", "PXAT", "KEEPTTL"]
                        .iter()
                        .any(|candidate| option.eq_ignore_ascii_case(candidate))
                });
                if has_options { None } else { Some(2) }
            }
            "SETNX" | "GETSET" | "APPEND" | "LPUSH" | "RPUSH" | "SADD" | "SREM" | "SISMEMBER" => {
                Some(2)
            }
            "PING" | "ECHO" => Some(1),
            "SETRANGE" => Some(3),
            _ => None,
        };
        let args = match tail_after {
            Some(head_len) => split_with_tail(input, head_len),
            None => input.split_whitespace().collect(),
        };

        Self::from_args(&args)
    }

    pub(crate) fn from_args(args: &[&str]) -> Result<Self, CommandError> {
        let args: Vec<&[u8]> = args.iter().map(|argument| argument.as_bytes()).collect();
        Self::from_bytes(&args)
    }

    pub(crate) fn from_owned_bytes(args: Vec<Vec<u8>>) -> Result<Self, CommandError> {
        let Some(command) = args.first() else {
            return Err(CommandError::EmptyInput);
        };

        if command.eq_ignore_ascii_case(b"GET") {
            exact_owned(&args, 2, "GET key")?;
            let mut args = args;
            return Ok(Self::Get {
                key: args.remove(1),
            });
        }
        if command.eq_ignore_ascii_case(b"SET") {
            let borrowed: Vec<_> = args.iter().map(Vec::as_slice).collect();
            return parse_set(&borrowed);
        }

        let borrowed: Vec<_> = args.iter().map(Vec::as_slice).collect();
        Self::from_bytes(&borrowed)
    }

    pub(crate) fn from_bytes(args: &[&[u8]]) -> Result<Self, CommandError> {
        let Some(command) = args.first() else {
            return Err(CommandError::EmptyInput);
        };

        // GET and SET dominate the measured workloads. Match them directly so
        // their names do not need an allocated uppercase copy.
        if command.eq_ignore_ascii_case(b"GET") {
            return Ok(Self::Get {
                key: one(args, "GET key")?,
            });
        }
        if command.eq_ignore_ascii_case(b"SET") {
            return parse_set(args);
        }

        let command = String::from_utf8_lossy(command).to_ascii_uppercase();

        match command.as_str() {
            "MSET" => parse_mset(args),
            "MSETNX" => parse_msetnx(args),
            "SETNX" => {
                exact(args, 3, "SETNX key value")?;
                Ok(Self::SetNx {
                    key: owned(args[1]),
                    value: owned(args[2]),
                })
            }
            "MGET" => Ok(Self::MGet {
                keys: many(args, "MGET key [key ...]")?,
            }),
            "GETEX" => parse_getex(args),
            "GETSET" => {
                exact(args, 3, "GETSET key value")?;
                Ok(Self::GetSet {
                    key: owned(args[1]),
                    value: owned(args[2]),
                })
            }
            "GETDEL" => Ok(Self::GetDel {
                key: one(args, "GETDEL key")?,
            }),
            "APPEND" => {
                exact(args, 3, "APPEND key value")?;
                Ok(Self::Append {
                    key: owned(args[1]),
                    append_value: owned(args[2]),
                })
            }
            "INCR" => Ok(Self::Increment {
                key: one(args, "INCR key")?,
            }),
            "INCRBY" => {
                let (key, amount) = key_i64(args, "INCRBY key inc_value")?;
                Ok(Self::IncrementBy { key, amount })
            }
            "DECR" => Ok(Self::Decrement {
                key: one(args, "DECR key")?,
            }),
            "DECRBY" => {
                let (key, amount) = key_i64(args, "DECRBY key decr_value")?;
                Ok(Self::DecrementBy { key, amount })
            }
            "INCRBYFLOAT" => parse_increment_by_float(args),
            "EXISTS" => Ok(Self::Exists {
                keys: many(args, "EXISTS key [key ...]")?,
            }),
            "DEL" => Ok(Self::Delete {
                keys: many(args, "DEL key [key ...]")?,
            }),
            "TYPE" => Ok(Self::Type {
                key: one(args, "TYPE key")?,
            }),
            "TOUCH" => Ok(Self::Touch {
                keys: many(args, "TOUCH key [key ...]")?,
            }),
            "UNLINK" => Ok(Self::Unlink {
                keys: many(args, "UNLINK key [key ...]")?,
            }),
            "RENAME" => {
                exact(args, 3, "RENAME old_key new_key")?;
                Ok(Self::Rename {
                    old_key: owned(args[1]),
                    new_key: owned(args[2]),
                })
            }
            "EXPIRE" => parse_expire(args, false),
            "PEXPIRE" => parse_expire(args, true),
            "EXPIREAT" => parse_expire_at(args, false),
            "PEXPIREAT" => parse_expire_at(args, true),
            "TTL" => Ok(Self::Ttl {
                key: one(args, "TTL key")?,
            }),
            "PTTL" => Ok(Self::PTtl {
                key: one(args, "PTTL key")?,
            }),
            "EXPIRETIME" => Ok(Self::ExpireTime {
                key: one(args, "EXPIRETIME key")?,
            }),
            "PEXPIRETIME" => Ok(Self::PExpireTime {
                key: one(args, "PEXPIRETIME key")?,
            }),
            "PERSIST" => Ok(Self::Persist {
                key: one(args, "PERSIST key")?,
            }),
            "STRLEN" => Ok(Self::StrLen {
                key: one(args, "STRLEN key")?,
            }),
            "GETRANGE" => {
                let (key, start, end) = range_args(args, "GETRANGE key start end")?;
                Ok(Self::GetRange { key, start, end })
            }
            "SETRANGE" => {
                exact(args, 4, "SETRANGE key offset value")?;
                let offset = parse_usize(args[2])?;
                Ok(Self::SetRange {
                    key: owned(args[1]),
                    offset,
                    value: owned(args[3]),
                })
            }
            "LPUSH" => {
                exact(args, 3, "LPUSH key value")?;
                Ok(Self::LPush {
                    key: owned(args[1]),
                    value: owned(args[2]),
                })
            }
            "RPUSH" => {
                exact(args, 3, "RPUSH key value")?;
                Ok(Self::RPush {
                    key: owned(args[1]),
                    value: owned(args[2]),
                })
            }
            "LLEN" => Ok(Self::LLen {
                key: one(args, "LLEN key")?,
            }),
            "LPOP" => Ok(Self::LPop {
                key: one(args, "LPOP key")?,
            }),
            "RPOP" => Ok(Self::RPop {
                key: one(args, "RPOP key")?,
            }),
            "LRANGE" => {
                let (key, start, end) = range_args(args, "LRANGE key start end")?;
                Ok(Self::LRange { key, start, end })
            }
            "SADD" => {
                exact(args, 3, "SADD key member")?;
                Ok(Self::SAdd {
                    key: owned(args[1]),
                    member: owned(args[2]),
                })
            }
            "SREM" => {
                exact(args, 3, "SREM key member")?;
                Ok(Self::SRem {
                    key: owned(args[1]),
                    member: owned(args[2]),
                })
            }
            "SISMEMBER" => {
                exact(args, 3, "SISMEMBER key member")?;
                Ok(Self::SIsMember {
                    key: owned(args[1]),
                    member: owned(args[2]),
                })
            }
            "SMEMBERS" => Ok(Self::SMembers {
                key: one(args, "SMEMBERS key")?,
            }),
            "SCARD" => Ok(Self::SCard {
                key: one(args, "SCARD key")?,
            }),
            "PING" => {
                if args.len() > 2 {
                    return Err(CommandError::InvalidArguments("PING [message]"));
                }
                Ok(Self::Ping {
                    message: args.get(1).map(|message| owned(message)),
                })
            }
            "ECHO" => Ok(Self::Echo {
                message: one(args, "ECHO message")?,
            }),
            "HELLO" => parse_hello(args),
            "CLIENT" => parse_client(args),
            "COMMAND" => parse_command_metadata(args),
            "SELECT" => parse_select(args),
            "DBSIZE" => no_args(args, "DBSIZE", Self::DbSize),
            "FLUSHDB" => parse_flush(args, "FLUSHDB [SYNC|ASYNC]", Self::FlushDb),
            "FLUSHALL" => parse_flush(args, "FLUSHALL [SYNC|ASYNC]", Self::FlushAll),
            "KEYS" => Ok(Self::Keys {
                pattern: one(args, "KEYS pattern")?,
            }),
            "SCAN" => parse_scan(args),
            "RANDOMKEY" => no_args(args, "RANDOMKEY", Self::RandomKey),
            "COPY" => parse_copy(args),
            "LEN" => no_args(args, "LEN", Self::Len),
            "CLEAR" => no_args(args, "CLEAR", Self::Clear),
            "SAVE" => no_args(args, "SAVE", Self::Save),
            "AOFREWRITE" => no_args(args, "AOFREWRITE", Self::AofRewrite),
            "INFO" => no_args(args, "INFO", Self::Info),
            "HELP" => no_args(args, "HELP", Self::Help),
            "EXIT" | "QUIT" => no_args(args, "EXIT", Self::Exit),
            _ => Err(CommandError::UnknownCommand(command)),
        }
    }
}

fn split_with_tail(input: &str, head_len: usize) -> Vec<&str> {
    let mut args = Vec::with_capacity(head_len + 1);
    let mut remaining = input;

    for _ in 0..head_len {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            return args;
        }

        let end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        args.push(&remaining[..end]);
        remaining = &remaining[end..];
    }

    remaining = remaining.trim_start();
    if !remaining.is_empty() {
        args.push(remaining);
    }

    args
}

fn exact(args: &[&[u8]], length: usize, usage: &'static str) -> Result<(), CommandError> {
    if args.len() == length {
        Ok(())
    } else {
        Err(CommandError::InvalidArguments(usage))
    }
}

fn exact_owned(args: &[Vec<u8>], length: usize, usage: &'static str) -> Result<(), CommandError> {
    if args.len() == length {
        Ok(())
    } else {
        Err(CommandError::InvalidArguments(usage))
    }
}

fn no_args(args: &[&[u8]], usage: &'static str, command: Command) -> Result<Command, CommandError> {
    exact(args, 1, usage)?;
    Ok(command)
}

fn one(args: &[&[u8]], usage: &'static str) -> Result<Vec<u8>, CommandError> {
    exact(args, 2, usage)?;
    Ok(owned(args[1]))
}

fn many(args: &[&[u8]], usage: &'static str) -> Result<Vec<Vec<u8>>, CommandError> {
    if args.len() < 2 {
        return Err(CommandError::InvalidArguments(usage));
    }
    Ok(args[1..].iter().map(|value| owned(value)).collect())
}

fn parse_mset(args: &[&[u8]]) -> Result<Command, CommandError> {
    let usage = "MSET key value [key value ...]";
    if args.len() < 3 || args.len() % 2 == 0 {
        return Err(CommandError::InvalidArguments(usage));
    }

    let entries = args[1..]
        .chunks_exact(2)
        .map(|entry| (owned(entry[0]), owned(entry[1])))
        .collect();
    Ok(Command::MSet { entries })
}

fn parse_scan(args: &[&[u8]]) -> Result<Command, CommandError> {
    const USAGE: &str = "SCAN cursor [MATCH pattern] [COUNT count] [TYPE type]";
    if args.len() < 2 {
        return Err(CommandError::InvalidArguments(USAGE));
    }
    let cursor = parse_usize(args[1])?;
    let mut pattern = None;
    let mut count = None;
    let mut type_name = None;
    let mut index = 2;
    while index < args.len() {
        let option = args[index];
        let Some(value) = args.get(index + 1) else {
            return Err(CommandError::InvalidArguments(USAGE));
        };
        if option.eq_ignore_ascii_case(b"MATCH") && pattern.is_none() {
            pattern = Some(owned(value));
        } else if option.eq_ignore_ascii_case(b"COUNT") && count.is_none() {
            let parsed = parse_usize(value)?;
            if parsed == 0 {
                return Err(CommandError::InvalidArguments(USAGE));
            }
            count = Some(parsed);
        } else if option.eq_ignore_ascii_case(b"TYPE") && type_name.is_none() {
            type_name = Some(owned(value));
        } else {
            return Err(CommandError::InvalidArguments(USAGE));
        }
        index += 2;
    }
    Ok(Command::Scan {
        cursor,
        pattern,
        count: count.unwrap_or(10),
        type_name,
    })
}

fn parse_copy(args: &[&[u8]]) -> Result<Command, CommandError> {
    const USAGE: &str = "COPY source destination [DB destination-db] [REPLACE]";
    if args.len() < 3 {
        return Err(CommandError::InvalidArguments(USAGE));
    }
    let mut replace = false;
    let mut database = None;
    let mut index = 3;
    while index < args.len() {
        if args[index].eq_ignore_ascii_case(b"REPLACE") && !replace {
            replace = true;
            index += 1;
        } else if args[index].eq_ignore_ascii_case(b"DB") && database.is_none() {
            let Some(value) = args.get(index + 1) else {
                return Err(CommandError::InvalidArguments(USAGE));
            };
            database = Some(parse_i64(value)?);
            index += 2;
        } else {
            return Err(CommandError::InvalidArguments(USAGE));
        }
    }
    if let Some(index) = database
        && index != 0
    {
        return Err(CommandError::UnsupportedDatabase(index));
    }
    Ok(Command::Copy {
        source: owned(args[1]),
        destination: owned(args[2]),
        replace,
    })
}

fn parse_msetnx(args: &[&[u8]]) -> Result<Command, CommandError> {
    let entries = parse_key_value_pairs(args, "MSETNX key value [key value ...]")?;
    Ok(Command::MSetNx { entries })
}

fn parse_key_value_pairs(
    args: &[&[u8]],
    usage: &'static str,
) -> Result<KeyValueEntries, CommandError> {
    if args.len() < 3 || args.len() % 2 == 0 {
        return Err(CommandError::InvalidArguments(usage));
    }
    Ok(args[1..]
        .chunks_exact(2)
        .map(|entry| (owned(entry[0]), owned(entry[1])))
        .collect())
}

fn parse_getex(args: &[&[u8]]) -> Result<Command, CommandError> {
    const USAGE: &str =
        "GETEX key [EX seconds|PX milliseconds|EXAT unix-seconds|PXAT unix-milliseconds|PERSIST]";
    if args.len() == 2 {
        return Ok(Command::GetEx {
            key: owned(args[1]),
            expiration: None,
        });
    }
    if args.len() == 3 && args[2].eq_ignore_ascii_case(b"PERSIST") {
        return Ok(Command::GetEx {
            key: owned(args[1]),
            expiration: Some(GetExExpiration::Persist),
        });
    }
    if args.len() != 4 {
        return Err(CommandError::InvalidArguments(USAGE));
    }
    let value = parse_u64(args[3])?;
    if value == 0 {
        return Err(CommandError::InvalidArguments(USAGE));
    }
    let expiration = if args[2].eq_ignore_ascii_case(b"EX") {
        GetExExpiration::Seconds(value)
    } else if args[2].eq_ignore_ascii_case(b"PX") {
        GetExExpiration::Milliseconds(value)
    } else if args[2].eq_ignore_ascii_case(b"EXAT") {
        GetExExpiration::UnixSeconds(value)
    } else if args[2].eq_ignore_ascii_case(b"PXAT") {
        GetExExpiration::UnixMilliseconds(value)
    } else {
        return Err(CommandError::InvalidArguments(USAGE));
    };
    Ok(Command::GetEx {
        key: owned(args[1]),
        expiration: Some(expiration),
    })
}

fn parse_expire(args: &[&[u8]], milliseconds: bool) -> Result<Command, CommandError> {
    let usage = if milliseconds {
        "PEXPIRE key milliseconds"
    } else {
        "EXPIRE key seconds"
    };
    if args.len() != 3 && args.len() != 4 {
        return Err(CommandError::InvalidArguments(usage));
    }
    let value = parse_u64(args[2])?;
    let key = owned(args[1]);
    let Some(option) = args.get(3) else {
        return Ok(if milliseconds {
            Command::PExpire {
                key,
                milliseconds: value,
            }
        } else {
            Command::Expire {
                key,
                seconds: value,
            }
        });
    };
    let condition = parse_expire_condition(option, usage)?;
    Ok(if milliseconds {
        Command::PExpireAdvanced {
            key,
            milliseconds: value,
            condition,
        }
    } else {
        Command::ExpireAdvanced {
            key,
            seconds: value,
            condition,
        }
    })
}

fn parse_expire_at(args: &[&[u8]], milliseconds: bool) -> Result<Command, CommandError> {
    let usage = if milliseconds {
        "PEXPIREAT key unix-milliseconds [NX|XX|GT|LT]"
    } else {
        "EXPIREAT key unix-seconds [NX|XX|GT|LT]"
    };
    if args.len() != 3 && args.len() != 4 {
        return Err(CommandError::InvalidArguments(usage));
    }
    let value = parse_u64(args[2])?;
    let condition = args
        .get(3)
        .map(|option| parse_expire_condition(option, usage))
        .transpose()?;
    Ok(if milliseconds {
        Command::PExpireAt {
            key: owned(args[1]),
            unix_milliseconds: value,
            condition,
        }
    } else {
        Command::ExpireAt {
            key: owned(args[1]),
            unix_seconds: value,
            condition,
        }
    })
}

fn parse_expire_condition(
    option: &[u8],
    usage: &'static str,
) -> Result<ExpireCondition, CommandError> {
    if option.eq_ignore_ascii_case(b"NX") {
        Ok(ExpireCondition::NoExpiration)
    } else if option.eq_ignore_ascii_case(b"XX") {
        Ok(ExpireCondition::HasExpiration)
    } else if option.eq_ignore_ascii_case(b"GT") {
        Ok(ExpireCondition::Greater)
    } else if option.eq_ignore_ascii_case(b"LT") {
        Ok(ExpireCondition::Less)
    } else {
        Err(CommandError::InvalidArguments(usage))
    }
}

fn parse_set(args: &[&[u8]]) -> Result<Command, CommandError> {
    const USAGE: &str = "SET key value";
    if args.len() < 3 {
        return Err(CommandError::InvalidArguments(USAGE));
    }
    if args.len() == 3 {
        return Ok(Command::Set {
            key: owned(args[1]),
            value: owned(args[2]),
        });
    }

    let mut condition = None;
    let mut return_old = false;
    let mut expiration = None;
    let mut index = 3;
    while index < args.len() {
        let option = args[index];
        if option.eq_ignore_ascii_case(b"NX") || option.eq_ignore_ascii_case(b"XX") {
            if condition.is_some() {
                return Err(CommandError::InvalidArguments(USAGE));
            }
            condition = Some(if option.eq_ignore_ascii_case(b"NX") {
                SetCondition::IfAbsent
            } else {
                SetCondition::IfPresent
            });
            index += 1;
        } else if option.eq_ignore_ascii_case(b"GET") {
            if return_old {
                return Err(CommandError::InvalidArguments(USAGE));
            }
            return_old = true;
            index += 1;
        } else if option.eq_ignore_ascii_case(b"KEEPTTL") {
            if expiration.is_some() {
                return Err(CommandError::InvalidArguments(USAGE));
            }
            expiration = Some(SetExpiration::KeepTtl);
            index += 1;
        } else if option.eq_ignore_ascii_case(b"EX")
            || option.eq_ignore_ascii_case(b"PX")
            || option.eq_ignore_ascii_case(b"EXAT")
            || option.eq_ignore_ascii_case(b"PXAT")
        {
            if expiration.is_some() || index + 1 >= args.len() {
                return Err(CommandError::InvalidArguments(USAGE));
            }
            let value = parse_u64(args[index + 1])?;
            if value == 0 {
                return Err(CommandError::InvalidArguments(USAGE));
            }
            expiration = Some(if option.eq_ignore_ascii_case(b"EX") {
                SetExpiration::Seconds(value)
            } else if option.eq_ignore_ascii_case(b"PX") {
                SetExpiration::Milliseconds(value)
            } else if option.eq_ignore_ascii_case(b"EXAT") {
                SetExpiration::UnixSeconds(value)
            } else {
                SetExpiration::UnixMilliseconds(value)
            });
            index += 2;
        } else {
            return Err(CommandError::InvalidArguments(USAGE));
        }
    }
    Ok(Command::SetAdvanced {
        key: owned(args[1]),
        value: owned(args[2]),
        condition,
        return_old,
        expiration,
    })
}

fn parse_hello(args: &[&[u8]]) -> Result<Command, CommandError> {
    if args.len() > 2 {
        return Err(CommandError::InvalidArguments("HELLO [2|3]"));
    }
    let Some(protocol) = args.get(1) else {
        return Ok(Command::Hello { protocol: None });
    };
    let protocol = parse_i64(protocol)?;
    let protocol = match protocol {
        2 => ProtocolVersion::Resp2,
        3 => ProtocolVersion::Resp3,
        unsupported => return Err(CommandError::UnsupportedProtocol(unsupported)),
    };
    Ok(Command::Hello {
        protocol: Some(protocol),
    })
}

fn parse_client(args: &[&[u8]]) -> Result<Command, CommandError> {
    let Some(subcommand) = args.get(1) else {
        return Err(CommandError::InvalidArguments(
            "CLIENT ID|GETNAME|SETNAME name|SETINFO LIB-NAME|LIB-VER value",
        ));
    };

    if subcommand.eq_ignore_ascii_case(b"ID") {
        exact(args, 2, "CLIENT ID")?;
        return Ok(Command::ClientId);
    }
    if subcommand.eq_ignore_ascii_case(b"GETNAME") {
        exact(args, 2, "CLIENT GETNAME")?;
        return Ok(Command::ClientGetName);
    }
    if subcommand.eq_ignore_ascii_case(b"SETNAME") {
        exact(args, 3, "CLIENT SETNAME name")?;
        validate_client_metadata(args[2])?;
        return Ok(Command::ClientSetName {
            name: owned(args[2]),
        });
    }
    if subcommand.eq_ignore_ascii_case(b"SETINFO") {
        exact(args, 4, "CLIENT SETINFO LIB-NAME|LIB-VER value")?;
        validate_client_metadata(args[3])?;
        let attribute = if args[2].eq_ignore_ascii_case(b"LIB-NAME") {
            ClientInfoAttribute::LibraryName
        } else if args[2].eq_ignore_ascii_case(b"LIB-VER") {
            ClientInfoAttribute::LibraryVersion
        } else {
            return Err(CommandError::InvalidArguments(
                "CLIENT SETINFO LIB-NAME|LIB-VER value",
            ));
        };
        return Ok(Command::ClientSetInfo {
            attribute,
            value: owned(args[3]),
        });
    }

    Err(CommandError::InvalidArguments(
        "CLIENT ID|GETNAME|SETNAME name|SETINFO LIB-NAME|LIB-VER value",
    ))
}

fn validate_client_metadata(value: &[u8]) -> Result<(), CommandError> {
    if value.iter().all(|byte| matches!(byte, b'!'..=b'~')) {
        Ok(())
    } else {
        Err(CommandError::InvalidClientMetadata)
    }
}

fn parse_command_metadata(args: &[&[u8]]) -> Result<Command, CommandError> {
    let Some(subcommand) = args.get(1) else {
        return Ok(Command::MetadataList);
    };
    if subcommand.eq_ignore_ascii_case(b"COUNT") {
        exact(args, 2, "COMMAND COUNT")?;
        return Ok(Command::MetadataCount);
    }
    if subcommand.eq_ignore_ascii_case(b"INFO") {
        return Ok(Command::MetadataInfo {
            names: args[2..].iter().map(|name| owned(name)).collect(),
        });
    }
    Err(CommandError::InvalidArguments(
        "COMMAND [INFO [command ...]|COUNT]",
    ))
}

fn parse_select(args: &[&[u8]]) -> Result<Command, CommandError> {
    exact(args, 2, "SELECT index")?;
    let index = parse_i64(args[1])?;
    if index == 0 {
        Ok(Command::Select)
    } else {
        Err(CommandError::UnsupportedDatabase(index))
    }
}

fn parse_flush(
    args: &[&[u8]],
    usage: &'static str,
    command: Command,
) -> Result<Command, CommandError> {
    match args {
        [_] => Ok(command),
        [_, mode] if mode.eq_ignore_ascii_case(b"SYNC") || mode.eq_ignore_ascii_case(b"ASYNC") => {
            Ok(command)
        }
        _ => Err(CommandError::InvalidArguments(usage)),
    }
}

fn key_i64(args: &[&[u8]], usage: &'static str) -> Result<(Vec<u8>, i64), CommandError> {
    exact(args, 3, usage)?;
    Ok((owned(args[1]), parse_i64(args[2])?))
}

fn range_args(args: &[&[u8]], usage: &'static str) -> Result<(Vec<u8>, i64, i64), CommandError> {
    exact(args, 4, usage)?;
    Ok((owned(args[1]), parse_i64(args[2])?, parse_i64(args[3])?))
}

fn parse_increment_by_float(args: &[&[u8]]) -> Result<Command, CommandError> {
    exact(args, 3, "INCRBYFLOAT key amount")?;
    let amount_text = String::from_utf8_lossy(args[2]);
    let amount = amount_text
        .parse::<f64>()
        .map_err(|_| CommandError::InvalidFloat(amount_text.to_string()))?;
    if !amount.is_finite() {
        return Err(CommandError::InvalidFloat(amount_text.to_string()));
    }
    Ok(Command::IncrementByFloat {
        key: owned(args[1]),
        amount,
    })
}

fn parse_i64(value: &[u8]) -> Result<i64, CommandError> {
    let text = String::from_utf8_lossy(value);
    text.parse()
        .map_err(|_| CommandError::InvalidInteger(text.to_string()))
}

fn parse_u64(value: &[u8]) -> Result<u64, CommandError> {
    let text = String::from_utf8_lossy(value);
    text.parse()
        .map_err(|_| CommandError::InvalidInteger(text.to_string()))
}

fn parse_usize(value: &[u8]) -> Result<usize, CommandError> {
    let text = String::from_utf8_lossy(value);
    text.parse()
        .map_err(|_| CommandError::InvalidInteger(text.to_string()))
}

fn owned(value: &[u8]) -> Vec<u8> {
    value.to_vec()
}
