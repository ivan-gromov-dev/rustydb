use super::*;

#[test]
fn parses_ping_with_an_optional_binary_message() {
    assert_eq!(Command::parse("PING"), Ok(Command::Ping { message: None }));
    assert_eq!(
        Command::parse("ping hello world"),
        Ok(Command::Ping {
            message: Some(b"hello world".to_vec()),
        })
    );
    assert_eq!(
        Command::from_bytes(&[b"PING", b"a\0\xff"]),
        Ok(Command::Ping {
            message: Some(b"a\0\xff".to_vec()),
        })
    );
    assert_eq!(
        Command::from_args(&["PING", "one", "two"]),
        Err(CommandError::InvalidArguments("PING [message]"))
    );
}

#[test]
fn echo_requires_one_binary_message() {
    assert_eq!(
        Command::parse("echo hello world"),
        Ok(Command::Echo {
            message: b"hello world".to_vec(),
        })
    );
    assert_eq!(
        Command::from_bytes(&[b"ECHO", b"a\0\xff"]),
        Ok(Command::Echo {
            message: b"a\0\xff".to_vec(),
        })
    );
    assert_eq!(
        Command::from_args(&["ECHO"]),
        Err(CommandError::InvalidArguments("ECHO message"))
    );
    assert_eq!(
        Command::from_args(&["ECHO", "one", "two"]),
        Err(CommandError::InvalidArguments("ECHO message"))
    );
}

#[test]
fn hello_accepts_current_and_supported_protocol_versions() {
    assert_eq!(
        Command::parse("HELLO"),
        Ok(Command::Hello { protocol: None })
    );
    assert_eq!(
        Command::parse("hello 2"),
        Ok(Command::Hello {
            protocol: Some(ProtocolVersion::Resp2),
        })
    );
    assert_eq!(
        Command::from_bytes(&[b"HELLO", b"3"]),
        Ok(Command::Hello {
            protocol: Some(ProtocolVersion::Resp3),
        })
    );
    assert_eq!(
        Command::parse("HELLO 4"),
        Err(CommandError::UnsupportedProtocol(4))
    );
    assert_eq!(
        Command::parse("HELLO nope"),
        Err(CommandError::InvalidInteger("nope".to_owned()))
    );
    assert_eq!(
        Command::parse("HELLO 3 extra"),
        Err(CommandError::InvalidArguments("HELLO [2|3]"))
    );
}

#[test]
fn save_accepts_no_arguments() {
    assert_eq!(Command::parse("SAVE"), Ok(Command::Save));
    assert_eq!(
        Command::parse("SAVE extra"),
        Err(CommandError::InvalidArguments("SAVE"))
    );
}

#[test]
fn aof_rewrite_accepts_no_arguments() {
    assert_eq!(Command::parse("AOFREWRITE"), Ok(Command::AofRewrite));
    assert_eq!(
        Command::parse("AOFREWRITE extra"),
        Err(CommandError::InvalidArguments("AOFREWRITE"))
    );
}

#[test]
fn info_accepts_no_arguments() {
    assert_eq!(Command::parse("INFO"), Ok(Command::Info));
    assert_eq!(
        Command::parse("INFO extra"),
        Err(CommandError::InvalidArguments("INFO"))
    );
}

#[test]
fn parses_exact_argument_vectors_without_retokenizing_values() {
    assert_eq!(
        Command::from_args(&["set", "key", "spaces\nnull\0byte"]),
        Ok(Command::Set {
            key: "key".to_owned().into(),
            value: "spaces\nnull\0byte".to_owned().into(),
        })
    );
    assert_eq!(
        Command::from_args(&["MSET", "first", "one two", "second", "three\nfour"]),
        Ok(Command::MSet {
            entries: vec![
                ("first".to_owned().into(), "one two".to_owned().into()),
                ("second".to_owned().into(), "three\nfour".to_owned().into()),
            ],
        })
    );
}

#[test]
fn parses_binary_argument_vectors_without_utf8_conversion() {
    assert_eq!(
        Command::from_bytes(&[b"SET", b"\xff-key", b"a\0\x80"]),
        Ok(Command::Set {
            key: b"\xff-key".to_vec(),
            value: b"a\0\x80".to_vec(),
        })
    );
}

