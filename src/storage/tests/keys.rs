use super::super::in_memory::InMemoryStore as Database;

#[test]
fn set_and_get_value() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "initial-value".to_owned().into());

    assert_eq!(database.get("name"), Ok(Some(b"initial-value".as_slice())));
}

#[test]
fn set_overwrites_value() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "initial-value".to_owned().into());
    database.set(
        "name".to_owned().into(),
        "replacement-value".to_owned().into(),
    );

    assert_eq!(
        database.get("name"),
        Ok(Some(b"replacement-value".as_slice()))
    );
    assert_eq!(database.len(), 1);
}

#[test]
fn key_limit_evicts_lexicographically_smallest_existing_key() {
    let mut database = Database::with_max_keys(Some(2));
    database.set(b"beta".to_vec(), b"2".to_vec());
    database.set(b"alpha".to_vec(), b"1".to_vec());

    database.set(b"gamma".to_vec(), b"3".to_vec());

    assert!(!database.storage.contains_key(b"alpha".as_slice()));
    assert!(database.storage.contains_key(b"beta".as_slice()));
    assert!(database.storage.contains_key(b"gamma".as_slice()));
    assert_eq!(database.evicted_keys(), 1);
}

#[test]
fn overwriting_existing_key_does_not_trigger_eviction() {
    let mut database = Database::with_max_keys(Some(2));
    database.set(b"alpha".to_vec(), b"old".to_vec());
    database.set(b"beta".to_vec(), b"2".to_vec());

    database.set(b"alpha".to_vec(), b"new".to_vec());

    assert_eq!(database.get(b"alpha"), Ok(Some(b"new".as_slice())));
    assert!(database.storage.contains_key(b"beta".as_slice()));
    assert_eq!(database.evicted_keys(), 0);
}

#[test]
fn key_limit_reclaims_expired_key_before_evicting_live_key() {
    let mut database = Database::with_max_keys(Some(2));
    database.set(b"alpha".to_vec(), b"live".to_vec());
    database.set(b"zeta".to_vec(), b"expired".to_vec());
    assert!(database.expire(b"zeta", 0));

    database.set(b"beta".to_vec(), b"new".to_vec());

    assert!(database.storage.contains_key(b"alpha".as_slice()));
    assert!(database.storage.contains_key(b"beta".as_slice()));
    assert!(!database.storage.contains_key(b"zeta".as_slice()));
    assert_eq!(database.evicted_keys(), 0);
}

#[test]
fn every_key_creating_operation_obeys_the_limit() {
    let mut database = Database::with_max_keys(Some(1));

    assert_eq!(database.append(b"append", b"value".to_vec()), Ok(5));
    assert_eq!(database.increment(b"counter".to_vec()), Ok(1));
    assert_eq!(
        database.set_range(b"range".to_vec(), 0, b"x".to_vec()),
        Ok(1)
    );
    assert_eq!(database.push_left(b"list", b"item".to_vec()), Ok(1));
    assert_eq!(database.set_add(b"set", b"member".to_vec()), Ok(true));
    assert!(database.set_if_absent(b"setnx".to_vec(), b"value".to_vec()));
    assert_eq!(
        database.get_and_set(b"getset".to_vec(), b"value".to_vec()),
        Ok(None)
    );

    assert_eq!(database.storage.len(), 1);
    assert!(database.storage.contains_key(b"getset".as_slice()));
    assert_eq!(database.evicted_keys(), 6);
}

#[test]
fn delete_value() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "sample-value".to_owned().into());

    assert!(database.delete("name"));
    assert!(!database.exists("name"));
}

#[test]
fn clear() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "first-value".to_owned().into());
    database.set(
        "surname".to_owned().into(),
        "second-value".to_owned().into(),
    );

    database.clear();

    assert_eq!(database.len(), 0);
}

#[test]
fn get_keys() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "first-value".to_owned().into());
    database.set(
        "surname".to_owned().into(),
        "second-value".to_owned().into(),
    );

    assert_eq!(database.keys(), [b"name".to_vec(), b"surname".to_vec()])
}

#[test]
fn delete_missing_value_returns_false() {
    let mut database = Database::new();

    assert!(!database.delete("missing"));
}

#[test]
fn renames_key() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "sample-value".to_owned().into());

    assert!(database.rename("name", "surname".to_owned().into()))
}

