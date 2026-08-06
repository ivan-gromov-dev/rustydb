use crate::database::entry::Entry;
use std::time::{Duration, Instant};

use super::memory::{Database, DatabaseError};

#[test]
fn set_and_get_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    assert_eq!(database.get("name"), Some("Ivan"));
}

#[test]
fn set_overwrites_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("name".to_owned(), "Alex".to_owned());

    assert_eq!(database.get("name"), Some("Alex"));
    assert_eq!(database.len(), 1);
}

#[test]
fn delete_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    assert!(database.delete("name"));
    assert!(!database.exists("name"));
}

#[test]
fn clear() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("surname".to_owned(), "Gromov".to_owned());

    database.clear();

    assert_eq!(database.len(), 0);
}

#[test]
fn get_keys() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("surname".to_owned(), "Gromov".to_owned());

    assert_eq!(database.keys(), ["name", "surname"])
}

#[test]
fn delete_missing_value_returns_false() {
    let mut database = Database::new();

    assert!(!database.delete("missing"));
}

#[test]
fn rename_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Gromov".to_owned());

    assert!(database.rename_key("name", "surname".to_owned()))
}

#[test]
fn append() {
    let mut database = Database::new();

    database.set("message".to_owned(), "Hello".to_owned());

    let length = database.append("message", ", world".to_owned());

    assert_eq!(length, 12);
    assert_eq!(database.get("message"), Some("Hello, world"));
}

#[test]
fn increment_missing_key_creates_value_one() {
    let mut database = Database::new();

    let result = database.increment("counter".to_owned());

    assert_eq!(result, Ok(1));
    assert_eq!(database.get("counter"), Some("1"));
}

#[test]
fn increment_existing_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());

    let result = database.increment("counter".to_owned());

    assert_eq!(result, Ok(11));
    assert_eq!(database.get("counter"), Some("11"));
}

#[test]
fn increment_multiple_times() {
    let mut database = Database::new();

    assert_eq!(database.increment("counter".to_owned()), Ok(1));
    assert_eq!(database.increment("counter".to_owned()), Ok(2));
    assert_eq!(database.increment("counter".to_owned()), Ok(3));

    assert_eq!(database.get("counter"), Some("3"));
}

#[test]
fn increment_negative_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "-2".to_owned());

    let result = database.increment("counter".to_owned());

    assert_eq!(result, Ok(-1));
    assert_eq!(database.get("counter"), Some("-1"));
}

#[test]
fn increment_non_integer_returns_error() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "hello".to_owned());

    let result = database.increment("counter".to_owned());

    assert_eq!(result, Err(DatabaseError::ValueIsNotInteger));
    assert_eq!(database.get("counter"), Some("hello"));
}
#[test]
fn decrement_missing_key_creates_minus_one() {
    let mut database = Database::new();

    let result = database.decrement("counter".to_owned());

    assert_eq!(result, Ok(-1));
    assert_eq!(database.get("counter"), Some("-1"));
}

#[test]
fn decrement_existing_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());

    let result = database.decrement("counter".to_owned());

    assert_eq!(result, Ok(9));
    assert_eq!(database.get("counter"), Some("9"));
}

#[test]
fn decrement_multiple_times() {
    let mut database = Database::new();

    assert_eq!(database.decrement("counter".to_owned()), Ok(-1));
    assert_eq!(database.decrement("counter".to_owned()), Ok(-2));
    assert_eq!(database.decrement("counter".to_owned()), Ok(-3));

    assert_eq!(database.get("counter"), Some("-3"));
}

#[test]
fn decrement_negative_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "-5".to_owned());

    let result = database.decrement("counter".to_owned());

    assert_eq!(result, Ok(-6));
    assert_eq!(database.get("counter"), Some("-6"));
}

#[test]
fn decrement_non_integer_returns_error() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "hello".to_owned());

    let result = database.decrement("counter".to_owned());

    assert_eq!(result, Err(DatabaseError::ValueIsNotInteger));
    assert_eq!(database.get("counter"), Some("hello"));
}

