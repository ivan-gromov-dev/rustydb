use super::super::in_memory::InMemoryStore as Database;

#[test]
fn set_and_get_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());

    assert_eq!(database.get("name"), Ok(Some("initial-value")));
}

#[test]
fn set_overwrites_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());
    database.set("name".to_owned(), "replacement-value".to_owned());

    assert_eq!(database.get("name"), Ok(Some("replacement-value")));
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
    assert_eq!(database.get("name"), Ok(Some("initial-value")));
}

#[test]
fn set_if_absent_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "initial-value".to_owned());

    let inserted = database.set_if_absent("name".to_owned(), "replacement-value".to_owned());

    assert!(!inserted);
    assert_eq!(database.get("name"), Ok(Some("initial-value")));
}

#[test]
fn set_if_absent_increases_length_only_once() {
    let mut database = Database::new();

    assert!(database.set_if_absent("name".to_owned(), "initial-value".to_owned(),));

    assert!(!database.set_if_absent("name".to_owned(), "replacement-value".to_owned(),));

    assert_eq!(database.len(), 1);
}

#[test]
fn delete_many_removes_existing_keys() {
    let mut database = Database::new();
    database.set("a".to_owned(), "1".to_owned());
    database.set("b".to_owned(), "2".to_owned());
    database.set("c".to_owned(), "3".to_owned());

    let deleted = database.delete_many(&["a".to_owned(), "c".to_owned()]);

    assert_eq!(deleted, 2);
    assert_eq!(database.get("a"), Ok(None));
    assert_eq!(database.get("b"), Ok(Some("2")));
    assert_eq!(database.get("c"), Ok(None));
}

#[test]
fn delete_many_ignores_missing_and_duplicate_keys() {
    let mut database = Database::new();
    database.set("a".to_owned(), "1".to_owned());

    let deleted = database.delete_many(&["a".to_owned(), "missing".to_owned(), "a".to_owned()]);

    assert_eq!(deleted, 1);
}

#[test]
fn delete_many_treats_expired_keys_as_missing() {
    let mut database = Database::new();
    database.set("expired".to_owned(), "1".to_owned());
    database.set("active".to_owned(), "2".to_owned());
    database.expire("expired", 0);

    let deleted = database.delete_many(&["expired".to_owned(), "active".to_owned()]);

    assert_eq!(deleted, 1);
}

#[test]
fn exists_many_counts_existing_and_duplicate_keys() {
    let mut database = Database::new();
    database.set("a".to_owned(), "1".to_owned());
    database.set("b".to_owned(), "2".to_owned());

    let count = database.exists_many(&[
        "a".to_owned(),
        "a".to_owned(),
        "missing".to_owned(),
        "b".to_owned(),
    ]);

    assert_eq!(count, 3);
}

#[test]
fn exists_many_does_not_count_expired_keys() {
    let mut database = Database::new();
    database.set("expired".to_owned(), "1".to_owned());
    database.set("active".to_owned(), "2".to_owned());
    database.expire("expired", 0);

    let count = database.exists_many(&["expired".to_owned(), "active".to_owned()]);

    assert_eq!(count, 1);
}

#[test]
fn get_and_delete_returns_existing_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "sample-value".to_owned());

    let result = database.get_and_delete("name".to_owned());

    assert_eq!(result, Ok(Some("sample-value".to_owned())));
}

#[test]
fn get_and_delete_removes_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "sample-value".to_owned());

    assert_eq!(
        database.get_and_delete("name".to_owned()),
        Ok(Some("sample-value".to_owned()))
    );

    assert_eq!(database.get("name"), Ok(None));
    assert!(!database.exists("name"));
}

#[test]
fn get_and_delete_missing_key_returns_none() {
    let mut database = Database::new();

    let result = database.get_and_delete("missing".to_owned());

    assert_eq!(result, Ok(None));
}

#[test]
fn get_and_delete_decreases_database_length() {
    let mut database = Database::new();

    database.set("name".to_owned(), "first-value".to_owned());
    database.set("city".to_owned(), "second-value".to_owned());

    let result = database.get_and_delete("name".to_owned());

    assert_eq!(result, Ok(Some("first-value".to_owned())));
    assert_eq!(database.len(), 1);
    assert_eq!(database.get("city"), Ok(Some("second-value")));
}
