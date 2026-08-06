use super::execute;
use crate::command::Command;
use crate::database::Database;
use crate::response::Response;

#[test]
fn execute_set_stores_value() {
    let mut database = Database::new();

    let response = execute(
        Command::Set {
            key: "name".to_owned(),
            value: "Ivan".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Ok);
    assert_eq!(database.get("name"), Some("Ivan"));
}

#[test]
fn execute_get_returns_value() {
    let mut database = Database::new();
    database.set("name".to_owned(), "Ivan".to_owned());

    let response = execute(
        Command::Get {
            key: "name".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Value("Ivan".to_owned()));
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
    database.set("name".to_owned(), "Ivan".to_owned());

    let response = execute(
        Command::Delete {
            key: "name".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(1));
    assert_eq!(database.get("name"), None);
}

#[test]
fn execute_delete_returns_zero_for_missing_key() {
    let mut database = Database::new();

    let response = execute(
        Command::Delete {
            key: "missing".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
}

#[test]
fn execute_mget_returns_values_in_requested_order() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("city".to_owned(), "Berlin".to_owned());

    let response = execute(
        Command::MGet {
            keys: vec!["name".to_owned(), "missing".to_owned(), "city".to_owned()],
        },
        &mut database,
    );

    assert_eq!(
        response,
        Response::Values(vec![
            Some("Ivan".to_owned()),
            None,
            Some("Berlin".to_owned()),
        ])
    );
}

#[test]
fn execute_setnx_inserts_missing_key() {
    let mut database = Database::new();

    let response = execute(
        Command::SetNx {
            key: "name".to_owned(),
            value: "Ivan".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(1));
    assert_eq!(database.get("name"), Some("Ivan"));
}

#[test]
fn execute_setnx_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    let response = execute(
        Command::SetNx {
            key: "name".to_owned(),
            value: "Alex".to_owned(),
        },
        &mut database,
    );

    assert_eq!(response, Response::Integer(0));
    assert_eq!(database.get("name"), Some("Ivan"));
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
    assert_eq!(database.get("counter"), Some("11"));
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

    assert_eq!(database.get("counter"), Some("hello"));
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

    assert_eq!(database.get("counter"), Some(max.as_str()));
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
