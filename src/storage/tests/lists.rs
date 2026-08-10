use super::super::in_memory::{InMemoryStore as Database, StoreError};
use std::time::{Duration, Instant};

#[test]
fn push_left_creates_a_list_and_prepends_values() {
    let mut database = Database::new();

    assert_eq!(database.push_left("key", "one".to_owned()), Ok(1));
    assert_eq!(database.push_left("key", "two".to_owned()), Ok(2));

    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["two".to_owned(), "one".to_owned()]))
    );
}

#[test]
fn push_right_creates_a_list_and_appends_values() {
    let mut database = Database::new();

    assert_eq!(database.push_right("key", "one".to_owned()), Ok(1));
    assert_eq!(database.push_right("key", "two".to_owned()), Ok(2));

    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["one".to_owned(), "two".to_owned()]))
    );
}

#[test]
fn pushes_at_opposite_ends_preserve_list_order() {
    let mut database = Database::new();

    database.push_right("key", "middle".to_owned()).unwrap();
    database.push_left("key", "first".to_owned()).unwrap();
    database.push_right("key", "last".to_owned()).unwrap();

    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec![
            "first".to_owned(),
            "middle".to_owned(),
            "last".to_owned(),
        ]))
    );
}

#[test]
fn list_length_handles_existing_and_missing_lists() {
    let mut database = Database::new();

    assert_eq!(database.list_length("missing"), Ok(0));

    database.push_right("key", "one".to_owned()).unwrap();
    database.push_right("key", "two".to_owned()).unwrap();

    assert_eq!(database.list_length("key"), Ok(2));
}

#[test]
fn list_commands_reject_strings_without_mutating_them() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set("key".to_owned(), "value".to_owned());
    assert!(database.expire_at("key", expires_at));

    assert_eq!(
        database.push_left("key", "left".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.push_right("key", "right".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.list_length("key"), Err(StoreError::WrongType));

    assert_eq!(database.get("key"), Ok(Some("value")));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn pushing_to_an_existing_list_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.push_right("key", "one".to_owned()).unwrap();
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.push_left("key", "zero".to_owned()), Ok(2));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn list_commands_treat_expired_keys_as_missing() {
    let mut database = Database::new();
    database.push_right("key", "old".to_owned()).unwrap();
    assert!(database.expire("key", 0));

    assert_eq!(database.list_length("key"), Ok(0));
    assert_eq!(database.push_left("key", "new".to_owned()), Ok(1));
    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["new".to_owned()]))
    );
    assert_eq!(database.ttl("key"), -1);
}

#[test]
fn pop_left_and_right_remove_values_from_opposite_ends() {
    let mut database = Database::new();
    database.set_list(
        "key".to_owned(),
        vec!["first".to_owned(), "middle".to_owned(), "last".to_owned()],
    );

    assert_eq!(database.pop_left("key"), Ok(Some("first".to_owned())));
    assert_eq!(database.pop_right("key"), Ok(Some("last".to_owned())));
    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["middle".to_owned()]))
    );
}

#[test]
fn pop_returns_none_for_missing_and_expired_keys() {
    let mut database = Database::new();

    assert_eq!(database.pop_left("missing"), Ok(None));
    assert_eq!(database.pop_right("missing"), Ok(None));

    database.set_list("expired".to_owned(), vec!["value".to_owned()]);
    assert!(database.expire("expired", 0));

    assert_eq!(database.pop_left("expired"), Ok(None));
    assert!(!database.exists("expired"));
}

#[test]
fn pop_rejects_strings_without_changing_value_or_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set("key".to_owned(), "value".to_owned());
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.pop_left("key"), Err(StoreError::WrongType));
    assert_eq!(database.pop_right("key"), Err(StoreError::WrongType));
    assert_eq!(database.get("key"), Ok(Some("value")));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn pop_preserves_expiration_while_the_list_is_not_empty() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set_list(
        "key".to_owned(),
        vec!["first".to_owned(), "last".to_owned()],
    );
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.pop_left("key"), Ok(Some("first".to_owned())));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn pop_removes_the_key_after_the_last_value() {
    let mut database = Database::new();
    database.set_list("key".to_owned(), vec!["only".to_owned()]);
    assert!(database.expire("key", 60));

    assert_eq!(database.pop_right("key"), Ok(Some("only".to_owned())));
    assert!(!database.exists("key"));
    assert_eq!(database.ttl("key"), -2);
}