#[test]
fn common_binary_commands_match_ascii_case_without_changing_validation() {
    assert_eq!(
        Command::from_bytes(&[b"gEt", b"key"]),
        Ok(Command::Get {
            key: b"key".to_vec()
        })
    );
    assert_eq!(
        Command::from_bytes(&[b"sEt", b"key", b"value"]),
        Ok(Command::Set {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        })
    );
    assert_eq!(
        Command::from_bytes(&[b"gEt", b"key", b"extra"]),
        Err(CommandError::InvalidArguments("GET key"))
    );
}

#[test]
fn common_owned_commands_move_arguments_and_preserve_validation() {
    assert_eq!(
        Command::from_owned_bytes(vec![b"gEt".to_vec(), b"key".to_vec()]),
        Ok(Command::Get {
            key: b"key".to_vec()
        })
    );
    assert_eq!(
        Command::from_owned_bytes(vec![b"sEt".to_vec(), b"key".to_vec(), b"value".to_vec(),]),
        Ok(Command::Set {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        })
    );
    assert_eq!(
        Command::from_owned_bytes(vec![b"sEt".to_vec(), b"key".to_vec()]),
        Err(CommandError::InvalidArguments("SET key value"))
    );
}

#[test]
fn argument_vectors_use_the_same_validation_as_text_commands() {
    assert_eq!(Command::from_args(&[]), Err(CommandError::EmptyInput));
    assert_eq!(
        Command::from_args(&["GET", "key", "extra"]),
        Err(CommandError::InvalidArguments("GET key"))
    );
    assert_eq!(
        Command::from_args(&["EXPIRE", "key", "nope"]),
        Err(CommandError::InvalidInteger("nope".to_owned()))
    );
    assert_eq!(
        Command::from_args(&["unknown"]),
        Err(CommandError::UnknownCommand("UNKNOWN".to_owned()))
    );
}

#[test]
fn parses_set() {
    let command = Command::parse("SET profile display-value");

    assert_eq!(
        command,
        Ok(Command::Set {
            key: "profile".to_owned().into(),
            value: "display-value".to_owned().into(),
        })
    );
}

#[test]
fn parses_complex_string_set() {
    let command = Command::parse("SET message sample text");

    assert_eq!(
        command,
        Ok(Command::Set {
            key: "message".to_owned().into(),
            value: "sample text".to_owned().into(),
        })
    );
}

#[test]
fn commands_are_case_insensitive() {
    let command = Command::parse("get profile");

    assert_eq!(
        command,
        Ok(Command::Get {
            key: "profile".to_owned().into(),
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
            key: "key".to_owned().into(),
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
            key: "counter".to_owned().into(),
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
            key: "counter".to_owned().into(),
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
            key: "counter".to_owned().into(),
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
            key: "profile".to_owned().into(),
            value: "sample value".to_owned().into(),
        })
    );
}

#[test]
fn set_range_accepts_repeated_whitespace() {
    assert_eq!(
        Command::parse("SETRANGE   key   2   sample value"),
        Ok(Command::SetRange {
            key: "key".to_owned().into(),
            offset: 2,
            value: "sample value".to_owned().into(),
        })
    );
}

