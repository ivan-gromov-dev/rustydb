use super::types::{Command, CommandError};

impl Command {
    pub(crate) fn parse(input: &str) -> Result<Self, CommandError> {
        let input = input.trim();
        let Some(command) = input.split_whitespace().next() else {
            return Err(CommandError::EmptyInput);
        };

        let command = command.to_ascii_uppercase();
        let tail_after = match command.as_str() {
            "SET" | "SETNX" | "GETSET" | "APPEND" | "LPUSH" | "RPUSH" | "SADD" | "SREM"
            | "SISMEMBER" => Some(2),
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
        let Some(command) = args.first() else {
            return Err(CommandError::EmptyInput);
        };
        let command = command.to_ascii_uppercase();

        match command.as_str() {
            "SET" => {
                exact(args, 3, "SET key value")?;
                Ok(Self::Set {
                    key: owned(args[1]),
                    value: owned(args[2]),
                })
            }
            "MSET" => parse_mset(args),
            "SETNX" => {
                exact(args, 3, "SETNX key value")?;
                Ok(Self::SetNx {
                    key: owned(args[1]),
                    value: owned(args[2]),
                })
            }
            "GET" => Ok(Self::Get {
                key: one(args, "GET key")?,
            }),
            "MGET" => Ok(Self::MGet {
                keys: many(args, "MGET key [key ...]")?,
            }),
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
            "RENAME" => {
                exact(args, 3, "RENAME old_key new_key")?;
                Ok(Self::Rename {
                    old_key: owned(args[1]),
                    new_key: owned(args[2]),
                })
            }
            "EXPIRE" => {
                let (key, seconds) = key_u64(args, "EXPIRE key seconds")?;
                Ok(Self::Expire { key, seconds })
            }
            "PEXPIRE" => {
                let (key, milliseconds) = key_u64(args, "PEXPIRE key milliseconds")?;
                Ok(Self::PExpire { key, milliseconds })
            }
            "TTL" => Ok(Self::Ttl {
                key: one(args, "TTL key")?,
            }),
            "PTTL" => Ok(Self::PTtl {
                key: one(args, "PTTL key")?,
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
            "KEYS" => no_args(args, "KEYS", Self::Keys),
            "LEN" => no_args(args, "LEN", Self::Len),
            "CLEAR" => no_args(args, "CLEAR", Self::Clear),
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

fn exact(args: &[&str], length: usize, usage: &'static str) -> Result<(), CommandError> {
    if args.len() == length {
        Ok(())
    } else {
        Err(CommandError::InvalidArguments(usage))
    }
}

fn no_args(args: &[&str], usage: &'static str, command: Command) -> Result<Command, CommandError> {
    exact(args, 1, usage)?;
    Ok(command)
}

fn one(args: &[&str], usage: &'static str) -> Result<String, CommandError> {
    exact(args, 2, usage)?;
    Ok(owned(args[1]))
}

fn many(args: &[&str], usage: &'static str) -> Result<Vec<String>, CommandError> {
    if args.len() < 2 {
        return Err(CommandError::InvalidArguments(usage));
    }
    Ok(args[1..].iter().map(|value| owned(value)).collect())
}

fn parse_mset(args: &[&str]) -> Result<Command, CommandError> {
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

fn key_i64(args: &[&str], usage: &'static str) -> Result<(String, i64), CommandError> {
    exact(args, 3, usage)?;
    Ok((owned(args[1]), parse_i64(args[2])?))
}

fn key_u64(args: &[&str], usage: &'static str) -> Result<(String, u64), CommandError> {
    exact(args, 3, usage)?;
    Ok((owned(args[1]), parse_u64(args[2])?))
}

fn range_args(args: &[&str], usage: &'static str) -> Result<(String, i64, i64), CommandError> {
    exact(args, 4, usage)?;
    Ok((owned(args[1]), parse_i64(args[2])?, parse_i64(args[3])?))
}

fn parse_increment_by_float(args: &[&str]) -> Result<Command, CommandError> {
    exact(args, 3, "INCRBYFLOAT key amount")?;
    let amount = args[2]
        .parse::<f64>()
        .map_err(|_| CommandError::InvalidFloat(owned(args[2])))?;
    if !amount.is_finite() {
        return Err(CommandError::InvalidFloat(owned(args[2])));
    }
    Ok(Command::IncrementByFloat {
        key: owned(args[1]),
        amount,
    })
}

fn parse_i64(value: &str) -> Result<i64, CommandError> {
    value
        .parse()
        .map_err(|_| CommandError::InvalidInteger(owned(value)))
}

fn parse_u64(value: &str) -> Result<u64, CommandError> {
    value
        .parse()
        .map_err(|_| CommandError::InvalidInteger(owned(value)))
}

fn parse_usize(value: &str) -> Result<usize, CommandError> {
    value
        .parse()
        .map_err(|_| CommandError::InvalidInteger(owned(value)))
}

fn owned(value: &str) -> String {
    value.to_owned()
}