#[test]
fn set_if_absent_inserts_missing_key() {
    let mut database = Database::new();

    let inserted =
        database.set_if_absent("name".to_owned().into(), "initial-value".to_owned().into());

    assert!(inserted);
    assert_eq!(database.get("name"), Ok(Some(b"initial-value".as_slice())));
}

#[test]
fn set_if_absent_does_not_overwrite_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "initial-value".to_owned().into());

    let inserted = database.set_if_absent(
        "name".to_owned().into(),
        "replacement-value".to_owned().into(),
    );

    assert!(!inserted);
    assert_eq!(database.get("name"), Ok(Some(b"initial-value".as_slice())));
}

#[test]
fn set_if_absent_increases_length_only_once() {
    let mut database = Database::new();

    assert!(database.set_if_absent("name".to_owned().into(), "initial-value".to_owned().into(),));

    assert!(!database.set_if_absent(
        "name".to_owned().into(),
        "replacement-value".to_owned().into(),
    ));

    assert_eq!(database.len(), 1);
}

#[test]
fn delete_many_removes_existing_keys() {
    let mut database = Database::new();
    database.set("a".to_owned().into(), "1".to_owned().into());
    database.set("b".to_owned().into(), "2".to_owned().into());
    database.set("c".to_owned().into(), "3".to_owned().into());

    let deleted = database.delete_many(&["a".to_owned().into(), "c".to_owned().into()]);

    assert_eq!(deleted, 2);
    assert_eq!(database.get("a"), Ok(None));
    assert_eq!(database.get("b"), Ok(Some(b"2".as_slice())));
    assert_eq!(database.get("c"), Ok(None));
}

#[test]
fn delete_many_ignores_missing_and_duplicate_keys() {
    let mut database = Database::new();
    database.set("a".to_owned().into(), "1".to_owned().into());

    let deleted = database.delete_many(&[
        "a".to_owned().into(),
        "missing".to_owned().into(),
        "a".to_owned().into(),
    ]);

    assert_eq!(deleted, 1);
}

#[test]
fn delete_many_treats_expired_keys_as_missing() {
    let mut database = Database::new();
    database.set("expired".to_owned().into(), "1".to_owned().into());
    database.set("active".to_owned().into(), "2".to_owned().into());
    database.expire("expired", 0);

    let deleted = database.delete_many(&["expired".to_owned().into(), "active".to_owned().into()]);

    assert_eq!(deleted, 1);
}

#[test]
fn exists_many_counts_existing_and_duplicate_keys() {
    let mut database = Database::new();
    database.set("a".to_owned().into(), "1".to_owned().into());
    database.set("b".to_owned().into(), "2".to_owned().into());

    let count = database.exists_many(&[
        "a".to_owned().into(),
        "a".to_owned().into(),
        "missing".to_owned().into(),
        "b".to_owned().into(),
    ]);

    assert_eq!(count, 3);
}

#[test]
fn exists_many_does_not_count_expired_keys() {
    let mut database = Database::new();
    database.set("expired".to_owned().into(), "1".to_owned().into());
    database.set("active".to_owned().into(), "2".to_owned().into());
    database.expire("expired", 0);

    let count = database.exists_many(&["expired".to_owned().into(), "active".to_owned().into()]);

    assert_eq!(count, 1);
}

#[test]
fn get_and_delete_returns_existing_value() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "sample-value".to_owned().into());

    let result = database.get_and_delete("name".to_owned().into());

    assert_eq!(result, Ok(Some("sample-value".to_owned().into())));
}

#[test]
fn get_and_delete_removes_existing_key() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "sample-value".to_owned().into());

    assert_eq!(
        database.get_and_delete("name".to_owned().into()),
        Ok(Some("sample-value".to_owned().into()))
    );

    assert_eq!(database.get("name"), Ok(None));
    assert!(!database.exists("name"));
}

#[test]
fn get_and_delete_missing_key_returns_none() {
    let mut database = Database::new();

    let result = database.get_and_delete("missing".to_owned().into());

    assert_eq!(result, Ok(None));
}

#[test]
fn get_and_delete_decreases_database_length() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "first-value".to_owned().into());
    database.set("city".to_owned().into(), "second-value".to_owned().into());

    let result = database.get_and_delete("name".to_owned().into());

    assert_eq!(result, Ok(Some("first-value".to_owned().into())));
    assert_eq!(database.len(), 1);
    assert_eq!(database.get("city"), Ok(Some(b"second-value".as_slice())));
}
