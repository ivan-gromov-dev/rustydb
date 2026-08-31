use super::*;

#[test]
fn parses_type_touch_and_unlink() {
    assert_eq!(
        Command::from_args(&["TYPE", "key"]),
        Ok(Command::Type {
            key: b"key".to_vec()
        })
    );
    assert_eq!(
        Command::from_args(&["TOUCH", "a", "b"]),
        Ok(Command::Touch {
            keys: vec![b"a".to_vec(), b"b".to_vec()]
        })
    );
    assert_eq!(
        Command::from_args(&["UNLINK", "a", "b"]),
        Ok(Command::Unlink {
            keys: vec![b"a".to_vec(), b"b".to_vec()]
        })
    );
    assert_eq!(
        Command::from_args(&["TYPE"]),
        Err(CommandError::InvalidArguments("TYPE key"))
    );
    assert_eq!(
        Command::from_args(&["TOUCH"]),
        Err(CommandError::InvalidArguments("TOUCH key [key ...]"))
    );
}

#[test]
fn unlink_is_persisted_as_del() {
    let command = Command::Unlink {
        keys: vec![b"a".to_vec(), b"b".to_vec()],
    };

    assert_eq!(
        command.aof_arguments(),
        Some(vec![b"DEL".to_vec(), b"a".to_vec(), b"b".to_vec()])
    );
}

#[test]
fn parses_keyspace_iteration_and_copy_options() {
    assert_eq!(
        Command::from_args(&["KEYS", "user:*"]),
        Ok(Command::Keys {
            pattern: b"user:*".to_vec()
        })
    );
    assert_eq!(
        Command::from_args(&["SCAN", "2", "TYPE", "list", "COUNT", "3", "MATCH", "q:*"]),
        Ok(Command::Scan {
            cursor: 2,
            pattern: Some(b"q:*".to_vec()),
            count: 3,
            type_name: Some(b"list".to_vec())
        })
    );
    assert_eq!(
        Command::from_args(&["COPY", "source", "destination", "REPLACE", "DB", "0"]),
        Ok(Command::Copy {
            source: b"source".to_vec(),
            destination: b"destination".to_vec(),
            replace: true
        })
    );
    assert_eq!(
        Command::from_args(&["COPY", "a", "b", "DB", "1"]),
        Err(CommandError::UnsupportedDatabase(1))
    );
    for invalid in [
        vec!["KEYS"],
        vec!["SCAN", "0", "COUNT", "0"],
        vec!["SCAN", "0", "MATCH"],
        vec!["SCAN", "0", "TYPE", "set", "TYPE", "list"],
        vec!["COPY", "a", "b", "REPLACE", "REPLACE"],
    ] {
        assert!(matches!(
            Command::from_args(&invalid),
            Err(CommandError::InvalidArguments(_))
        ));
    }
}

#[test]
fn copy_aof_record_is_replay_safe() {
    let command = Command::Copy {
        source: b"source".to_vec(),
        destination: b"destination".to_vec(),
        replace: false,
    };
    assert_eq!(
        command.aof_arguments(),
        Some(vec![
            b"COPY".to_vec(),
            b"source".to_vec(),
            b"destination".to_vec(),
            b"REPLACE".to_vec()
        ])
    );
}

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
fn parses_connection_metadata_commands() {
    assert_eq!(Command::parse("CLIENT ID"), Ok(Command::ClientId));
    assert_eq!(Command::parse("client getname"), Ok(Command::ClientGetName));
    assert_eq!(
        Command::from_bytes(&[b"CLIENT", b"SETNAME", b"worker-1"]),
        Ok(Command::ClientSetName {
            name: b"worker-1".to_vec(),
        })
    );
    assert_eq!(
        Command::parse("CLIENT SETINFO LIB-NAME redis-rs"),
        Ok(Command::ClientSetInfo {
            attribute: ClientInfoAttribute::LibraryName,
            value: b"redis-rs".to_vec(),
        })
    );
    assert_eq!(
        Command::parse("CLIENT SETINFO lib-ver 0.27.6"),
        Ok(Command::ClientSetInfo {
            attribute: ClientInfoAttribute::LibraryVersion,
            value: b"0.27.6".to_vec(),
        })
    );
}

