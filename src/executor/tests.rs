use super::execute;
use crate::command::{Command, ExpireCondition, GetExExpiration, SetCondition, SetExpiration};
use crate::output::CommandOutput as Response;
use crate::storage::InMemoryStore as Database;

#[test]
fn executes_type_touch_and_unlink() {
    let mut database = Database::new();
    database.set(b"string".to_vec(), b"value".to_vec());
    database.push_left(b"list", b"value".to_vec()).unwrap();
    database.set_add(b"set", b"value".to_vec()).unwrap();

    for (key, expected) in [
        (b"string".as_slice(), "string"),
        (b"list".as_slice(), "list"),
        (b"set".as_slice(), "set"),
        (b"missing".as_slice(), "none"),
    ] {
        assert_eq!(
            execute(Command::Type { key: key.to_vec() }, &mut database),
            Response::SimpleString(expected)
        );
    }
    assert_eq!(
        execute(
            Command::Touch {
                keys: vec![b"string".to_vec(), b"string".to_vec(), b"missing".to_vec()]
            },
            &mut database
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::Unlink {
                keys: vec![b"string".to_vec(), b"string".to_vec(), b"missing".to_vec()]
            },
            &mut database
        ),
        Response::Integer(1)
    );
}

#[test]
fn executes_keyspace_iteration_random_and_copy() {
    let mut database = Database::new();
    database.set(b"a:1".to_vec(), b"one".to_vec());
    database.set(b"b:1".to_vec(), b"two".to_vec());

    assert_eq!(
        execute(
            Command::Keys {
                pattern: b"a:*".to_vec()
            },
            &mut database
        ),
        Response::KeyList(vec![b"a:1".to_vec()])
    );
    assert_eq!(
        execute(
            Command::Scan {
                cursor: 0,
                pattern: None,
                count: 1,
                type_name: None
            },
            &mut database
        ),
        Response::Scan {
            cursor: 1,
            keys: vec![b"a:1".to_vec()]
        }
    );
    assert!(matches!(
        execute(Command::RandomKey, &mut database),
        Response::Value(key) if key == b"a:1" || key == b"b:1"
    ));
    assert_eq!(
        execute(
            Command::Copy {
                source: b"a:1".to_vec(),
                destination: b"copy".to_vec(),
                replace: false
            },
            &mut database
        ),
        Response::Integer(1)
    );
}

#[test]
fn save_reports_when_persistence_is_not_configured() {
    let mut database = Database::new();

    assert_eq!(
        execute(Command::Save, &mut database),
        Response::Error("snapshot path is not configured".to_owned())
    );
}

#[test]
fn aof_rewrite_reports_when_persistence_is_not_configured() {
    let mut database = Database::new();
    assert_eq!(
        execute(Command::AofRewrite, &mut database),
        Response::Error("AOF is not configured".to_owned())
    );
}

