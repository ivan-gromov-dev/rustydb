use super::arguments::{
    ensure_no_extra_arguments, parse_integer_argument_command, parse_key_value_command,
    parse_keys_argument_command, required_argument,
};
use super::types::{Command, CommandError};

impl Command {
    pub(crate) fn parse(input: &str) -> Result<Self, CommandError> {
        let input = input.trim();

        if input.is_empty() {
            return Err(CommandError::EmptyInput);
        }

        let command = input
            .split_whitespace()
            .next()
            .ok_or(CommandError::EmptyInput)?
            .to_ascii_uppercase();

        match command.as_str() {
            "SET" => parse_set(input),

            "MSET" => parse_mset(input),

            "SETNX" => parse_setnx(input),

            "GET" => parse_get(input),

            "MGET" => parse_mget(input),

            "GETSET" => parse_getset(input),

            "GETDEL" => parse_getdel(input),

            "APPEND" => parse_append(input),

            "INCR" => parse_increment(input),

            "INCRBY" => parse_incrby(input),

            "DECR" => parse_decrement(input),

            "DECRBY" => parse_decrby(input),

            "INCRBYFLOAT" => parse_increment_by_float(input),

            "EXISTS" => parse_exists(input),

            "DEL" => parse_del(input),

            "RENAME" => parse_rename(input),

            "EXPIRE" => parse_expire(input),

            "PEXPIRE" => parse_pexpire(input),

            "TTL" => parse_ttl(input),

            "PTTL" => parse_pttl(input),

            "PERSIST" => parse_persist(input),

            "STRLEN" => parse_strlen(input),

            "GETRANGE" => parse_getrange(input),

            "SETRANGE" => parse_setrange(input),

            "KEYS" => parse_keys(input),

            "LEN" => parse_len(input),

            "CLEAR" => parse_clear(input),

            "HELP" => parse_help(input),

            "EXIT" | "QUIT" => parse_exit(input),

            _ => Err(CommandError::UnknownCommand(command)),
        }
    }
}

fn parse_set(input: &str) -> Result<Command, CommandError> {
    let (key, value) = parse_key_value_command(input, "SET key value")?;

    Ok(Command::Set {
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

fn parse_mset(input: &str) -> Result<Command, CommandError> {
    let usage = "MSET key value [key value ...]";

    let mut parts = input.split_whitespace();
    parts.next();

    let mut entries = Vec::new();

    while let Some(key) = parts.next() {
        let Some(value) = parts.next() else {
            return Err(CommandError::InvalidArguments(usage));
        };

        entries.push((key.to_owned(), value.to_owned()));
    }

    if entries.is_empty() {
        return Err(CommandError::InvalidArguments(usage));
    }

    Ok(Command::MSet { entries })
}

fn parse_setnx(input: &str) -> Result<Command, CommandError> {
    let (key, value) = parse_key_value_command(input, "SETNX key value")?;
    Ok(Command::SetNx {
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

fn parse_get(input: &str) -> Result<Command, CommandError> {
    let usage = "GET key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::Get {
        key: key.to_owned(),
    })
}

fn parse_mget(input: &str) -> Result<Command, CommandError> {
    let usage = "MGET key [key ...]";

    let mut parts = input.split_whitespace();

    parts.next();

    let keys: Vec<String> = parts.map(str::to_owned).collect();

    if keys.is_empty() {
        return Err(CommandError::InvalidArguments(usage));
    }

    Ok(Command::MGet { keys })
}

fn parse_getset(input: &str) -> Result<Command, CommandError> {
    let (key, value) = parse_key_value_command(input, "GETSET key value")?;

    Ok(Command::GetSet {
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

fn parse_getdel(input: &str) -> Result<Command, CommandError> {
    let usage = "GETDEL key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::GetDel {
        key: key.to_owned(),
    })
}

fn parse_append(input: &str) -> Result<Command, CommandError> {
    let (key, value) = parse_key_value_command(input, "APPEND key value")?;

    Ok(Command::Append {
        key: key.to_owned(),
        append_value: value.to_owned(),
    })
}

fn parse_increment(input: &str) -> Result<Command, CommandError> {
    let usage = "INCR key";

    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::Increment {
        key: key.to_owned(),
    })
}

fn parse_incrby(input: &str) -> Result<Command, CommandError> {
    let (key, amount) = parse_integer_argument_command(input, "INCRBY key inc_value")?;

    Ok(Command::IncrementBy { key, amount })
}

fn parse_decrement(input: &str) -> Result<Command, CommandError> {
    let usage = "DECR key";

    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::Decrement {
        key: key.to_owned(),
    })
}

fn parse_decrby(input: &str) -> Result<Command, CommandError> {
    let (key, amount) = parse_integer_argument_command(input, "DECRBY key decr_value")?;

    Ok(Command::DecrementBy { key, amount })
}

fn parse_increment_by_float(input: &str) -> Result<Command, CommandError> {
    let usage = "INCRBYFLOAT key amount";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    let amount = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    let amount = amount
        .parse::<f64>()
        .map_err(|_| CommandError::InvalidFloat(amount.to_owned()))?;

    if !amount.is_finite() {
        return Err(CommandError::InvalidFloat(amount.to_string()));
    }

    Ok(Command::IncrementByFloat {
        key: key.to_owned(),
        amount,
    })
}

fn parse_del(input: &str) -> Result<Command, CommandError> {
    let keys = parse_keys_argument_command(input, "DEL key [key ...]")?;

    Ok(Command::Delete { keys })
}

fn parse_exists(input: &str) -> Result<Command, CommandError> {
    let keys = parse_keys_argument_command(input, "EXISTS key [key ...]")?;

    Ok(Command::Exists { keys })
}

fn parse_rename(input: &str) -> Result<Command, CommandError> {
    let usage = "RENAME old_key new_key";
    let mut parts = input.split_whitespace();

    parts.next();

    let old_key = required_argument(&mut parts, usage)?;
    let new_key = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::Rename {
        old_key: old_key.to_owned(),
        new_key: new_key.to_owned(),
    })
}

fn parse_expire(input: &str) -> Result<Command, CommandError> {
    let usage = "EXPIRE key seconds";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    let seconds = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    let seconds = seconds
        .parse::<u64>()
        .map_err(|_| CommandError::InvalidInteger(seconds.to_owned()))?;

    Ok(Command::Expire {
        key: key.to_owned(),
        seconds,
    })
}

fn parse_pexpire(input: &str) -> Result<Command, CommandError> {
    let usage = "PEXPIRE key milliseconds";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    let milliseconds = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    let milliseconds = milliseconds
        .parse::<u64>()
        .map_err(|_| CommandError::InvalidInteger(milliseconds.to_owned()))?;

    Ok(Command::PExpire {
        key: key.to_owned(),
        milliseconds,
    })
}

fn parse_ttl(input: &str) -> Result<Command, CommandError> {
    let usage = "TTL key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;
    Ok(Command::Ttl {
        key: key.to_owned(),
    })
}

fn parse_pttl(input: &str) -> Result<Command, CommandError> {
    let usage = "PTTL key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::PTtl {
        key: key.to_owned(),
    })
}

