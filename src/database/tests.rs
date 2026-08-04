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