#[test]
fn decrement_by_missing_key_uses_zero() {
    let mut database = Database::new();

    let result = database.decrement_by("counter".to_owned(), 5);

    assert_eq!(result, Ok(-5));
    assert_eq!(database.get("counter"), Some("-5"));
}

#[test]
fn decrement_by_existing_integer() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());

    let result = database.decrement_by("counter".to_owned(), 4);

    assert_eq!(result, Ok(6));
    assert_eq!(database.get("counter"), Some("6"));
}

#[test]
fn decrement_by_negative_amount_increments_value() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());

    let result = database.decrement_by("counter".to_owned(), -5);

    assert_eq!(result, Ok(15));
    assert_eq!(database.get("counter"), Some("15"));
}

#[test]
fn decrement_by_non_integer_returns_error() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "hello".to_owned());

    let result = database.decrement_by("counter".to_owned(), 5);

    assert_eq!(result, Err(DatabaseError::ValueIsNotInteger));
    assert_eq!(database.get("counter"), Some("hello"));
}

#[test]
fn decrement_by_detects_overflow() {
    let mut database = Database::new();
    let min = i64::MIN.to_string();

    database.set("counter".to_owned(), min.clone());

    let result = database.decrement_by("counter".to_owned(), 1);

    assert_eq!(result, Err(DatabaseError::IntegerOverflow));
    assert_eq!(database.get("counter"), Some(min.as_str()));
}

#[test]
fn decrement_by_detects_overflow_with_negative_amount() {
    let mut database = Database::new();
    let max = i64::MAX.to_string();

    database.set("counter".to_owned(), max.clone());

    let result = database.decrement_by("counter".to_owned(), -1);

    assert_eq!(result, Err(DatabaseError::IntegerOverflow));
    assert_eq!(database.get("counter"), Some(max.as_str()));
}
#[test]
fn set_if_absent_inserts_missing_key() {
    let mut database = Database::new();

    let inserted = database.set_if_absent("name".to_owned(), "Ivan".to_owned());

    assert!(inserted);
    assert_eq!(database.get("name"), Some("Ivan"));
}

#[test]
fn set_if_absent_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    let inserted = database.set_if_absent("name".to_owned(), "Alex".to_owned());

    assert!(!inserted);
    assert_eq!(database.get("name"), Some("Ivan"));
}

#[test]
fn set_if_absent_increases_length_only_once() {
    let mut database = Database::new();

    assert!(database.set_if_absent("name".to_owned(), "Ivan".to_owned(),));

    assert!(!database.set_if_absent("name".to_owned(), "Alex".to_owned(),));

    assert_eq!(database.len(), 1);
}
#[test]
fn get_and_delete_returns_existing_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    let result = database.get_and_delete("name".to_owned());

    assert_eq!(result, Some("Ivan".to_owned()));
}

#[test]
fn get_and_delete_removes_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    database.get_and_delete("name".to_owned());

    assert_eq!(database.get("name"), None);
    assert!(!database.exists("name"));
}

#[test]
fn get_and_delete_missing_key_returns_none() {
    let mut database = Database::new();

    let result = database.get_and_delete("missing".to_owned());

    assert_eq!(result, None);
}

#[test]
fn get_and_delete_decreases_database_length() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("city".to_owned(), "Berlin".to_owned());

    let result = database.get_and_delete("name".to_owned());

    assert_eq!(result, Some("Ivan".to_owned()));
    assert_eq!(database.len(), 1);
    assert_eq!(database.get("city"), Some("Berlin"));
}

#[test]
fn new_entry_has_no_expiration() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.expires_at(), None);
}

#[test]
fn entry_without_expiration_is_not_expired() {
    let entry = Entry::new("value".to_owned());

    assert!(!entry.is_expired(Instant::now()));
}

#[test]
fn entry_is_expired_after_expiration_time() {
    let now = Instant::now();

    let mut entry = Entry::new("value".to_owned());
    entry.set_expires_at(now);

    assert!(entry.is_expired(now));
}