fn parse_persist(input: &str) -> Result<Command, CommandError> {
    let usage = "PERSIST key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;
    Ok(Command::Persist {
        key: key.to_owned(),
    })
}

fn parse_strlen(input: &str) -> Result<Command, CommandError> {
    let usage = "STRLEN key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::StrLen {
        key: key.to_owned(),
    })
}

fn parse_getrange(input: &str) -> Result<Command, CommandError> {
    let usage = "GETRANGE key start end";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    let start = required_argument(&mut parts, usage)?;
    let end = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    let start = start
        .parse::<i64>()
        .map_err(|_| CommandError::InvalidInteger(start.to_owned()))?;

    let end = end
        .parse::<i64>()
        .map_err(|_| CommandError::InvalidInteger(end.to_owned()))?;

    Ok(Command::GetRange {
        key: key.to_owned(),
        start,
        end,
    })
}

fn parse_setrange(input: &str) -> Result<Command, CommandError> {
    let usage = "SETRANGE key offset value";
    let arguments = input
        .split_once(char::is_whitespace)
        .map(|(_, arguments)| arguments.trim_start())
        .ok_or(CommandError::InvalidArguments(usage))?;
    let (key, arguments) = arguments
        .split_once(char::is_whitespace)
        .map(|(key, arguments)| (key, arguments.trim_start()))
        .ok_or(CommandError::InvalidArguments(usage))?;
    let (offset, value) = arguments
        .split_once(char::is_whitespace)
        .map(|(offset, value)| (offset, value.trim_start()))
        .ok_or(CommandError::InvalidArguments(usage))?;

    if key.is_empty() || offset.is_empty() || value.is_empty() {
        return Err(CommandError::InvalidArguments(usage));
    }

    let offset = offset
        .parse::<usize>()
        .map_err(|_| CommandError::InvalidInteger(offset.to_owned()))?;

    Ok(Command::SetRange {
        key: key.to_owned(),
        offset,
        value: value.to_owned(),
    })
}

fn parse_keys(input: &str) -> Result<Command, CommandError> {
    let mut parts = input.split_whitespace();

    parts.next();

    ensure_no_extra_arguments(&mut parts, "KEYS")?;
    Ok(Command::Keys)
}

fn parse_len(input: &str) -> Result<Command, CommandError> {
    let mut parts = input.split_whitespace();

    parts.next();

    ensure_no_extra_arguments(&mut parts, "LEN")?;
    Ok(Command::Len)
}

fn parse_clear(input: &str) -> Result<Command, CommandError> {
    let mut parts = input.split_whitespace();

    parts.next();

    ensure_no_extra_arguments(&mut parts, "CLEAR")?;
    Ok(Command::Clear)
}

fn parse_help(input: &str) -> Result<Command, CommandError> {
    let mut parts = input.split_whitespace();

    parts.next();

    ensure_no_extra_arguments(&mut parts, "HELP")?;
    Ok(Command::Help)
}

fn parse_exit(input: &str) -> Result<Command, CommandError> {
    let mut parts = input.split_whitespace();

    parts.next();

    ensure_no_extra_arguments(&mut parts, "EXIT")?;
    Ok(Command::Exit)
}
