use super::super::in_memory::InMemoryStore as Database;

#[test]
fn set_and_get_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());

    assert_eq!(database.get("name"), Some("initial-value"));
}

#[test]
fn set_overwrites_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());
    database.set("name".to_owned(), "replacement-value".to_owned());

    assert_eq!(database.get("name"), Some("replacement-value"));
    assert_eq!(database.len(), 1);
}

#[test]
fn delete_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "sample-value".to_owned());

    assert!(database.delete("name"));
    assert!(!database.exists("name"));
}

#[test]
fn clear() {
    let mut database = Database::new();

    database.set("name".to_owned(), "first-value".to_owned());
    database.set("surname".to_owned(), "second-value".to_owned());

    database.clear();

    assert_eq!(database.len(), 0);
}

#[test]
fn get_keys() {
    let mut database = Database::new();

    database.set("name".to_owned(), "first-value".to_owned());
    database.set("surname".to_owned(), "second-value".to_owned());

    assert_eq!(database.keys(), ["name", "surname"])
}

#[test]
fn delete_missing_value_returns_false() {
    let mut database = Database::new();

    assert!(!database.delete("missing"));
}

#[test]
fn renames_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "sample-value".to_owned());

    assert!(database.rename("name", "surname".to_owned()))
}

#[test]
fn set_if_absent_inserts_missing_key() {
    let mut database = Database::new();

    let inserted = database.set_if_absent("name".to_owned(), "initial-value".to_owned());

    assert!(inserted);
    assert_eq!(database.get("name"), Some("initial-value"));
}

#[test]
fn set_if_absent_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());

    let inserted = database.set_if_absent("name".to_owned(), "replacement-value".to_owned());

    assert!(!inserted);
    assert_eq!(database.get("name"), Some("initial-value"));
}

#[test]
fn set_if_absent_increases_length_only_once() {
    let mut database = Database::new();

    assert!(database.set_if_absent("name".to_owned(), "initial-value".to_owned(),));

    assert!(!database.set_if_absent("name".to_owned(), "replacement-value".to_owned(),));

    assert_eq!(database.len(), 1);
}

#[test]
fn get_and_delete_returns_existing_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "sample-value".to_owned());

    let result = database.get_and_delete("name".to_owned());

    assert_eq!(result, Some("sample-value".to_owned()));
}

#[test]
fn get_and_delete_removes_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "sample-value".to_owned());

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

    database.set("name".to_owned(), "first-value".to_owned());
    database.set("city".to_owned(), "second-value".to_owned());

    let result = database.get_and_delete("name".to_owned());

    assert_eq!(result, Some("first-value".to_owned()));
    assert_eq!(database.len(), 1);
    assert_eq!(database.get("city"), Some("second-value"));
}
