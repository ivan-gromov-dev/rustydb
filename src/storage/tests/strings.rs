use super::super::in_memory::InMemoryStore as Database;
use std::time::{Duration, Instant};

#[test]
fn string_operations_preserve_arbitrary_bytes() {
    let mut database = Database::new();
    database.set(b"\xff-key".to_vec(), b"a\0\xff".to_vec());

    assert_eq!(database.string_length(b"\xff-key"), Ok(3));
    assert_eq!(database.append(b"\xff-key", b"\x80".to_vec()), Ok(4));
    assert_eq!(
        database.get_range(b"\xff-key", 1, 3),
        Ok(b"\0\xff\x80".to_vec())
    );
    assert_eq!(
        database.get(b"\xff-key"),
        Ok(Some(b"a\0\xff\x80".as_slice()))
    );
}

#[test]
fn append() {
    let mut database = Database::new();

    database.set("message".to_owned().into(), "Hello".to_owned().into());

    let length = database.append("message", ", world".to_owned().into());

    assert_eq!(length, Ok(12));
    assert_eq!(
        database.get("message"),
        Ok(Some(b"Hello, world".as_slice()))
    );
}

#[test]
fn append_returns_byte_length() {
    let mut database = Database::new();
    database.set("message".to_owned().into(), "Привет".to_owned().into());

    let length = database.append("message", " 🌍".to_owned().into());

    assert_eq!(length, Ok(17));
    assert_eq!(database.get("message"), Ok(Some("Привет 🌍".as_bytes())));
}

#[test]
fn append_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "old".to_owned().into());
    database.expire_at("key", Instant::now());

    let length = database.append("key", "new".to_owned().into());

    assert_eq!(length, Ok(3));
    assert_eq!(database.get("key"), Ok(Some(b"new".as_slice())));
}

#[test]
fn append_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("key".to_owned().into(), "hello".to_owned().into());
    database.expire_at("key", expires_at);

    assert_eq!(database.append("key", " world".to_owned().into()), Ok(11));

    assert_eq!(database.get("key"), Ok(Some(b"hello world".as_slice())));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn string_length_returns_byte_count() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(database.string_length("key"), Ok(5));
}

#[test]
fn string_length_counts_utf8_bytes() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "cafe\u{301}".to_owned().into());

    assert_eq!(database.string_length("key"), Ok(6));
}

#[test]
fn ranges_use_the_same_byte_offsets_as_string_length() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "a\u{00e9}z".to_owned().into());

    assert_eq!(database.string_length("key"), Ok(4));
    assert_eq!(
        database.get_range("key", 1, 2).as_deref(),
        Ok("é".as_bytes())
    );
    assert_eq!(
        database.set_range("key".to_owned().into(), 1, "XY".to_owned().into()),
        Ok(4)
    );
    assert_eq!(database.get("key"), Ok(Some(b"aXYz".as_slice())));
}

#[test]
fn string_length_returns_zero_for_missing_key() {
    let mut database = Database::new();

    assert_eq!(database.string_length("missing"), Ok(0));
}

#[test]
fn string_length_returns_zero_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());
    database.expire("key", 0);

    assert_eq!(database.string_length("key"), Ok(0));
}

#[test]
fn get_range_returns_requested_range() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", 1, 3).as_deref(),
        Ok(b"ell".as_slice())
    );
}

#[test]
fn get_range_includes_end_index() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", 0, 4).as_deref(),
        Ok(b"hello".as_slice())
    );
}

#[test]
fn get_range_supports_negative_indices() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", -3, -1).as_deref(),
        Ok(b"llo".as_slice())
    );
}

#[test]
fn get_range_clamps_start_to_zero() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", -100, 1).as_deref(),
        Ok(b"he".as_slice())
    );
}

#[test]
fn get_range_clamps_end_to_last_character() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", 3, 100).as_deref(),
        Ok(b"lo".as_slice())
    );
}

#[test]
fn get_range_returns_empty_when_start_is_after_end() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", 3, 1).as_deref(),
        Ok(b"".as_slice())
    );
}

#[test]
fn get_range_returns_empty_when_start_is_out_of_bounds() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", 10, 20).as_deref(),
        Ok(b"".as_slice())
    );
}

#[test]
fn get_range_returns_empty_when_end_is_before_string() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    assert_eq!(
        database.get_range("key", -10, -6).as_deref(),
        Ok(b"".as_slice())
    );
}

#[test]
fn get_range_returns_empty_for_missing_key() {
    let mut database = Database::new();

    assert_eq!(
        database.get_range("missing", 0, 10).as_deref(),
        Ok(b"".as_slice())
    );
}

#[test]
fn get_range_returns_empty_for_expired_key() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());
    database.expire("key", 0);

    assert_eq!(
        database.get_range("key", 0, 4).as_deref(),
        Ok(b"".as_slice())
    );
}

#[test]
fn set_range_replaces_existing_characters() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    let length = database.set_range("key".to_owned().into(), 1, "XYZ".to_owned().into());

    assert_eq!(length, Ok(5));
    assert_eq!(database.get("key"), Ok(Some(b"hXYZo".as_slice())));
}

#[test]
fn set_range_appends_at_end() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    let length = database.set_range("key".to_owned().into(), 5, " world".to_owned().into());

    assert_eq!(length, Ok(11));
    assert_eq!(database.get("key"), Ok(Some(b"hello world".as_slice())));
}

#[test]
fn set_range_extends_existing_value() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "hello".to_owned().into());

    let length = database.set_range("key".to_owned().into(), 4, "XYZ".to_owned().into());

    assert_eq!(length, Ok(7));
    assert_eq!(database.get("key"), Ok(Some(b"hellXYZ".as_slice())));
}

#[test]
fn set_range_creates_missing_key() {
    let mut database = Database::new();

    let length = database.set_range("key".to_owned().into(), 0, "hello".to_owned().into());

    assert_eq!(length, Ok(5));
    assert_eq!(database.get("key"), Ok(Some(b"hello".as_slice())));
}

#[test]
fn set_range_pads_missing_space_with_null_characters() {
    let mut database = Database::new();

    let length = database.set_range("key".to_owned().into(), 3, "hi".to_owned().into());

    assert_eq!(length, Ok(5));
    assert_eq!(database.get("key"), Ok(Some(b"\0\0\0hi".as_slice())));
}

#[test]
fn set_range_preserves_existing_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("key".to_owned().into(), "hello".to_owned().into());
    database.expire_at("key", expires_at);

    assert_eq!(
        database.set_range("key".to_owned().into(), 1, "XYZ".to_owned().into()),
        Ok(5)
    );

    assert_eq!(database.get("key"), Ok(Some(b"hXYZo".as_slice())));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn set_range_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("key".to_owned().into(), "old".to_owned().into());
    database.expire("key", 0);

    let length = database.set_range("key".to_owned().into(), 0, "new".to_owned().into());

    assert_eq!(length, Ok(3));
    assert_eq!(database.get("key"), Ok(Some(b"new".as_slice())));
    assert_eq!(database.ttl("key"), -1);
}
