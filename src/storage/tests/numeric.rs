use super::super::in_memory::{InMemoryStore as Database, StoreError as DatabaseError};
use std::time::{Duration, Instant};

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
fn increment_by_float_creates_missing_key() {
    let mut database = Database::new();

    let result = database.increment_by_float("counter".to_owned(), 1.5);

    assert_eq!(result, Ok(1.5));
    assert_eq!(database.get("counter"), Some("1.5"));
}

#[test]
fn increment_by_float_increments_existing_float() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10.5".to_owned());

    let result = database.increment_by_float("counter".to_owned(), 2.25);

    assert_eq!(result, Ok(12.75));
    assert_eq!(database.get("counter"), Some("12.75"));
}

#[test]
fn increment_by_float_accepts_integer_value() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10".to_owned());

    let result = database.increment_by_float("counter".to_owned(), 0.5);

    assert_eq!(result, Ok(10.5));
    assert_eq!(database.get("counter"), Some("10.5"));
}

#[test]
fn increment_by_float_supports_negative_amount() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10.5".to_owned());

    let result = database.increment_by_float("counter".to_owned(), -2.5);

    assert_eq!(result, Ok(8.0));
    assert_eq!(database.get("counter"), Some("8"));
}

#[test]
fn increment_by_float_returns_error_for_non_float_value() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "hello".to_owned());

    let result = database.increment_by_float("counter".to_owned(), 1.5);

    assert_eq!(result, Err(DatabaseError::ValueIsNotFloat));

    assert_eq!(database.get("counter"), Some("hello"));
}

#[test]
fn increment_by_float_returns_error_for_infinite_stored_value() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "inf".to_owned());

    let result = database.increment_by_float("counter".to_owned(), 1.0);

    assert_eq!(result, Err(DatabaseError::ValueIsNotFloat));

    assert_eq!(database.get("counter"), Some("inf"));
}

#[test]
fn increment_by_float_returns_error_for_non_finite_result() {
    let mut database = Database::new();

    database.set("counter".to_owned(), f64::MAX.to_string());

    let old_value = database.get("counter").unwrap().to_owned();

    let result = database.increment_by_float("counter".to_owned(), f64::MAX);

    assert_eq!(result, Err(DatabaseError::FloatIsNotFinite));

    assert_eq!(database.get("counter"), Some(old_value.as_str()));
}

#[test]
fn increment_by_float_preserves_expiration() {
    let mut database = Database::new();

    let expires_at = Instant::now() + Duration::from_secs(60);

    database.set("counter".to_owned(), "10.5".to_owned());

    database.expire_at("counter", expires_at);

    let result = database.increment_by_float("counter".to_owned(), 1.5);

    assert_eq!(result, Ok(12.0));

    assert_eq!(database.expiration("counter"), Some(expires_at));
}

#[test]
fn increment_by_float_treats_expired_key_as_missing() {
    let mut database = Database::new();

    database.set("counter".to_owned(), "10.5".to_owned());

    database.expire("counter", 0);

    let result = database.increment_by_float("counter".to_owned(), 1.5);

    assert_eq!(result, Ok(1.5));
    assert_eq!(database.get("counter"), Some("1.5"));
}