#[test]
fn connection_metadata_commands_validate_subcommands_and_values() {
    for input in [
        "CLIENT",
        "CLIENT ID extra",
        "CLIENT GETNAME extra",
        "CLIENT SETNAME",
        "CLIENT SETINFO LIB-NAME",
        "CLIENT SETINFO unknown value",
        "CLIENT unknown",
    ] {
        assert!(matches!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(_))
        ));
    }
    for arguments in [
        &[b"CLIENT".as_slice(), b"SETNAME", b"two words"][..],
        &[b"CLIENT".as_slice(), b"SETINFO", b"LIB-NAME", b"bad\nname"][..],
    ] {
        assert_eq!(
            Command::from_bytes(arguments),
            Err(CommandError::InvalidClientMetadata)
        );
    }
    assert_eq!(
        Command::from_bytes(&[b"CLIENT", b"SETNAME", b""]),
        Ok(Command::ClientSetName { name: Vec::new() })
    );
}

#[test]
fn parses_command_metadata_queries() {
    assert_eq!(Command::parse("COMMAND"), Ok(Command::MetadataList));
    assert_eq!(Command::parse("command count"), Ok(Command::MetadataCount));
    assert_eq!(
        Command::parse("COMMAND INFO GET missing"),
        Ok(Command::MetadataInfo {
            names: vec![b"GET".to_vec(), b"missing".to_vec()],
        })
    );
    assert_eq!(
        Command::parse("COMMAND INFO"),
        Ok(Command::MetadataInfo { names: Vec::new() })
    );
    for input in ["COMMAND COUNT extra", "COMMAND unknown"] {
        assert!(matches!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(_))
        ));
    }
}

#[test]
fn select_accepts_only_database_zero() {
    assert_eq!(Command::parse("SELECT 0"), Ok(Command::Select));
    assert_eq!(
        Command::parse("SELECT 1"),
        Err(CommandError::UnsupportedDatabase(1))
    );
    assert_eq!(
        Command::parse("SELECT -1"),
        Err(CommandError::UnsupportedDatabase(-1))
    );
    assert_eq!(
        Command::parse("SELECT nope"),
        Err(CommandError::InvalidInteger("nope".to_owned()))
    );
    assert_eq!(
        Command::parse("SELECT 0 extra"),
        Err(CommandError::InvalidArguments("SELECT index"))
    );
}

#[test]
fn parses_database_size_and_flush_modes() {
    assert_eq!(Command::parse("DBSIZE"), Ok(Command::DbSize));
    assert_eq!(Command::parse("FLUSHDB"), Ok(Command::FlushDb));
    assert_eq!(Command::parse("flushdb async"), Ok(Command::FlushDb));
    assert_eq!(Command::parse("FLUSHALL SYNC"), Ok(Command::FlushAll));
    for input in ["DBSIZE extra", "FLUSHDB unknown", "FLUSHALL SYNC extra"] {
        assert!(matches!(
            Command::parse(input),
            Err(CommandError::InvalidArguments(_))
        ));
    }
}

#[test]
fn flush_commands_have_canonical_aof_records() {
    assert_eq!(
        Command::FlushDb.aof_arguments(),
        Some(vec![b"FLUSHDB".to_vec()])
    );
    assert_eq!(
        Command::FlushAll.aof_arguments(),
        Some(vec![b"FLUSHALL".to_vec()])
    );
    assert_eq!(Command::Select.aof_arguments(), None);
    assert_eq!(Command::DbSize.aof_arguments(), None);
}

