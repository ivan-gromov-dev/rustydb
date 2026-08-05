use super::helper::{
    ensure_no_extra_arguments, parse_integer_argument_command, parse_key_value_command,
    required_argument,
};
use super::model::{Command, CommandError};

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

            "EXISTS" => parse_exists(input),

            "DEL" => parse_del(input),

            "RENAME" => parse_rename(input),

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

fn parse_exists(input: &str) -> Result<Command, CommandError> {
    let usage = "EXISTS key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::Exists {
        key: key.to_owned(),
    })
}

fn parse_del(input: &str) -> Result<Command, CommandError> {
    let usage = "DEL key";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    ensure_no_extra_arguments(&mut parts, usage)?;

    Ok(Command::Delete {
        key: key.to_owned(),
    })
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
