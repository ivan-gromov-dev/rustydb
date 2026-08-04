use super::helper::{ensure_no_extra_arguments, required_argument};
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

            "GET" => parse_get(input),

            "MGET" => parse_mget(input),

            "APPEND" => parse_append(input),

            "INCR" => parse_increment(input),

            "INCRBY" => parse_incrby(input),

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
    let usage = "SET key value";

    let mut parts = input.splitn(3, char::is_whitespace);

    parts.next();

    let key = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CommandError::InvalidArguments(usage))?;

    let value = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CommandError::InvalidArguments(usage))?;

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

    Ok(Command::Mset { entries })
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

fn parse_append(input: &str) -> Result<Command, CommandError> {
    let usage = "APPEND key value";

    let mut parts = input.splitn(3, char::is_whitespace);

    parts.next();

    let key = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CommandError::InvalidArguments(usage))?;

    let value = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(CommandError::InvalidArguments(usage))?;

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
    let usage = "INCRBY key inc_value";
    let mut parts = input.split_whitespace();

    parts.next();

    let key = required_argument(&mut parts, usage)?;
    let inc_value = required_argument(&mut parts, usage)?;

    ensure_no_extra_arguments(&mut parts, usage)?;

    match inc_value.parse::<i64>() {
        Ok(value) => Ok(Command::IncrementBy {
            key: key.to_owned(),
            inc_value: value,
        }),
        Err(_) => Err(CommandError::InvalidInteger(inc_value.to_owned())),
    }
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
