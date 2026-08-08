use super::super::in_memory::InMemoryStore as Database;
use super::super::stored_value::StoredValue as Entry;
use std::time::{Duration, Instant};

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

    let renamed = database.rename("old", "new".to_owned());

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

    let renamed = database.rename("old", "new".to_owned());

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
fn expire_rejects_duration_outside_instant_range_without_panicking() {
    let mut database = Database::new();
    database.set("key".to_owned(), "value".to_owned());

    assert!(!database.expire("key", u64::MAX));
    assert_eq!(database.get("key"), Some("value"));
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
