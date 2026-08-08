use super::*;

#[test]
fn parses_set() {
    let command = Command::parse("SET profile display-value");

    assert_eq!(
        command,
        Ok(Command::Set {
            key: "profile".to_owned(),
            value: "display-value".to_owned(),
        })
    );
}

#[test]
fn parses_complex_string_set() {
    let command = Command::parse("SET message sample text");

    assert_eq!(
        command,
        Ok(Command::Set {
            key: "message".to_owned(),
            value: "sample text".to_owned(),
        })
    );
}

#[test]
fn commands_are_case_insensitive() {
    let command = Command::parse("get profile");

    assert_eq!(
        command,
        Ok(Command::Get {
            key: "profile".to_owned(),
        })
    );
}

#[test]
fn rejects_missing_argument() {
    let command = Command::parse("SET profile");

    assert_eq!(
        command,
        Err(CommandError::InvalidArguments("SET key value"))
    );
}

#[test]
fn rejects_extra_argument() {
    let command = Command::parse("GET profile extra");

    assert_eq!(command, Err(CommandError::InvalidArguments("GET key")));
}

#[test]
fn parses_expire() {
    let result = Command::parse("EXPIRE key 60");

    assert_eq!(
        result,
        Ok(Command::Expire {
            key: "key".to_owned(),
            seconds: 60,
        })
    );
}

#[test]
fn expire_requires_key_and_seconds() {
    assert_eq!(
        Command::parse("EXPIRE"),
        Err(CommandError::InvalidArguments("EXPIRE key seconds"))
    );

    assert_eq!(
        Command::parse("EXPIRE key"),
        Err(CommandError::InvalidArguments("EXPIRE key seconds"))
    );
}

#[test]
fn expire_rejects_invalid_seconds() {
    assert_eq!(
        Command::parse("EXPIRE key abc"),
        Err(CommandError::InvalidInteger("abc".to_owned()))
    );
}

#[test]
fn expire_rejects_negative_seconds() {
    assert_eq!(
        Command::parse("EXPIRE key -1"),
        Err(CommandError::InvalidInteger("-1".to_owned()))
    );
}

#[test]
fn expire_rejects_extra_arguments() {
    assert_eq!(
        Command::parse("EXPIRE key 10 extra"),
        Err(CommandError::InvalidArguments("EXPIRE key seconds"))
    );
}

#[test]
fn parses_increment_by_float() {
    let result = Command::parse("INCRBYFLOAT counter 1.5");

    assert_eq!(
        result,
        Ok(Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: 1.5,
        })
    );
}

#[test]
fn parses_increment_by_float_with_negative_amount() {
    let result = Command::parse("INCRBYFLOAT counter -2.25");

    assert_eq!(
        result,
        Ok(Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: -2.25,
        })
    );
}

#[test]
fn parses_increment_by_float_case_insensitively() {
    let result = Command::parse("incrbyfloat counter 1.5");

    assert_eq!(
        result,
        Ok(Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: 1.5,
        })
    );
}

#[test]
fn increment_by_float_requires_key() {
    let result = Command::parse("INCRBYFLOAT");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("INCRBYFLOAT key amount"))
    );
}

#[test]
fn increment_by_float_requires_amount() {
    let result = Command::parse("INCRBYFLOAT counter");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("INCRBYFLOAT key amount"))
    );
}

#[test]
fn increment_by_float_rejects_invalid_float() {
    let result = Command::parse("INCRBYFLOAT counter abc");

    assert_eq!(result, Err(CommandError::InvalidFloat("abc".to_owned())));
}

#[test]
fn increment_by_float_rejects_nan() {
    let result = Command::parse("INCRBYFLOAT counter NaN");

    assert_eq!(result, Err(CommandError::InvalidFloat("NaN".to_owned())));
}

#[test]
fn increment_by_float_rejects_infinity() {
    let result = Command::parse("INCRBYFLOAT counter inf");

    assert_eq!(result, Err(CommandError::InvalidFloat("inf".to_owned())));
}

