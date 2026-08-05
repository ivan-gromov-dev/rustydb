use super::*;

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