#[test]
fn clear_expiration_removes_expiration() {
    let now = Instant::now();

    let mut entry = Entry::new("value".to_owned());
    entry.set_expires_at(now);
    entry.clear_expiration();

    assert_eq!(entry.expires_at(), None);
    assert!(!entry.is_expired(now));
}

#[test]
fn get_removes_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire_at("key", Instant::now());

    assert_eq!(database.get("key"), None);
    assert!(!database.exists("key"));
}

#[test]
fn get_returns_non_expired_value() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire_at("key", Instant::now() + Duration::from_secs(60));

    assert_eq!(database.get("key"), Some("value"));
}

#[test]
fn expire_at_returns_false_for_missing_key() {
    let mut database = Database::new();

    let result = database.expire_at("missing", Instant::now());

    assert!(!result);
}

#[test]
fn expire_at_sets_expiration_for_existing_key() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("key".to_owned(), "value".to_owned());

    let result = database.expire_at("key", expires_at);

    assert!(result);
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn exists_returns_false_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire_at("key", Instant::now());

    assert!(!database.exists("key"));
}

#[test]
fn delete_returns_false_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire_at("key", Instant::now());

    assert!(!database.delete("key"));
}
#[test]
fn set_if_absent_replaces_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "old".to_owned());
    database.expire_at("key", Instant::now());

    let inserted = database.set_if_absent("key".to_owned(), "new".to_owned());

    assert!(inserted);
    assert_eq!(database.get("key"), Some("new"));
    assert_eq!(database.expiration("key"), None);
}

#[test]
fn append_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("key".to_owned(), "old".to_owned());
    database.expire_at("key", Instant::now());

    let length = database.append("key", "new".to_owned());

    assert_eq!(length, 3);
    assert_eq!(database.get("key"), Some("new"));
}

#[test]
fn increment_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());
    database.expire_at("counter", Instant::now());

    let result = database.increment("counter".to_owned());

    assert_eq!(result, Ok(1));
    assert_eq!(database.get("counter"), Some("1"));
}

#[test]
fn decrement_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());
    database.expire_at("counter", Instant::now());

    let result = database.decrement("counter".to_owned());

    assert_eq!(result, Ok(-1));
    assert_eq!(database.get("counter"), Some("-1"));
}

#[test]
fn get_and_set_does_not_return_expired_value() {
    let mut database = Database::new();

    database.set("key".to_owned(), "old".to_owned());
    database.expire_at("key", Instant::now());

    let result = database.get_and_set("key".to_owned(), "new".to_owned());

    assert_eq!(result, None);
    assert_eq!(database.get("key"), Some("new"));
}

#[test]
fn get_and_delete_does_not_return_expired_value() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire_at("key", Instant::now());

    let result = database.get_and_delete("key".to_owned());

    assert_eq!(result, None);
    assert!(!database.exists("key"));
}

#[test]
fn rename_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("old".to_owned(), "value".to_owned());
    database.expire_at("old", expires_at);

    let renamed = database.rename_key("old", "new".to_owned());

    assert!(renamed);
    assert!(!database.exists("old"));
    assert_eq!(database.get("new"), Some("value"));
    assert_eq!(database.expiration("new"), Some(expires_at));
}

#[test]
fn rename_returns_false_for_expired_source_key() {
    let mut database = Database::new();

    database.set("old".to_owned(), "value".to_owned());
    database.expire_at("old", Instant::now());

    let renamed = database.rename_key("old", "new".to_owned());

    assert!(!renamed);
    assert!(!database.exists("new"));
}

#[test]
fn len_does_not_count_expired_keys() {
    let mut database = Database::new();

    database.set("alive".to_owned(), "value".to_owned());
    database.set("expired".to_owned(), "value".to_owned());
    database.expire_at("expired", Instant::now());

    assert_eq!(database.len(), 1);
}

#[test]
fn keys_do_not_include_expired_keys() {
    let mut database = Database::new();

    database.set("beta".to_owned(), "value".to_owned());
    database.set("expired".to_owned(), "value".to_owned());
    database.set("alpha".to_owned(), "value".to_owned());

    database.expire_at("expired", Instant::now());

    assert_eq!(
        database.keys(),
        vec!["alpha".to_owned(), "beta".to_owned(),]
    );
}