#[test]
fn parses_list_commands() {
    assert_eq!(
        Command::parse("lpush queue first item"),
        Ok(Command::LPush {
            key: "queue".to_owned().into(),
            value: "first item".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("RPUSH queue last item"),
        Ok(Command::RPush {
            key: "queue".to_owned().into(),
            value: "last item".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("LLEN queue"),
        Ok(Command::LLen {
            key: "queue".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("lpop queue"),
        Ok(Command::LPop {
            key: "queue".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("RPOP queue"),
        Ok(Command::RPop {
            key: "queue".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("LRANGE queue -2 -1"),
        Ok(Command::LRange {
            key: "queue".to_owned().into(),
            start: -2,
            end: -1,
        })
    );
}

#[test]
fn list_commands_validate_arguments() {
    for input in ["LPUSH", "LPUSH key", "RPUSH", "RPUSH key"] {
        assert!(matches!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(_))
        ));
    }

    assert_eq!(
        Command::parse("LLEN"),
        Err(CommandError::InvalidArguments("LLEN key"))
    );
    assert_eq!(
        Command::parse("LLEN key extra"),
        Err(CommandError::InvalidArguments("LLEN key"))
    );
    for (input, usage) in [
        ("LPOP", "LPOP key"),
        ("LPOP key extra", "LPOP key"),
        ("RPOP", "RPOP key"),
        ("RPOP key extra", "RPOP key"),
    ] {
        assert_eq!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(usage))
        );
    }
    for input in [
        "LRANGE",
        "LRANGE key",
        "LRANGE key 0",
        "LRANGE key 0 1 extra",
    ] {
        assert_eq!(
            Command::parse(input),
            Err(CommandError::InvalidArguments("LRANGE key start end"))
        );
    }
    assert_eq!(
        Command::parse("LRANGE key start 1"),
        Err(CommandError::InvalidInteger("start".to_owned()))
    );
    assert_eq!(
        Command::parse("LRANGE key 0 end"),
        Err(CommandError::InvalidInteger("end".to_owned()))
    );
}

#[test]
fn parses_set_collection_commands() {
    assert_eq!(
        Command::parse("sadd tags first member"),
        Ok(Command::SAdd {
            key: "tags".to_owned().into(),
            member: "first member".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("SREM tags first member"),
        Ok(Command::SRem {
            key: "tags".to_owned().into(),
            member: "first member".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("SISMEMBER tags first member"),
        Ok(Command::SIsMember {
            key: "tags".to_owned().into(),
            member: "first member".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("SMEMBERS tags"),
        Ok(Command::SMembers {
            key: "tags".to_owned().into(),
        })
    );
    assert_eq!(
        Command::parse("SCARD tags"),
        Ok(Command::SCard {
            key: "tags".to_owned().into(),
        })
    );
}

#[test]
fn set_collection_commands_validate_arguments() {
    for input in [
        "SADD",
        "SADD key",
        "SREM",
        "SREM key",
        "SISMEMBER",
        "SISMEMBER key",
    ] {
        assert!(matches!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(_))
        ));
    }

    for (input, usage) in [
        ("SMEMBERS", "SMEMBERS key"),
        ("SMEMBERS key extra", "SMEMBERS key"),
        ("SCARD", "SCARD key"),
        ("SCARD key extra", "SCARD key"),
    ] {
        assert_eq!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(usage))
        );
    }
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
        "LPUSH list first",
        "RPUSH list last",
        "LLEN list",
        "LPOP list",
        "RPOP list",
        "LRANGE list 0 -1",
        "SADD set member",
        "SREM set member",
        "SISMEMBER set member",
        "SMEMBERS set",
        "SCARD set",
        "PING",
        "PING message",
        "ECHO message",
        "HELLO",
        "HELLO 2",
        "HELLO 3",
        "KEYS",
        "LEN",
        "CLEAR",
        "SAVE",
        "AOFREWRITE",
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

#[test]
fn parses_delete_with_single_key() {
    let result = Command::parse("DEL a");

    assert_eq!(
        result,
        Ok(Command::Delete {
            keys: vec!["a".to_owned().into()],
        })
    );
}

#[test]
fn parses_delete_with_multiple_keys() {
    let result = Command::parse("DEL a b c");

    assert_eq!(
        result,
        Ok(Command::Delete {
            keys: vec![
                "a".to_owned().into(),
                "b".to_owned().into(),
                "c".to_owned().into(),
            ],
        })
    );
}

#[test]
fn delete_requires_at_least_one_key() {
    let result = Command::parse("DEL");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("DEL key [key ...]"))
    );
}

#[test]
fn parses_exists_with_single_key() {
    let result = Command::parse("EXISTS a");

    assert_eq!(
        result,
        Ok(Command::Exists {
            keys: vec!["a".to_owned().into()],
        })
    );
}

#[test]
fn parses_exists_with_multiple_keys() {
    let result = Command::parse("EXISTS a b c");

    assert_eq!(
        result,
        Ok(Command::Exists {
            keys: vec![
                "a".to_owned().into(),
                "b".to_owned().into(),
                "c".to_owned().into(),
            ],
        })
    );
}

#[test]
fn exists_requires_at_least_one_key() {
    let result = Command::parse("EXISTS");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("EXISTS key [key ...]"))
    );
}
