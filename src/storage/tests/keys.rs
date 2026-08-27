use super::super::in_memory::InMemoryStore as Database;

#[test]
fn set_and_get_value() {
    let mut database = Database::new();

    database.set("name".to_owned().into(), "initial-value".to_owned().into());

    assert_eq!(database.get("name"), Ok(Some(b"initial-value".as_slice())));
}

#[test]
fn key_patterns_scan_random_and_copy_cover_keyspace_semantics() {
    let mut database = Database::new();
    database.set(b"user:1".to_vec(), b"one".to_vec());
    database.set(b"user:2".to_vec(), b"two".to_vec());
    database.push_left(b"queue:1", b"job".to_vec()).unwrap();
    assert!(database.expire(b"user:1", 60));

    assert_eq!(
        database.keys_matching(b"user:[12]"),
        vec![b"user:1".to_vec(), b"user:2".to_vec()]
    );
    assert_eq!(
        database.scan(0, Some(b"*:*"), 2, Some(b"string")),
        (2, vec![b"user:1".to_vec()])
    );
    assert_eq!(
        database.scan(2, Some(b"*:*"), 2, Some(b"string")),
        (0, vec![b"user:2".to_vec()])
    );
    assert!(database.random_key().is_some());

    assert_eq!(database.copy(b"user:1", b"copy".to_vec(), false), Ok(true));
    assert_eq!(database.get(b"copy"), Ok(Some(b"one".as_slice())));
    assert_eq!(database.ttl(b"copy"), database.ttl(b"user:1"));
    assert_eq!(database.copy(b"user:2", b"copy".to_vec(), false), Ok(false));
    assert_eq!(database.copy(b"user:2", b"copy".to_vec(), true), Ok(true));
    assert_eq!(database.get(b"copy"), Ok(Some(b"two".as_slice())));
    assert_eq!(
        database.copy(b"copy", b"copy".to_vec(), true),
        Err(super::super::in_memory::StoreError::SameSourceDestination)
    );
}

#[test]
fn random_key_and_scan_handle_empty_or_finished_keyspaces() {
    let mut database = Database::new();
    assert_eq!(database.random_key(), None);
    assert_eq!(database.scan(0, None, 10, None), (0, Vec::new()));
    database.set(b"key".to_vec(), b"value".to_vec());
    assert_eq!(database.scan(9, None, 10, None), (0, Vec::new()));
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
fn reclamation_metrics_distinguish_deletion_expiration_and_eviction() {
    let mut database = Database::with_max_keys(Some(2));
    database.set(b"deleted".to_vec(), b"value".to_vec());
    assert!(database.delete(b"deleted"));

    database.set(b"live".to_vec(), b"value".to_vec());
    database.set(b"expired".to_vec(), b"value".to_vec());
    assert!(database.expire(b"expired", 0));
    database.set(b"replacement".to_vec(), b"value".to_vec());
    database.set(b"new".to_vec(), b"value".to_vec());

    let metrics = database.reclamation_metrics();
    assert_eq!(metrics.deletions, 1);
    assert_eq!(metrics.expirations, 1);
    assert_eq!(metrics.evictions, 1);
}

#[test]
fn deletion_metrics_cover_commands_that_remove_whole_keys() {
    let mut database = Database::new();
    database.push_right(b"list", b"item".to_vec()).unwrap();
    assert_eq!(database.pop_left(b"list"), Ok(Some(b"item".to_vec())));
    database.push_left(b"other-list", b"item".to_vec()).unwrap();
    assert_eq!(
        database.pop_right(b"other-list"),
        Ok(Some(b"item".to_vec()))
    );
    database.set_add(b"set", b"member".to_vec()).unwrap();
    assert_eq!(database.set_remove(b"set", b"member"), Ok(true));
    database.set(b"source".to_vec(), b"value".to_vec());
    database.set(b"target".to_vec(), b"value".to_vec());
    assert!(database.rename(b"source", b"target".to_vec()));
    database.set(b"getdel".to_vec(), b"value".to_vec());
    assert_eq!(
        database.get_and_delete(b"getdel".to_vec()),
        Ok(Some(b"value".to_vec()))
    );
    database.set(b"clear-one".to_vec(), b"value".to_vec());
    database.set(b"clear-two".to_vec(), b"value".to_vec());
    database.clear();

    let metrics = database.reclamation_metrics();
    assert_eq!(metrics.deletions, 8);
    assert_eq!(metrics.expirations, 0);
    assert_eq!(metrics.evictions, 0);
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