#[test]
fn expire_sets_ttl_for_existing_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    let result = database.expire("key", 60);

    assert!(result);
    assert_eq!(database.get("key"), Some("value"));
}

#[test]
fn expire_returns_false_for_missing_key() {
    let mut database = Database::new();

    let result = database.expire("missing", 60);

    assert!(!result);
}

#[test]
fn expire_with_zero_seconds_expires_key_immediately() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    let result = database.expire("key", 0);

    assert!(result);
    assert_eq!(database.get("key"), None);
}

#[test]
fn ttl_returns_minus_two_for_missing_key() {
    let mut database = Database::new();

    assert_eq!(database.ttl("missing"), -2);
}

#[test]
fn ttl_returns_minus_one_for_key_without_expiration() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    assert_eq!(database.ttl("key"), -1);
}

#[test]
fn ttl_returns_remaining_seconds() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire("key", 60);

    let ttl = database.ttl("key");

    assert!((59..=60).contains(&ttl));
}

#[test]
fn ttl_returns_minus_two_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire("key", 0);

    assert_eq!(database.ttl("key"), -2);
}

#[test]
fn persist_removes_expiration() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire("key", 60);

    let result = database.persist("key");

    assert!(result);
    assert_eq!(database.ttl("key"), -1);
    assert_eq!(database.get("key"), Some("value"));
}

#[test]
fn persist_returns_false_for_key_without_expiration() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    let result = database.persist("key");

    assert!(!result);
    assert_eq!(database.ttl("key"), -1);
}

#[test]
fn persist_returns_false_for_missing_key() {
    let mut database = Database::new();

    assert!(!database.persist("missing"));
}

#[test]
fn persist_returns_false_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.expire("key", 0);

    assert!(!database.persist("key"));
    assert_eq!(database.ttl("key"), -2);
}

#[test]
fn increment_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("counter".to_owned(), "10".to_owned());
    database.expire_at("counter", expires_at);

    let result = database.increment("counter".to_owned());

    assert_eq!(result, Ok(11));
    assert_eq!(database.expiration("counter"), Some(expires_at));
}

#[test]
fn decrement_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("counter".to_owned(), "10".to_owned());
    database.expire_at("counter", expires_at);

    let result = database.decrement("counter".to_owned());

    assert_eq!(result, Ok(9));
    assert_eq!(database.expiration("counter"), Some(expires_at));
}

#[test]
fn append_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("key".to_owned(), "hello".to_owned());
    database.expire_at("key", expires_at);

    database.append("key", " world".to_owned());

    assert_eq!(database.get("key"), Some("hello world"));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn get_and_set_clears_expiration() {
    let mut database = Database::new();

    database.set("key".to_owned(), "old".to_owned());
    database.expire("key", 60);

    database.get_and_set("key".to_owned(), "new".to_owned());

    assert_eq!(database.ttl("key"), -1);
}

#[test]
fn pexpire_sets_expiration_for_existing_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    let result = database.pexpire("key", 60_000);

    assert!(result);

    let ttl = database.pttl("key");
    assert!((59_000..=60_000).contains(&ttl));
}

#[test]
fn pexpire_returns_false_for_missing_key() {
    let mut database = Database::new();

    assert!(!database.pexpire("missing", 60_000));
}

#[test]
fn pexpire_with_zero_expires_key_immediately() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    assert!(database.pexpire("key", 0));
    assert_eq!(database.get("key"), None);
}

#[test]
fn pttl_returns_minus_two_for_missing_key() {
    let mut database = Database::new();

    assert_eq!(database.pttl("missing"), -2);
}

#[test]
fn pttl_returns_minus_one_without_expiration() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());

    assert_eq!(database.pttl("key"), -1);
}

#[test]
fn pttl_returns_remaining_milliseconds() {
    let mut database = Database::new();

    database.set("key".to_owned(), "value".to_owned());
    database.pexpire("key", 10_000);

    let ttl = database.pttl("key");

    assert!((9_000..=10_000).contains(&ttl));
}
