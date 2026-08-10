use super::execute;
use crate::command::Command;
use crate::output::CommandOutput as Response;
use crate::storage::InMemoryStore as Database;

#[test]
fn execute_set_stores_value() {
    let mut database = Database::new();

    let response = execute(
        Command::Set {
            key: "name".to_owned(),
            value: "sample-value".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Ok);
    assert_eq!(database.get("name"), Ok(Some("sample-value")));
}

#[test]
fn execute_get_returns_value() {
    let mut database = Database::new();
    database.set("name".to_owned(), "sample-value".to_owned());

    let response = execute(
        Command::Get {
            key: "name".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Value("sample-value".to_owned()));
}

#[test]
fn execute_get_missing_key_returns_nil() {
    let mut database = Database::new();

    let response = execute(
        Command::Get {
            key: "missing".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Nil);
}

#[test]
fn execute_delete_returns_one_for_existing_key() {
    let mut database = Database::new();
    database.set("name".to_owned(), "sample-value".to_owned());

    let response = execute(
        Command::Delete {
            keys: vec!["name".to_owned()],
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
            keys: vec!["missing".to_owned()],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
}

#[test]
fn execute_mget_returns_values_in_requested_order() {
    let mut database = Database::new();

    database.set("name".to_owned(), "first-value".to_owned());
    database.set("city".to_owned(), "second-value".to_owned());

    let response = execute(
        Command::MGet {
            keys: vec!["name".to_owned(), "missing".to_owned(), "city".to_owned()],
        },
        &mut database,
    );

    assert_eq!(
        response,
        Response::OptionalValues(vec![
            Some("first-value".to_owned()),
            None,
            Some("second-value".to_owned()),
        ])
    );
}

#[test]
fn execute_setnx_inserts_missing_key() {
    let mut database = Database::new();

    let response = execute(
        Command::SetNx {
            key: "name".to_owned(),
            value: "initial-value".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(1));
    assert_eq!(database.get("name"), Ok(Some("initial-value")));
}

#[test]
fn execute_setnx_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());

    let response = execute(
        Command::SetNx {
            key: "name".to_owned(),
            value: "replacement-value".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
    assert_eq!(database.get("name"), Ok(Some("initial-value")));
}

#[test]
fn execute_increment_returns_new_value() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());

    let response = execute(
        Command::Increment {
            key: "counter".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(11));
    assert_eq!(database.get("counter"), Ok(Some("11")));
}

#[test]
fn execute_increment_returns_error_for_non_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "hello".to_owned());

    let response = execute(
        Command::Increment {
            key: "counter".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Error("value is not integer".to_owned()));

    assert_eq!(database.get("counter"), Ok(Some("hello")));
}

#[test]
fn execute_increment_by_returns_overflow_error() {
    let mut database = Database::new();
    let max = i64::MAX.to_string();

    database.set("counter".to_owned(), max.clone());

    let response = execute(
        Command::IncrementBy {
            key: "counter".to_owned(),
            amount: 1,
        },
        &mut database,
    );

    assert_eq!(response, Response::Error("integer overflow".to_owned()));

    assert_eq!(database.get("counter"), Ok(Some(max.as_str())));
}

#[test]
fn execute_expire_returns_one_for_existing_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    let response = execute(
        Command::Expire {
            key: "key".to_owned(),
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
            key: "missing".to_owned(),
            seconds: 60,
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
}

#[test]
fn execute_ttl_returns_database_ttl() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    let response = execute(
        Command::Ttl {
            key: "key".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(-1));
}

#[test]
fn execute_increment_by_float_returns_float() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10.5".to_owned());

    let response = execute(
        Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: 2.25,
        },
        &mut database,
    );

    assert_eq!(response, Response::Float(12.75));
}

#[test]
fn execute_increment_by_float_returns_error() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "hello".to_owned());

    let response = execute(
        Command::IncrementByFloat {
            key: "counter".to_owned(),
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
                    ("first".to_owned(), "alpha".to_owned()),
                    ("second".to_owned(), "beta".to_owned()),
                ],
            },
            &mut database,
        ),
        Response::Ok
    );
    assert_eq!(execute(Command::Len, &mut database), Response::Integer(2));
    assert_eq!(
        execute(Command::Keys, &mut database),
        Response::KeyList(vec!["first".to_owned(), "second".to_owned()])
    );
    assert_eq!(
        execute(
            Command::Append {
                key: "first".to_owned(),
                append_value: "-value".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(11)
    );
    assert_eq!(
        execute(
            Command::StrLen {
                key: "first".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(11)
    );
    assert_eq!(
        execute(
            Command::GetRange {
                key: "first".to_owned(),
                start: 6,
                end: 10,
            },
            &mut database,
        ),
        Response::Value("value".to_owned())
    );
    assert_eq!(
        execute(
            Command::SetRange {
                key: "second".to_owned(),
                offset: 0,
                value: "z".to_owned(),
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
    database.set("key".to_owned(), "old-value".to_owned());

    assert_eq!(
        execute(
            Command::GetSet {
                key: "key".to_owned(),
                value: "new-value".to_owned(),
            },
            &mut database,
        ),
        Response::Value("old-value".to_owned())
    );
    assert_eq!(
        execute(
            Command::Rename {
                old_key: "key".to_owned(),
                new_key: "renamed".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::Exists {
                keys: vec!["renamed".to_owned()],
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::GetDel {
                key: "renamed".to_owned(),
            },
            &mut database,
        ),
        Response::Value("new-value".to_owned())
    );
    assert_eq!(database.len(), 0);
}

#[test]
fn execute_numeric_and_expiration_commands() {
    let mut database = Database::new();

    assert_eq!(
        execute(
            Command::Decrement {
                key: "counter".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(-1)
    );
    assert_eq!(
        execute(
            Command::DecrementBy {
                key: "counter".to_owned(),
                amount: 2,
            },
            &mut database,
        ),
        Response::Integer(-3)
    );
    assert_eq!(
        execute(
            Command::PExpire {
                key: "counter".to_owned(),
                milliseconds: 60_000,
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert!(matches!(
        execute(
            Command::PTtl {
                key: "counter".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(0..=60_000)
    ));
    assert_eq!(
        execute(
            Command::Persist {
                key: "counter".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
}

#[test]
fn execute_control_commands_return_control_responses() {
    let mut database = Database::new();

    assert_eq!(execute(Command::Help, &mut database), Response::Help);
    assert_eq!(execute(Command::Exit, &mut database), Response::Exit);
}

#[test]
fn execute_delete_returns_number_of_deleted_keys() {
    let mut database = Database::new();

    database.set("a".to_owned(), "1".to_owned());
    database.set("b".to_owned(), "2".to_owned());

    let response = execute(
        Command::Delete {
            keys: vec!["a".to_owned(), "missing".to_owned(), "b".to_owned()],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(2));
}

#[test]
fn execute_exists_returns_number_of_existing_keys() {
    let mut database = Database::new();

    database.set("a".to_owned(), "1".to_owned());
    database.set("b".to_owned(), "2".to_owned());

    let response = execute(
        Command::Exists {
            keys: vec!["a".to_owned(), "missing".to_owned(), "b".to_owned()],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(2));
}

#[test]
fn execute_exists_counts_duplicates() {
    let mut database = Database::new();

    database.set("a".to_owned(), "1".to_owned());

    let response = execute(
        Command::Exists {
            keys: vec!["a".to_owned(), "a".to_owned(), "a".to_owned()],
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(3));
}

#[test]
fn execute_string_commands_report_wrong_type() {
    let mut database = Database::new();
    database.set_list("list".to_owned(), vec!["value".to_owned()]);

    let get = execute(
        Command::Get {
            key: "list".to_owned(),
        },
        &mut database,
    );
    let append = execute(
        Command::Append {
            key: "list".to_owned(),
            append_value: "suffix".to_owned(),
        },
        &mut database,
    );
    let mget = execute(
        Command::MGet {
            keys: vec!["missing".to_owned(), "list".to_owned()],
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
                key: "list".to_owned(),
                value: "middle".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::RPush {
                key: "list".to_owned(),
                value: "last".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::LLen {
                key: "list".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::LLen {
                key: "missing".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(0)
    );
}

#[test]
fn execute_list_commands_report_wrong_type() {
    let mut database = Database::new();
    database.set("key".to_owned(), "string".to_owned());

    let response = execute(
        Command::LPush {
            key: "key".to_owned(),
            value: "value".to_owned(),
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
        "list".to_owned(),
        vec!["first".to_owned(), "last".to_owned()],
    );

    assert_eq!(
        execute(
            Command::LPop {
                key: "list".to_owned(),
            },
            &mut database,
        ),
        Response::Value("first".to_owned())
    );
    assert_eq!(
        execute(
            Command::RPop {
                key: "list".to_owned(),
            },
            &mut database,
        ),
        Response::Value("last".to_owned())
    );
    assert_eq!(
        execute(
            Command::LPop {
                key: "list".to_owned(),
            },
            &mut database,
        ),
        Response::Nil
    );
}

#[test]
fn execute_list_pop_commands_report_wrong_type() {
    let mut database = Database::new();
    database.set("key".to_owned(), "string".to_owned());

    let response = execute(
        Command::RPop {
            key: "key".to_owned(),
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
        "list".to_owned(),
        vec!["first".to_owned(), "second".to_owned(), "third".to_owned()],
    );
    database.set("string".to_owned(), "value".to_owned());

    assert_eq!(
        execute(
            Command::LRange {
                key: "list".to_owned(),
                start: 1,
                end: -1,
            },
            &mut database,
        ),
        Response::KeyList(vec!["second".to_owned(), "third".to_owned()])
    );
    assert_eq!(
        execute(
            Command::LRange {
                key: "missing".to_owned(),
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
                key: "string".to_owned(),
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
                    key: "set".to_owned(),
                    member: member.to_owned(),
                },
                &mut database,
            ),
            Response::Integer(expected)
        );
    }
    assert_eq!(
        execute(
            Command::SIsMember {
                key: "set".to_owned(),
                member: "alpha".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
    assert_eq!(
        execute(
            Command::SMembers {
                key: "set".to_owned()
            },
            &mut database
        ),
        Response::KeyList(vec!["alpha".to_owned(), "zeta".to_owned()])
    );
    assert_eq!(
        execute(
            Command::SCard {
                key: "set".to_owned()
            },
            &mut database
        ),
        Response::Integer(2)
    );
    assert_eq!(
        execute(
            Command::SRem {
                key: "set".to_owned(),
                member: "alpha".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(1)
    );
}

#[test]
fn execute_set_collection_commands_handle_missing_and_wrong_types() {
    let mut database = Database::new();
    database.set("string".to_owned(), "value".to_owned());

    assert_eq!(
        execute(
            Command::SIsMember {
                key: "missing".to_owned(),
                member: "member".to_owned(),
            },
            &mut database,
        ),
        Response::Integer(0)
    );
    assert_eq!(
        execute(
            Command::SMembers {
                key: "missing".to_owned()
            },
            &mut database
        ),
        Response::KeyList(Vec::new())
    );
    assert_eq!(
        execute(
            Command::SCard {
                key: "missing".to_owned()
            },
            &mut database
        ),
        Response::Integer(0)
    );
    assert_eq!(
        execute(
            Command::SAdd {
                key: "string".to_owned(),
                member: "member".to_owned(),
            },
            &mut database,
        ),
        Response::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}