#[test]
fn execute_set_stores_value() {
    let mut database = Database::new();

    let response = execute(
        Command::Set {
            key: "name".to_owned().into(),
            value: "sample-value".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Ok);
    assert_eq!(database.get("name"), Ok(Some(b"sample-value".as_slice())));
}

#[test]
fn execute_set_nx_get_applies_only_to_a_missing_key() {
    let mut database = Database::new();
    let command = |value: &[u8]| Command::SetAdvanced {
        key: b"key".to_vec(),
        value: value.to_vec(),
        condition: Some(SetCondition::IfAbsent),
        return_old: true,
        expiration: None,
    };

    assert_eq!(execute(command(b"first"), &mut database), Response::Nil);
    assert_eq!(
        execute(command(b"second"), &mut database),
        Response::Value(b"first".to_vec())
    );
    assert_eq!(database.get(b"key"), Ok(Some(b"first".as_slice())));
}

#[test]
fn execute_set_xx_get_returns_old_value_and_replaces_it() {
    let mut database = Database::new();
    database.set(b"key".to_vec(), b"old".to_vec());

    let response = execute(
        Command::SetAdvanced {
            key: b"key".to_vec(),
            value: b"new".to_vec(),
            condition: Some(SetCondition::IfPresent),
            return_old: true,
            expiration: None,
        },
        &mut database,
    );

    assert_eq!(response, Response::Value(b"old".to_vec()));
    assert_eq!(database.get(b"key"), Ok(Some(b"new".as_slice())));
}

#[test]
fn execute_set_px_and_keepttl_manage_expiration() {
    let mut database = Database::new();
    assert_eq!(
        execute(
            Command::SetAdvanced {
                key: b"key".to_vec(),
                value: b"first".to_vec(),
                condition: None,
                return_old: false,
                expiration: Some(SetExpiration::Milliseconds(1_000)),
            },
            &mut database,
        ),
        Response::Ok
    );
    let ttl = database.pttl(b"key");
    assert!((1..=1_000).contains(&ttl));

    assert_eq!(
        execute(
            Command::SetAdvanced {
                key: b"key".to_vec(),
                value: b"second".to_vec(),
                condition: None,
                return_old: false,
                expiration: Some(SetExpiration::KeepTtl),
            },
            &mut database,
        ),
        Response::Ok
    );
    assert!((0..=ttl).contains(&database.pttl(b"key")));
}

#[test]
fn execute_set_get_rejects_wrong_type_without_mutation() {
    let mut database = Database::new();
    database.set_list(b"key".to_vec(), vec![b"item".to_vec()]);

    assert_eq!(
        execute(
            Command::SetAdvanced {
                key: b"key".to_vec(),
                value: b"replacement".to_vec(),
                condition: None,
                return_old: true,
                expiration: None,
            },
            &mut database,
        ),
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
    assert_eq!(
        database.list_values(b"key"),
        Ok(Some(vec![b"item".to_vec()]))
    );
}

#[test]
fn execute_getex_updates_and_removes_expiration() {
    let mut database = Database::new();
    database.set(b"key".to_vec(), b"value".to_vec());

    assert_eq!(
        execute(
            Command::GetEx {
                key: b"key".to_vec(),
                expiration: Some(GetExExpiration::Milliseconds(1_000)),
            },
            &mut database,
        ),
        Response::Value(b"value".to_vec())
    );
    assert!((1..=1_000).contains(&database.pttl(b"key")));
    assert_eq!(
        execute(
            Command::GetEx {
                key: b"key".to_vec(),
                expiration: Some(GetExExpiration::Persist),
            },
            &mut database,
        ),
        Response::Value(b"value".to_vec())
    );
    assert_eq!(database.pttl(b"key"), -1);
}

#[test]
fn execute_getex_handles_missing_and_wrong_type_without_mutation() {
    let mut database = Database::new();
    database.set_list(b"list".to_vec(), vec![b"item".to_vec()]);
    assert_eq!(
        execute(
            Command::GetEx {
                key: b"missing".to_vec(),
                expiration: Some(GetExExpiration::Seconds(1)),
            },
            &mut database,
        ),
        Response::Nil
    );
    assert_eq!(
        execute(
            Command::GetEx {
                key: b"list".to_vec(),
                expiration: Some(GetExExpiration::Seconds(1)),
            },
            &mut database,
        ),
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
    assert_eq!(
        database.list_values(b"list"),
        Ok(Some(vec![b"item".to_vec()]))
    );
}

#[test]
fn execute_msetnx_is_all_or_nothing() {
    let mut database = Database::new();
    database.set(b"existing".to_vec(), b"old".to_vec());
    assert_eq!(
        execute(
            Command::MSetNx {
                entries: vec![
                    (b"new".to_vec(), b"one".to_vec()),
                    (b"existing".to_vec(), b"replacement".to_vec()),
                ],
            },
            &mut database,
        ),
        Response::Integer(0)
    );
    assert_eq!(database.get(b"new"), Ok(None));
    assert_eq!(database.get(b"existing"), Ok(Some(b"old".as_slice())));

    assert_eq!(
        execute(
            Command::MSetNx {
                entries: vec![
                    (b"one".to_vec(), b"1".to_vec()),
                    (b"two".to_vec(), b"2".to_vec()),
                ],
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(database.get(b"one"), Ok(Some(b"1".as_slice())));
    assert_eq!(database.get(b"two"), Ok(Some(b"2".as_slice())));
}

#[test]
fn execute_expiration_conditions_compare_existing_deadlines() {
    let mut database = Database::new();
    database.set(b"key".to_vec(), b"value".to_vec());

    let expire = |seconds, condition| Command::ExpireAdvanced {
        key: b"key".to_vec(),
        seconds,
        condition,
    };
    assert_eq!(
        execute(expire(10, ExpireCondition::Greater), &mut database),
        Response::Integer(0)
    );
    assert_eq!(
        execute(expire(10, ExpireCondition::Less), &mut database),
        Response::Integer(1)
    );
    assert_eq!(
        execute(expire(20, ExpireCondition::NoExpiration), &mut database),
        Response::Integer(0)
    );
    assert_eq!(
        execute(expire(20, ExpireCondition::HasExpiration), &mut database),
        Response::Integer(1)
    );
    assert_eq!(
        execute(expire(10, ExpireCondition::Greater), &mut database),
        Response::Integer(0)
    );
    assert_eq!(
        execute(expire(30, ExpireCondition::Greater), &mut database),
        Response::Integer(1)
    );
}

#[test]
fn execute_absolute_expiration_and_expiretime_use_unix_time() {
    use std::time::SystemTime;

    let mut database = Database::new();
    database.set(b"key".to_vec(), b"value".to_vec());
    let target = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60;

    assert_eq!(
        execute(
            Command::ExpireAt {
                key: b"key".to_vec(),
                unix_seconds: target,
                condition: None,
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    let seconds = match execute(
        Command::ExpireTime {
            key: b"key".to_vec(),
        },
        &mut database,
    ) {
        Response::Integer(value) => value,
        response => panic!("unexpected response: {response:?}"),
    };
    assert!(((target - 1) as i64..=target as i64).contains(&seconds));
    let milliseconds = match execute(
        Command::PExpireTime {
            key: b"key".to_vec(),
        },
        &mut database,
    ) {
        Response::Integer(value) => value,
        response => panic!("unexpected response: {response:?}"),
    };
    assert!(((target * 1_000) as i64 - 20..=(target * 1_000) as i64).contains(&milliseconds));

    database.set(b"persistent".to_vec(), b"value".to_vec());
    assert_eq!(
        execute(
            Command::ExpireTime {
                key: b"persistent".to_vec(),
            },
            &mut database,
        ),
        Response::Integer(-1)
    );
    assert_eq!(
        execute(
            Command::ExpireTime {
                key: b"missing".to_vec(),
            },
            &mut database,
        ),
        Response::Integer(-2)
    );
}

#[test]
fn execute_get_returns_value() {
    let mut database = Database::new();
    database.set("name".to_owned().into(), "sample-value".to_owned().into());

    let response = execute(
        Command::Get {
            key: "name".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Value("sample-value".to_owned().into()));
}

#[test]
fn execute_get_missing_key_returns_nil() {
    let mut database = Database::new();

    let response = execute(
        Command::Get {
            key: "missing".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Nil);
}

#[test]
fn execute_delete_returns_one_for_existing_key() {
    let mut database = Database::new();
    database.set("name".to_owned().into(), "sample-value".to_owned().into());

    let response = execute(
        Command::Delete {
            keys: vec!["name".to_owned().into()],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(1));
    assert_eq!(database.get("name"), Ok(None));
}

#[test]
fn execute_delete_returns_zero_for_missing_key() {
    let mut database = Database::new();

    let response = execute(
        Command::Delete {
            keys: vec!["missing".to_owned().into()],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
}

#[test]
fn execute_mget_returns_values_in_requested_order() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "first-value".to_owned().into());
    database.set("city".to_owned().into(), "second-value".to_owned().into());

    let response = execute(
        Command::MGet {
            keys: vec![
                "name".to_owned().into(),
                "missing".to_owned().into(),
                "city".to_owned().into(),
            ],
        },
        &mut database,
    );

    assert_eq!(
        response,
        Response::OptionalValues(vec![
            Some("first-value".to_owned().into()),
            None,
            Some("second-value".to_owned().into()),
        ])
    );
}

#[test]
fn execute_setnx_inserts_missing_key() {
    let mut database = Database::new();

    let response = execute(
        Command::SetNx {
            key: "name".to_owned().into(),
            value: "initial-value".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(1));
    assert_eq!(database.get("name"), Ok(Some(b"initial-value".as_slice())));
}

#[test]
fn execute_setnx_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "initial-value".to_owned().into());

    let response = execute(
        Command::SetNx {
            key: "name".to_owned().into(),
            value: "replacement-value".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
    assert_eq!(database.get("name"), Ok(Some(b"initial-value".as_slice())));
}

#[test]
fn execute_increment_returns_new_value() {
    let mut database = Database::new();

    database.set("counter".to_owned().into(), "10".to_owned().into());

    let response = execute(
        Command::Increment {
            key: "counter".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(11));
    assert_eq!(database.get("counter"), Ok(Some(b"11".as_slice())));
}

#[test]
fn execute_increment_returns_error_for_non_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned().into(), "hello".to_owned().into());

    let response = execute(
        Command::Increment {
            key: "counter".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Error("value is not integer".to_owned()));

    assert_eq!(database.get("counter"), Ok(Some(b"hello".as_slice())));
}

#[test]
fn execute_increment_by_returns_overflow_error() {
    let mut database = Database::new();
    let max = i64::MAX.to_string();

    database.set("counter".to_owned().into(), max.clone().into());

    let response = execute(
        Command::IncrementBy {
            key: "counter".to_owned().into(),
            amount: 1,
        },
        &mut database,
    );

    assert_eq!(response, Response::Error("integer overflow".to_owned()));

    assert_eq!(database.get("counter"), Ok(Some(max.as_bytes())));
}

#[test]
fn execute_expire_returns_one_for_existing_key() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "value".to_owned().into());

    let response = execute(
        Command::Expire {
            key: "key".to_owned().into(),
            seconds: 60,
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(1));
}

#[test]
fn execute_expire_returns_zero_for_missing_key() {
    let mut database = Database::new();

    let response = execute(
        Command::Expire {
            key: "missing".to_owned().into(),
            seconds: 60,
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
}

#[test]
fn execute_ttl_returns_database_ttl() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "value".to_owned().into());

    let response = execute(
        Command::Ttl {
            key: "key".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(-1));
}

#[test]
fn execute_increment_by_float_returns_float() {
    let mut database = Database::new();

    database.set("counter".to_owned().into(), "10.5".to_owned().into());

    let response = execute(
        Command::IncrementByFloat {
            key: "counter".to_owned().into(),
            amount: 2.25,
        },
        &mut database,
    );

    assert_eq!(response, Response::Float(12.75));
}

#[test]
fn execute_increment_by_float_returns_error() {
    let mut database = Database::new();

    database.set("counter".to_owned().into(), "hello".to_owned().into());

    let response = execute(
        Command::IncrementByFloat {
            key: "counter".to_owned().into(),
            amount: 1.5,
        },
        &mut database,
    );

    assert_eq!(response, Response::Error("value is not float".to_owned()));
}

#[test]
fn execute_string_and_collection_commands() {
    let mut database = Database::new();

    assert_eq!(
        execute(
            Command::MSet {
                entries: vec![
                    ("first".to_owned().into(), "alpha".to_owned().into()),
                    ("second".to_owned().into(), "beta".to_owned().into()),
                ],
            },
            &mut database,
        ),
        Response::Ok
    );
    assert_eq!(execute(Command::Len, &mut database), Response::Integer(2));
    assert_eq!(
        execute(
            Command::Keys {
                pattern: b"*".to_vec(),
            },
            &mut database,
        ),
        Response::KeyList(vec!["first".to_owned().into(), "second".to_owned().into()])
    );
    assert_eq!(
        execute(
            Command::Append {
                key: "first".to_owned().into(),
                append_value: "-value".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(11)
    );
    assert_eq!(
        execute(
            Command::StrLen {
                key: "first".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(11)
    );
    assert_eq!(
        execute(
            Command::GetRange {
                key: "first".to_owned().into(),
                start: 6,
                end: 10,
            },
            &mut database,
        ),
        Response::Value("value".to_owned().into())
    );
    assert_eq!(
        execute(
            Command::SetRange {
                key: "second".to_owned().into(),
                offset: 0,
                value: "z".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(4)
    );
    assert_eq!(execute(Command::Clear, &mut database), Response::Ok);
    assert_eq!(execute(Command::Len, &mut database), Response::Integer(0));
}

#[test]
fn execute_atomic_value_commands() {
    let mut database = Database::new();
    database.set("key".to_owned().into(), "old-value".to_owned().into());

    assert_eq!(
        execute(
            Command::GetSet {
                key: "key".to_owned().into(),
                value: "new-value".to_owned().into(),
            },
            &mut database,
        ),
        Response::Value("old-value".to_owned().into())
    );
    assert_eq!(
        execute(
            Command::Rename {
                old_key: "key".to_owned().into(),
                new_key: "renamed".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::Exists {
                keys: vec!["renamed".to_owned().into()],
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::GetDel {
                key: "renamed".to_owned().into(),
            },
            &mut database,
        ),
        Response::Value("new-value".to_owned().into())
    );
    assert_eq!(database.len(), 0);
}

#[test]
fn execute_numeric_and_expiration_commands() {
    let mut database = Database::new();

    assert_eq!(
        execute(
            Command::Decrement {
                key: "counter".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(-1)
    );
    assert_eq!(
        execute(
            Command::DecrementBy {
                key: "counter".to_owned().into(),
                amount: 2,
            },
            &mut database,
        ),
        Response::Integer(-3)
    );
    assert_eq!(
        execute(
            Command::PExpire {
                key: "counter".to_owned().into(),
                milliseconds: 60_000,
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert!(matches!(
        execute(
            Command::PTtl {
                key: "counter".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(0..=60_000)
    ));
    assert_eq!(
        execute(
            Command::Persist {
                key: "counter".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
}

#[test]
fn execute_control_commands_return_control_responses() {
    let mut database = Database::new();

    assert_eq!(
        execute(Command::Ping { message: None }, &mut database),
        Response::Pong
    );
    assert_eq!(
        execute(
            Command::Ping {
                message: Some(b"message\0\xff".to_vec()),
            },
            &mut database,
        ),
        Response::Value(b"message\0\xff".to_vec())
    );
    assert_eq!(
        execute(
            Command::Echo {
                message: b"message\0\xff".to_vec(),
            },
            &mut database,
        ),
        Response::Value(b"message\0\xff".to_vec())
    );
    assert_eq!(
        execute(
            Command::Hello {
                protocol: Some(crate::command::ProtocolVersion::Resp3),
            },
            &mut database,
        ),
        Response::Hello {
            protocol: Some(crate::command::ProtocolVersion::Resp3),
            connection_id: None,
        }
    );
    assert_eq!(execute(Command::Help, &mut database), Response::Help);
    assert_eq!(execute(Command::Exit, &mut database), Response::Exit);
}

#[test]
fn execute_command_metadata_queries_use_the_shared_registry() {
    let mut database = Database::new();

    assert_eq!(
        execute(Command::MetadataCount, &mut database),
        Response::Integer(crate::command::COMMANDS.len() as i64)
    );
    assert_eq!(
        execute(
            Command::MetadataInfo {
                names: vec![b"GET".to_vec(), b"missing".to_vec()],
            },
            &mut database,
        ),
        Response::CommandMetadata(vec![crate::command::command_metadata(b"get"), None])
    );
    assert_eq!(
        execute(Command::MetadataInfo { names: Vec::new() }, &mut database),
        Response::CommandMetadata(crate::command::COMMANDS.iter().copied().map(Some).collect())
    );
}

#[test]
fn select_size_and_flush_commands_operate_on_the_single_database() {
    let mut database = Database::new();
    database.set(b"one".to_vec(), b"1".to_vec());
    database.set(b"two".to_vec(), b"2".to_vec());

    assert_eq!(execute(Command::Select, &mut database), Response::Ok);
    assert_eq!(
        execute(Command::DbSize, &mut database),
        Response::Integer(2)
    );
    assert_eq!(execute(Command::FlushDb, &mut database), Response::Ok);
    assert_eq!(
        execute(Command::DbSize, &mut database),
        Response::Integer(0)
    );

    database.set(b"again".to_vec(), b"value".to_vec());
    assert_eq!(execute(Command::FlushAll, &mut database), Response::Ok);
    assert_eq!(
        execute(Command::DbSize, &mut database),
        Response::Integer(0)
    );
}

#[test]
fn execute_delete_returns_number_of_deleted_keys() {
    let mut database = Database::new();

    database.set("a".to_owned().into(), "1".to_owned().into());
    database.set("b".to_owned().into(), "2".to_owned().into());

    let response = execute(
        Command::Delete {
            keys: vec![
                "a".to_owned().into(),
                "missing".to_owned().into(),
                "b".to_owned().into(),
            ],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(2));
}

#[test]
fn execute_exists_returns_number_of_existing_keys() {
    let mut database = Database::new();

    database.set("a".to_owned().into(), "1".to_owned().into());
    database.set("b".to_owned().into(), "2".to_owned().into());

    let response = execute(
        Command::Exists {
            keys: vec![
                "a".to_owned().into(),
                "missing".to_owned().into(),
                "b".to_owned().into(),
            ],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(2));
}

#[test]
fn execute_exists_counts_duplicates() {
    let mut database = Database::new();

    database.set("a".to_owned().into(), "1".to_owned().into());

    let response = execute(
        Command::Exists {
            keys: vec![
                "a".to_owned().into(),
                "a".to_owned().into(),
                "a".to_owned().into(),
            ],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(3));
}

#[test]
fn execute_string_commands_report_wrong_type() {
    let mut database = Database::new();
    database.set_list("list".to_owned().into(), vec!["value".to_owned().into()]);

    let get = execute(
        Command::Get {
            key: "list".to_owned().into(),
        },
        &mut database,
    );
    let append = execute(
        Command::Append {
            key: "list".to_owned().into(),
            append_value: "suffix".to_owned().into(),
        },
        &mut database,
    );
    let mget = execute(
        Command::MGet {
            keys: vec!["missing".to_owned().into(), "list".to_owned().into()],
        },
        &mut database,
    );

    let wrong_type =
        Response::Error("operation against a key holding the wrong kind of value".to_owned());
    assert_eq!(get, wrong_type);
    assert_eq!(append, wrong_type);
    assert_eq!(mget, wrong_type);
}

#[test]
fn execute_list_commands_return_lengths() {
    let mut database = Database::new();

    assert_eq!(
        execute(
            Command::LPush {
                key: "list".to_owned().into(),
                value: "middle".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::RPush {
                key: "list".to_owned().into(),
                value: "last".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::LLen {
                key: "list".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::LLen {
                key: "missing".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(0)
    );
}

#[test]
fn execute_list_commands_report_wrong_type() {
    let mut database = Database::new();
    database.set("key".to_owned().into(), "string".to_owned().into());

    let response = execute(
        Command::LPush {
            key: "key".to_owned().into(),
            value: "value".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(
        response,
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}

#[test]
fn execute_list_pop_commands_return_values_and_nil() {
    let mut database = Database::new();
    database.set_list(
        "list".to_owned().into(),
        vec!["first".to_owned().into(), "last".to_owned().into()],
    );

    assert_eq!(
        execute(
            Command::LPop {
                key: "list".to_owned().into(),
            },
            &mut database,
        ),
        Response::Value("first".to_owned().into())
    );
    assert_eq!(
        execute(
            Command::RPop {
                key: "list".to_owned().into(),
            },
            &mut database,
        ),
        Response::Value("last".to_owned().into())
    );
    assert_eq!(
        execute(
            Command::LPop {
                key: "list".to_owned().into(),
            },
            &mut database,
        ),
        Response::Nil
    );
}

#[test]
fn execute_list_pop_commands_report_wrong_type() {
    let mut database = Database::new();
    database.set("key".to_owned().into(), "string".to_owned().into());

    let response = execute(
        Command::RPop {
            key: "key".to_owned().into(),
        },
        &mut database,
    );

    assert_eq!(
        response,
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}

#[test]
fn execute_list_range_returns_values_nil_and_wrong_type() {
    let mut database = Database::new();
    database.set_list(
        "list".to_owned().into(),
        vec![
            "first".to_owned().into(),
            "second".to_owned().into(),
            "third".to_owned().into(),
        ],
    );
    database.set("string".to_owned().into(), "value".to_owned().into());

    assert_eq!(
        execute(
            Command::LRange {
                key: "list".to_owned().into(),
                start: 1,
                end: -1,
            },
            &mut database,
        ),
        Response::KeyList(vec!["second".to_owned().into(), "third".to_owned().into()])
    );
    assert_eq!(
        execute(
            Command::LRange {
                key: "missing".to_owned().into(),
                start: 0,
                end: -1,
            },
            &mut database,
        ),
        Response::KeyList(Vec::new())
    );
    assert_eq!(
        execute(
            Command::LRange {
                key: "string".to_owned().into(),
                start: 0,
                end: -1,
            },
            &mut database,
        ),
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}

#[test]
fn execute_set_collection_commands() {
    let mut database = Database::new();

    for (member, expected) in [("zeta", 1), ("alpha", 1), ("zeta", 0)] {
        assert_eq!(
            execute(
                Command::SAdd {
                    key: "set".to_owned().into(),
                    member: member.to_owned().into(),
                },
                &mut database,
            ),
            Response::Integer(expected)
        );
    }
    assert_eq!(
        execute(
            Command::SIsMember {
                key: "set".to_owned().into(),
                member: "alpha".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::SMembers {
                key: "set".to_owned().into()
            },
            &mut database
        ),
        Response::KeyList(vec!["alpha".to_owned().into(), "zeta".to_owned().into()])
    );
    assert_eq!(
        execute(
            Command::SCard {
                key: "set".to_owned().into()
            },
            &mut database
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::SRem {
                key: "set".to_owned().into(),
                member: "alpha".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
}

#[test]
fn execute_set_collection_commands_handle_missing_and_wrong_types() {
    let mut database = Database::new();
    database.set("string".to_owned().into(), "value".to_owned().into());

    assert_eq!(
        execute(
            Command::SIsMember {
                key: "missing".to_owned().into(),
                member: "member".to_owned().into(),
            },
            &mut database,
        ),
        Response::Integer(0)
    );
    assert_eq!(
        execute(
            Command::SMembers {
                key: "missing".to_owned().into()
            },
            &mut database
        ),
        Response::KeyList(Vec::new())
    );
    assert_eq!(
        execute(
            Command::SCard {
                key: "missing".to_owned().into()
            },
            &mut database
        ),
        Response::Integer(0)
    );
    assert_eq!(
        execute(
            Command::SAdd {
                key: "string".to_owned().into(),
                member: "member".to_owned().into(),
            },
            &mut database,
        ),
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}

#[test]
fn execute_hash_commands() {
    let mut database = Database::new();
    assert_eq!(
        execute(
            Command::HSet {
                key: b"record".to_vec(),
                entries: vec![
                    (b"z".to_vec(), b"1".to_vec()),
                    (b"a".to_vec(), b"2".to_vec())
                ]
            },
            &mut database
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::HGetAll {
                key: b"record".to_vec()
            },
            &mut database
        ),
        Response::HashEntries(vec![
            (b"a".to_vec(), b"2".to_vec()),
            (b"z".to_vec(), b"1".to_vec())
        ])
    );
    assert_eq!(
        execute(
            Command::HMGet {
                key: b"record".to_vec(),
                fields: vec![b"z".to_vec(), b"missing".to_vec()]
            },
            &mut database
        ),
        Response::OptionalValues(vec![Some(b"1".to_vec()), None])
    );
    assert_eq!(
        execute(
            Command::HDel {
                key: b"record".to_vec(),
                fields: vec![b"z".to_vec()]
            },
            &mut database
        ),
        Response::Integer(1)
    );
}

#[test]
fn execute_counted_list_pops_return_arrays() {
    let mut database = Database::new();
    database.set_list(
        b"list".to_vec(),
        vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()],
    );
    assert_eq!(
        execute(
            Command::LPopCount {
                key: b"list".to_vec(),
                count: 2,
            },
            &mut database,
        ),
        Response::KeyList(vec![b"one".to_vec(), b"two".to_vec()])
    );
    assert_eq!(
        execute(
            Command::RPopCount {
                key: b"missing".to_vec(),
                count: 2,
            },
            &mut database,
        ),
        Response::KeyList(Vec::new())
    );
}

#[test]
fn execute_conditional_pushes_return_lengths() {
    let mut database = Database::new();
    assert_eq!(
        execute(
            Command::LPushX {
                key: b"missing".to_vec(),
                values: vec![b"one".to_vec()],
            },
            &mut database,
        ),
        Response::Integer(0)
    );
    database.set_list(b"list".to_vec(), vec![b"one".to_vec()]);
    assert_eq!(
        execute(
            Command::RPushX {
                key: b"list".to_vec(),
                values: vec![b"two".to_vec(), b"three".to_vec()],
            },
            &mut database,
        ),
        Response::Integer(3)
    );
}

#[test]
fn execute_list_index_and_set_map_values_and_errors() {
    let mut database = Database::new();
    database.set_list(b"list".to_vec(), vec![b"one".to_vec(), b"two".to_vec()]);
    assert_eq!(
        execute(
            Command::LIndex {
                key: b"list".to_vec(),
                index: -1,
            },
            &mut database,
        ),
        Response::Value(b"two".to_vec())
    );
    assert_eq!(
        execute(
            Command::LSet {
                key: b"list".to_vec(),
                index: 0,
                value: b"changed".to_vec(),
            },
            &mut database,
        ),
        Response::Ok
    );
    assert_eq!(
        execute(
            Command::LSet {
                key: b"missing".to_vec(),
                index: 0,
                value: b"changed".to_vec(),
            },
            &mut database,
        ),
        Response::Error("no such key".to_owned())
    );
}

#[test]
fn execute_extended_list_commands_map_results() {
    let mut database = Database::new();
    database.set_list(
        b"source".to_vec(),
        vec![b"a".to_vec(), b"b".to_vec(), b"a".to_vec()],
    );
    assert_eq!(
        execute(
            Command::LInsert {
                key: b"source".to_vec(),
                position: crate::command::InsertPosition::After,
                pivot: b"b".to_vec(),
                value: b"x".to_vec()
            },
            &mut database
        ),
        Response::Integer(4)
    );
    assert_eq!(
        execute(
            Command::LPos {
                key: b"source".to_vec(),
                value: b"a".to_vec(),
                rank: 1,
                count: Some(0),
                max_len: None
            },
            &mut database
        ),
        Response::IntegerList(vec![0, 3])
    );
    assert_eq!(
        execute(
            Command::LRem {
                key: b"source".to_vec(),
                count: 1,
                value: b"a".to_vec()
            },
            &mut database
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::LTrim {
                key: b"source".to_vec(),
                start: 0,
                end: 1
            },
            &mut database
        ),
        Response::Ok
    );
    assert_eq!(
        execute(
            Command::LMove {
                source: b"source".to_vec(),
                destination: b"destination".to_vec(),
                source_end: crate::command::ListEnd::Right,
                destination_end: crate::command::ListEnd::Left
            },
            &mut database
        ),
        Response::Value(b"x".to_vec())
    );
    assert_eq!(
        execute(
            Command::RPopLPush {
                source: b"destination".to_vec(),
                destination: b"source".to_vec()
            },
            &mut database
        ),
        Response::Value(b"x".to_vec())
    );
}

#[test]
fn execute_set_membership_and_selection_commands() {
    let mut database = Database::new();
    database.set_set(b"set".to_vec(), vec![b"a".to_vec(), b"b".to_vec()]);
    assert_eq!(
        execute(
            Command::SMIsMember {
                key: b"set".to_vec(),
                members: vec![b"b".to_vec(), b"x".to_vec()]
            },
            &mut database
        ),
        Response::IntegerList(vec![1, 0])
    );
    assert_eq!(
        execute(
            Command::SPop {
                key: b"set".to_vec(),
                count: None
            },
            &mut database
        ),
        Response::Value(b"a".to_vec())
    );
    assert_eq!(
        execute(
            Command::SRandMember {
                key: b"set".to_vec(),
                count: Some(-3)
            },
            &mut database
        ),
        Response::KeyList(vec![b"b".to_vec(), b"b".to_vec(), b"b".to_vec()])
    );
    assert_eq!(database.set_cardinality("set"), Ok(1));
}