#[test]
fn command_metadata_registry_is_sorted_unique_and_searchable() {
    assert!(COMMANDS.windows(2).all(|pair| pair[0].name < pair[1].name));
    assert_eq!(
        command_metadata(b"GeT"),
        Some(CommandMetadata {
            name: "get",
            arity: 2,
            flags: &["readonly", "fast"],
            first_key: 1,
            last_key: 1,
            key_step: 1,
        })
    );
    assert_eq!(command_metadata(b"\xff"), None);
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
        ("LPOP", "LPOP key [count]"),
        ("LPOP key 1 extra", "LPOP key [count]"),
        ("RPOP", "RPOP key [count]"),
        ("RPOP key 1 extra", "RPOP key [count]"),
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
fn parses_hash_commands_and_validates_arity() {
    assert_eq!(
        Command::from_args(&["HSET", "record", "name", "Ada", "role", "admin"]),
        Ok(Command::HSet {
            key: b"record".to_vec(),
            entries: vec![
                (b"name".to_vec(), b"Ada".to_vec()),
                (b"role".to_vec(), b"admin".to_vec())
            ]
        })
    );
    assert_eq!(
        Command::from_args(&["HMGET", "record", "name", "missing"]),
        Ok(Command::HMGet {
            key: b"record".to_vec(),
            fields: vec![b"name".to_vec(), b"missing".to_vec()]
        })
    );
    assert_eq!(
        Command::from_args(&["HDEL", "record", "name", "role"]),
        Ok(Command::HDel {
            key: b"record".to_vec(),
            fields: vec![b"name".to_vec(), b"role".to_vec()]
        })
    );
    for command in [
        "HSET key field",
        "HSET key field value extra",
        "HMGET key",
        "HDEL key",
        "HLEN key extra",
    ] {
        assert!(
            matches!(
                Command::parse(command),
                Err(CommandError::InvalidArguments(_))
            ),
            "accepted {command}"
        );
    }
}

#[test]
fn parses_hash_numeric_and_scan_commands() {
    assert_eq!(
        Command::from_args(&["HINCRBY", "hash", "count", "-2"]),
        Ok(Command::HIncrementBy {
            key: b"hash".to_vec(),
            field: b"count".to_vec(),
            amount: -2
        })
    );
    assert_eq!(
        Command::from_args(&["HINCRBYFLOAT", "hash", "score", "1.5"]),
        Ok(Command::HIncrementByFloat {
            key: b"hash".to_vec(),
            field: b"score".to_vec(),
            amount: 1.5
        })
    );
    assert_eq!(
        Command::from_args(&["HSCAN", "hash", "2", "COUNT", "5", "MATCH", "user:*"]),
        Ok(Command::HScan {
            key: b"hash".to_vec(),
            cursor: 2,
            pattern: Some(b"user:*".to_vec()),
            count: 5
        })
    );
    for command in [
        "HINCRBY hash field nope",
        "HINCRBYFLOAT hash field inf",
        "HSCAN hash",
        "HSCAN hash 0 COUNT 0",
        "HSCAN hash 0 TYPE string",
    ] {
        assert!(Command::parse(command).is_err(), "accepted {command}");
    }
}

#[test]
fn hash_numeric_mutations_have_replayable_aof_arguments() {
    assert_eq!(
        Command::HIncrementBy {
            key: b"hash".to_vec(),
            field: b"count".to_vec(),
            amount: -2
        }
        .aof_arguments(),
        Some(vec![
            b"HINCRBY".to_vec(),
            b"hash".to_vec(),
            b"count".to_vec(),
            b"-2".to_vec()
        ])
    );
    assert_eq!(
        Command::HIncrementByFloat {
            key: b"hash".to_vec(),
            field: b"score".to_vec(),
            amount: 1.5
        }
        .aof_arguments(),
        Some(vec![
            b"HINCRBYFLOAT".to_vec(),
            b"hash".to_vec(),
            b"score".to_vec(),
            b"1.5".to_vec()
        ])
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
        "CLIENT ID",
        "CLIENT GETNAME",
        "CLIENT SETNAME worker",
        "CLIENT SETINFO LIB-NAME redis-rs",
        "CLIENT SETINFO LIB-VER 1.0",
        "COMMAND",
        "COMMAND COUNT",
        "COMMAND INFO",
        "COMMAND INFO GET missing",
        "SELECT 0",
        "DBSIZE",
        "FLUSHDB",
        "FLUSHDB ASYNC",
        "FLUSHALL",
        "FLUSHALL SYNC",
        "KEYS *",
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

#[test]
fn parses_set_options_in_any_supported_order() {
    assert_eq!(
        Command::parse("SET key value GET NX PX 1500"),
        Ok(Command::SetAdvanced {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
            condition: Some(SetCondition::IfAbsent),
            return_old: true,
            expiration: Some(SetExpiration::Milliseconds(1500)),
        })
    );
}

#[test]
fn set_rejects_conflicting_or_incomplete_options() {
    for command in [
        "SET key value NX XX",
        "SET key value EX 1 KEEPTTL",
        "SET key value GET GET",
        "SET key value PX",
        "SET key value EX 0",
    ] {
        assert!(matches!(
            Command::parse(command),
            Err(CommandError::InvalidArguments(_))
        ));
    }
    assert!(matches!(
        Command::from_args(&["SET", "key", "value", "UNKNOWN"]),
        Err(CommandError::InvalidArguments(_))
    ));
}

#[test]
fn parses_getex_and_msetnx() {
    assert_eq!(
        Command::parse("GETEX key PXAT 2000"),
        Ok(Command::GetEx {
            key: b"key".to_vec(),
            expiration: Some(GetExExpiration::UnixMilliseconds(2000)),
        })
    );
    assert_eq!(
        Command::parse("GETEX key PERSIST"),
        Ok(Command::GetEx {
            key: b"key".to_vec(),
            expiration: Some(GetExExpiration::Persist),
        })
    );
    assert_eq!(
        Command::parse("MSETNX one 1 two 2"),
        Ok(Command::MSetNx {
            entries: vec![
                (b"one".to_vec(), b"1".to_vec()),
                (b"two".to_vec(), b"2".to_vec())
            ],
        })
    );
}

#[test]
fn getex_and_msetnx_reject_malformed_arguments() {
    for command in [
        "GETEX",
        "GETEX key PX",
        "GETEX key EX 0",
        "GETEX key PERSIST EX 1",
        "MSETNX key",
        "MSETNX one 1 two",
    ] {
        assert!(matches!(
            Command::parse(command),
            Err(CommandError::InvalidArguments(_))
        ));
    }
}

#[test]
fn parses_absolute_and_conditional_expiration_commands() {
    assert_eq!(
        Command::parse("EXPIRE key 10 GT"),
        Ok(Command::ExpireAdvanced {
            key: b"key".to_vec(),
            seconds: 10,
            condition: ExpireCondition::Greater,
        })
    );
    assert_eq!(
        Command::parse("PEXPIREAT key 1234 NX"),
        Ok(Command::PExpireAt {
            key: b"key".to_vec(),
            unix_milliseconds: 1234,
            condition: Some(ExpireCondition::NoExpiration),
        })
    );
    assert_eq!(
        Command::parse("EXPIRETIME key"),
        Ok(Command::ExpireTime {
            key: b"key".to_vec()
        })
    );
    assert_eq!(
        Command::parse("PEXPIRETIME key"),
        Ok(Command::PExpireTime {
            key: b"key".to_vec()
        })
    );
}

#[test]
fn expiration_conditions_reject_unknown_or_repeated_options() {
    for command in [
        "EXPIRE key 10 NX XX",
        "PEXPIRE key 10 UNKNOWN",
        "EXPIREAT key 10 GT LT",
        "PEXPIREAT key",
        "EXPIRETIME key extra",
    ] {
        assert!(matches!(
            Command::parse(command),
            Err(CommandError::InvalidArguments(_))
        ));
    }
}

#[test]
fn parses_and_persists_variadic_collection_mutations() {
    let lpush = Command::from_owned_bytes(vec![
        b"LPUSH".to_vec(),
        b"list".to_vec(),
        b"one".to_vec(),
        b"two".to_vec(),
    ])
    .unwrap();
    assert_eq!(
        lpush,
        Command::LPushMany {
            key: b"list".to_vec(),
            values: vec![b"one".to_vec(), b"two".to_vec()],
        }
    );
    assert_eq!(
        lpush.aof_arguments(),
        Some(vec![
            b"LPUSH".to_vec(),
            b"list".to_vec(),
            b"one".to_vec(),
            b"two".to_vec()
        ])
    );

    assert_eq!(
        Command::from_owned_bytes(vec![
            b"SREM".to_vec(),
            b"set".to_vec(),
            b"one".to_vec(),
            b"two".to_vec(),
        ]),
        Ok(Command::SRemMany {
            key: b"set".to_vec(),
            members: vec![b"one".to_vec(), b"two".to_vec()],
        })
    );
}

#[test]
fn parses_counted_list_pops_and_rejects_invalid_counts() {
    let command =
        Command::from_owned_bytes(vec![b"LPOP".to_vec(), b"list".to_vec(), b"2".to_vec()]).unwrap();
    assert_eq!(
        command,
        Command::LPopCount {
            key: b"list".to_vec(),
            count: 2,
        }
    );
    assert_eq!(
        command.aof_arguments(),
        Some(vec![b"LPOP".to_vec(), b"list".to_vec(), b"2".to_vec()])
    );
    assert!(matches!(
        Command::parse("RPOP list -1"),
        Err(CommandError::InvalidInteger(_))
    ));
    assert!(matches!(
        Command::parse("LPOP list 1 extra"),
        Err(CommandError::InvalidArguments("LPOP key [count]"))
    ));
}

#[test]
fn parses_and_persists_conditional_variadic_pushes() {
    let command = Command::from_owned_bytes(vec![
        b"LPUSHX".to_vec(),
        b"list".to_vec(),
        b"one".to_vec(),
        b"two".to_vec(),
    ])
    .unwrap();
    assert_eq!(
        command,
        Command::LPushX {
            key: b"list".to_vec(),
            values: vec![b"one".to_vec(), b"two".to_vec()],
        }
    );
    assert_eq!(
        command.aof_arguments(),
        Some(vec![
            b"LPUSHX".to_vec(),
            b"list".to_vec(),
            b"one".to_vec(),
            b"two".to_vec(),
        ])
    );
    assert!(matches!(
        Command::parse("RPUSHX list"),
        Err(CommandError::InvalidArguments(
            "RPUSHX key value [value ...]"
        ))
    ));
}

#[test]
fn parses_list_index_and_set_with_replayable_binary_values() {
    assert_eq!(
        Command::parse("LINDEX list -1"),
        Ok(Command::LIndex {
            key: b"list".to_vec(),
            index: -1,
        })
    );
    let command = Command::from_owned_bytes(vec![
        b"LSET".to_vec(),
        b"list".to_vec(),
        b"-2".to_vec(),
        b"binary\0value".to_vec(),
    ])
    .unwrap();
    assert_eq!(
        command.aof_arguments(),
        Some(vec![
            b"LSET".to_vec(),
            b"list".to_vec(),
            b"-2".to_vec(),
            b"binary\0value".to_vec(),
        ])
    );
    for input in ["LINDEX list", "LINDEX list nope", "LSET list 0"] {
        assert!(Command::parse(input).is_err());
    }
}

#[test]
fn parses_extended_list_commands_and_options() {
    assert!(matches!(
        Command::parse("LINSERT list BEFORE pivot value"),
        Ok(Command::LInsert {
            position: InsertPosition::Before,
            ..
        })
    ));
    assert_eq!(
        Command::parse("LTRIM list -2 -1"),
        Ok(Command::LTrim {
            key: b"list".to_vec(),
            start: -2,
            end: -1
        })
    );
    assert_eq!(
        Command::parse("LREM list -2 value"),
        Ok(Command::LRem {
            key: b"list".to_vec(),
            count: -2,
            value: b"value".to_vec()
        })
    );
    assert_eq!(
        Command::parse("LPOS list value RANK -2 COUNT 0 MAXLEN 20"),
        Ok(Command::LPos {
            key: b"list".to_vec(),
            value: b"value".to_vec(),
            rank: -2,
            count: Some(0),
            max_len: Some(20)
        })
    );
    assert_eq!(
        Command::parse("LMOVE source destination RIGHT LEFT"),
        Ok(Command::LMove {
            source: b"source".to_vec(),
            destination: b"destination".to_vec(),
            source_end: ListEnd::Right,
            destination_end: ListEnd::Left
        })
    );
    assert_eq!(
        Command::parse("RPOPLPUSH source destination"),
        Ok(Command::RPopLPush {
            source: b"source".to_vec(),
            destination: b"destination".to_vec()
        })
    );
    for input in [
        "LINSERT list MIDDLE pivot value",
        "LPOS list value RANK 0",
        "LPOS list value COUNT 1 COUNT 2",
        "LPOS list value RANK 1 RANK 2",
        "LMOVE source destination UP LEFT",
        "RPOPLPUSH source",
    ] {
        assert!(Command::parse(input).is_err(), "{input}");
    }
}

#[test]
fn extended_list_mutations_have_replayable_aof_arguments() {
    let commands = [
        Command::LInsert {
            key: b"list".to_vec(),
            position: InsertPosition::After,
            pivot: b"pivot".to_vec(),
            value: b"value".to_vec(),
        },
        Command::LTrim {
            key: b"list".to_vec(),
            start: 1,
            end: -1,
        },
        Command::LRem {
            key: b"list".to_vec(),
            count: 0,
            value: b"value".to_vec(),
        },
        Command::LMove {
            source: b"a".to_vec(),
            destination: b"b".to_vec(),
            source_end: ListEnd::Left,
            destination_end: ListEnd::Right,
        },
        Command::RPopLPush {
            source: b"a".to_vec(),
            destination: b"b".to_vec(),
        },
    ];
    for command in commands {
        let arguments = command.aof_arguments().unwrap();
        assert_eq!(Command::from_owned_bytes(arguments), Ok(command));
    }
}