#[test]
fn increment_by_float_rejects_extra_arguments() {
    let result = Command::parse("INCRBYFLOAT counter 1.5 extra");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("INCRBYFLOAT key amount"))
    );
}

#[test]
fn key_value_commands_accept_repeated_whitespace() {
    assert_eq!(
        Command::parse("SET   profile   sample value"),
        Ok(Command::Set {
            key: "profile".to_owned(),
            value: "sample value".to_owned(),
        })
    );
}

#[test]
fn set_range_accepts_repeated_whitespace() {
    assert_eq!(
        Command::parse("SETRANGE   key   2   sample value"),
        Ok(Command::SetRange {
            key: "key".to_owned(),
            offset: 2,
            value: "sample value".to_owned(),
        })
    );
}

#[test]
fn persist_reports_its_own_usage() {
    assert_eq!(
        Command::parse("PERSIST"),
        Err(CommandError::InvalidArguments("PERSIST key"))
    );
}

#[test]
fn rejects_empty_and_unknown_commands() {
    assert_eq!(Command::parse(" \t\n"), Err(CommandError::EmptyInput));
    assert_eq!(
        Command::parse("unsupported"),
        Err(CommandError::UnknownCommand("UNSUPPORTED".to_owned()))
    );
}

#[test]
fn parses_every_supported_command_form() {
    for input in [
        "MSET first one second two",
        "SETNX key value",
        "GET key",
        "MGET first second",
        "GETSET key value",
        "GETDEL key",
        "APPEND key suffix",
        "INCR counter",
        "INCRBY counter 2",
        "DECR counter",
        "DECRBY counter 2",
        "EXISTS key",
        "DEL key",
        "RENAME old new",
        "PEXPIRE key 500",
        "TTL key",
        "PTTL key",
        "PERSIST key",
        "STRLEN key",
        "GETRANGE key -2 -1",
        "SETRANGE key 2 value",
        "KEYS",
        "LEN",
        "CLEAR",
        "HELP",
        "EXIT",
        "QUIT",
    ] {
        assert!(Command::parse(input).is_ok(), "failed to parse {input}");
    }
}

#[test]
fn validates_collection_and_integer_command_arguments() {
    assert_eq!(
        Command::parse("MSET"),
        Err(CommandError::InvalidArguments(
            "MSET key value [key value ...]"
        ))
    );
    assert_eq!(
        Command::parse("MSET key"),
        Err(CommandError::InvalidArguments(
            "MSET key value [key value ...]"
        ))
    );
    assert_eq!(
        Command::parse("MGET"),
        Err(CommandError::InvalidArguments("MGET key [key ...]"))
    );
    assert_eq!(
        Command::parse("INCRBY counter nope"),
        Err(CommandError::InvalidInteger("nope".to_owned()))
    );
    assert_eq!(
        Command::parse("GETRANGE key start 1"),
        Err(CommandError::InvalidInteger("start".to_owned()))
    );
    assert_eq!(
        Command::parse("GETRANGE key 0 end"),
        Err(CommandError::InvalidInteger("end".to_owned()))
    );
    assert_eq!(
        Command::parse("SETRANGE key offset value"),
        Err(CommandError::InvalidInteger("offset".to_owned()))
    );
}

#[test]
fn command_errors_have_user_facing_messages() {
    assert_eq!(CommandError::EmptyInput.to_string(), "empty command");
    assert_eq!(
        CommandError::InvalidArguments("GET key").to_string(),
        "usage: GET key"
    );
    assert_eq!(
        CommandError::UnknownCommand("UNKNOWN".to_owned()).to_string(),
        "unknown command: UNKNOWN"
    );
    assert_eq!(
        CommandError::InvalidInteger("value".to_owned()).to_string(),
        "invalid integer: value"
    );
    assert_eq!(
        CommandError::InvalidFloat("value".to_owned()).to_string(),
        "invalid float: value"
    );
}
