use super::super::in_memory::{InMemoryStore as Database, StoreError};
use std::time::{Duration, Instant};

fn list_with_expiration() -> (Database, Instant) {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set_list("key".to_owned(), vec!["value".to_owned()]);
    assert!(database.expire_at("key", expires_at));

    (database, expires_at)
}

fn assert_list_and_expiration_are_unchanged(database: &mut Database, expires_at: Instant) {
    assert_eq!(database.get("key"), Err(StoreError::WrongType));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn string_reads_reject_lists() {
    let (mut database, expires_at) = list_with_expiration();

    assert_eq!(database.get("key"), Err(StoreError::WrongType));
    assert_eq!(database.string_length("key"), Err(StoreError::WrongType));
    assert_eq!(database.get_range("key", 0, 1), Err(StoreError::WrongType));
    assert_list_and_expiration_are_unchanged(&mut database, expires_at);
}

#[test]
fn string_mutations_reject_lists_without_changing_value_or_ttl() {
    let (mut database, expires_at) = list_with_expiration();

    assert_eq!(
        database.append("key", "suffix".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.set_range("key".to_owned(), 0, "replacement".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.get_and_set("key".to_owned(), "replacement".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.get_and_delete("key".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_list_and_expiration_are_unchanged(&mut database, expires_at);
}

#[test]
fn numeric_mutations_reject_lists_without_changing_value_or_ttl() {
    let (mut database, expires_at) = list_with_expiration();

    assert_eq!(
        database.increment("key".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.increment_by("key".to_owned(), 2),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.decrement("key".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.decrement_by("key".to_owned(), 2),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.increment_by_float("key".to_owned(), 1.5),
        Err(StoreError::WrongType)
    );
    assert_list_and_expiration_are_unchanged(&mut database, expires_at);
}

#[test]
fn string_and_list_commands_reject_sets_without_changing_value_or_ttl() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set_set("key".to_owned(), vec!["member".to_owned()]);
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.get("key"), Err(StoreError::WrongType));
    assert_eq!(
        database.append("key", "value".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.push_left("key", "value".to_owned()),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.list_length("key"), Err(StoreError::WrongType));

    assert_eq!(database.set_members("key"), Ok(vec!["member".to_owned()]));
    assert_eq!(database.expiration("key"), Some(expires_at));
}
