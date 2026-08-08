use super::super::in_memory::InMemoryStore as Database;
use std::time::{Duration, Instant};

#[test]
fn append() {
    let mut database = Database::new();

    database.set("message".to_owned(), "Hello".to_owned());

    let length = database.append("message", ", world".to_owned());

    assert_eq!(length, 12);
    assert_eq!(database.get("message"), Some("Hello, world"));
}

#[test]
fn append_returns_unicode_scalar_length() {
    let mut database = Database::new();
    database.set("message".to_owned(), "Привет".to_owned());

    let length = database.append("message", " 🌍".to_owned());

    assert_eq!(length, 8);
    assert_eq!(database.get("message"), Some("Привет 🌍"));
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
fn string_length_returns_character_count() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.string_length("key"), 5);
}

#[test]
fn string_length_counts_unicode_characters_instead_of_bytes() {
    let mut database = Database::new();

    database.set("key".to_owned(), "cafe\u{301}".to_owned());

    assert_eq!(database.string_length("key"), 5);
}

#[test]
fn ranges_use_the_same_character_offsets_as_string_length() {
    let mut database = Database::new();

    database.set("key".to_owned(), "a\u{00e9}z".to_owned());

    assert_eq!(database.string_length("key"), 3);
    assert_eq!(database.get_range("key", 1, 1), "\u{00e9}");
    assert_eq!(database.set_range("key".to_owned(), 1, "x".to_owned()), 3);
    assert_eq!(database.get("key"), Some("axz"));
}

#[test]
fn string_length_returns_zero_for_missing_key() {
    let mut database = Database::new();

    assert_eq!(database.string_length("missing"), 0);
}

#[test]
fn string_length_returns_zero_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());
    database.expire("key", 0);

    assert_eq!(database.string_length("key"), 0);
}

#[test]
fn get_range_returns_requested_range() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", 1, 3), "ell");
}

#[test]
fn get_range_includes_end_index() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", 0, 4), "hello");
}

#[test]
fn get_range_supports_negative_indices() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", -3, -1), "llo");
}

#[test]
fn get_range_clamps_start_to_zero() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", -100, 1), "he");
}

#[test]
fn get_range_clamps_end_to_last_character() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", 3, 100), "lo");
}

#[test]
fn get_range_returns_empty_when_start_is_after_end() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", 3, 1), "");
}

#[test]
fn get_range_returns_empty_when_start_is_out_of_bounds() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", 10, 20), "");
}

#[test]
fn get_range_returns_empty_when_end_is_before_string() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    assert_eq!(database.get_range("key", -10, -6), "");
}

#[test]
fn get_range_returns_empty_for_missing_key() {
    let mut database = Database::new();

    assert_eq!(database.get_range("missing", 0, 10), "");
}

#[test]
fn get_range_returns_empty_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());
    database.expire("key", 0);

    assert_eq!(database.get_range("key", 0, 4), "");
}

#[test]
fn set_range_replaces_existing_characters() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    let length = database.set_range("key".to_owned(), 1, "XYZ".to_owned());

    assert_eq!(length, 5);
    assert_eq!(database.get("key"), Some("hXYZo"));
}

#[test]
fn set_range_appends_at_end() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    let length = database.set_range("key".to_owned(), 5, " world".to_owned());

    assert_eq!(length, 11);
    assert_eq!(database.get("key"), Some("hello world"));
}

#[test]
fn set_range_extends_existing_value() {
    let mut database = Database::new();

    database.set("key".to_owned(), "hello".to_owned());

    let length = database.set_range("key".to_owned(), 4, "XYZ".to_owned());

    assert_eq!(length, 7);
    assert_eq!(database.get("key"), Some("hellXYZ"));
}

#[test]
fn set_range_creates_missing_key() {
    let mut database = Database::new();

    let length = database.set_range("key".to_owned(), 0, "hello".to_owned());

    assert_eq!(length, 5);
    assert_eq!(database.get("key"), Some("hello"));
}

#[test]
fn set_range_pads_missing_space_with_null_characters() {
    let mut database = Database::new();

    let length = database.set_range("key".to_owned(), 3, "hi".to_owned());

    assert_eq!(length, 5);
    assert_eq!(database.get("key"), Some("\0\0\0hi"));
}

#[test]
fn set_range_preserves_existing_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("key".to_owned(), "hello".to_owned());
    database.expire_at("key", expires_at);

    database.set_range("key".to_owned(), 1, "XYZ".to_owned());

    assert_eq!(database.get("key"), Some("hXYZo"));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn set_range_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("key".to_owned(), "old".to_owned());
    database.expire("key", 0);

    let length = database.set_range("key".to_owned(), 0, "new".to_owned());

    assert_eq!(length, 3);
    assert_eq!(database.get("key"), Some("new"));
    assert_eq!(database.ttl("key"), -1);
}
