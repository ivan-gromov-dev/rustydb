use super::super::in_memory::{InMemoryStore as Database, SetOperation, StoreError};
use std::time::{Duration, Instant};

#[test]
fn add_reports_new_members_and_ignores_duplicates() {
    let mut database = Database::new();

    assert_eq!(
        database.set_add("set", "member".to_owned().into()),
        Ok(true)
    );
    assert_eq!(
        database.set_add("set", "member".to_owned().into()),
        Ok(false)
    );
    assert_eq!(database.set_cardinality("set"), Ok(1));
}

#[test]
fn membership_and_cardinality_handle_missing_keys() {
    let mut database = Database::new();

    assert_eq!(database.set_contains("missing", "member"), Ok(false));
    assert_eq!(database.set_cardinality("missing"), Ok(0));
    assert_eq!(database.set_members("missing"), Ok(Vec::new()));
}

#[test]
fn members_are_returned_in_sorted_order() {
    let mut database = Database::new();
    database.set_add("set", "zeta".to_owned().into()).unwrap();
    database.set_add("set", "alpha".to_owned().into()).unwrap();
    database
        .set_add("set", "middle value".to_owned().into())
        .unwrap();

    assert_eq!(
        database.set_members("set"),
        Ok(vec![
            "alpha".to_owned().into(),
            "middle value".to_owned().into(),
            "zeta".to_owned().into(),
        ])
    );
}

#[test]
fn remove_reports_membership_and_removes_the_last_members_key() {
    let mut database = Database::new();
    database.set_set(
        "set".to_owned().into(),
        vec!["first".to_owned().into(), "last".to_owned().into()],
    );

    assert_eq!(database.set_remove("set", "missing"), Ok(false));
    assert_eq!(database.set_remove("set", "first"), Ok(true));
    assert!(database.exists("set"));
    assert_eq!(database.set_remove("set", "last"), Ok(true));
    assert!(!database.exists("set"));
    assert_eq!(database.ttl("set"), -2);
}

#[test]
fn set_mutations_preserve_expiration_while_members_remain() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set_set(
        "set".to_owned().into(),
        vec!["first".to_owned().into(), "second".to_owned().into()],
    );
    assert!(database.expire_at("set", expires_at));

    assert_eq!(database.set_add("set", "third".to_owned().into()), Ok(true));
    assert_eq!(database.set_remove("set", "first"), Ok(true));
    assert_eq!(database.expiration("set"), Some(expires_at));
}

#[test]
fn set_commands_treat_expired_keys_as_missing() {
    let mut database = Database::new();
    database.set_set("set".to_owned().into(), vec!["old".to_owned().into()]);
    assert!(database.expire("set", 0));

    assert_eq!(database.set_contains("set", "old"), Ok(false));
    assert_eq!(database.set_remove("set", "old"), Ok(false));
    assert_eq!(database.set_add("set", "new".to_owned().into()), Ok(true));
    assert_eq!(database.ttl("set"), -1);
}

#[test]
fn set_commands_reject_other_types_without_mutation() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set("string".to_owned().into(), "value".to_owned().into());
    assert!(database.expire_at("string", expires_at));
    database.set_list("list".to_owned().into(), vec!["value".to_owned().into()]);

    assert_eq!(
        database.set_add("string", "member".to_owned().into()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.set_remove("string", "member"),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.set_contains("string", "member"),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.set_members("list"), Err(StoreError::WrongType));
    assert_eq!(database.set_cardinality("list"), Err(StoreError::WrongType));

    assert_eq!(database.get("string"), Ok(Some(b"value".as_slice())));
    assert_eq!(database.expiration("string"), Some(expires_at));
    assert_eq!(
        database.list_values("list"),
        Ok(Some(vec!["value".to_owned().into()]))
    );
}

#[test]
fn variadic_set_mutations_count_distinct_changes_and_remove_empty_key() {
    let mut database = Database::new();
    assert_eq!(
        database.set_add_many(
            "set",
            vec![b"one".to_vec(), b"two".to_vec(), b"one".to_vec()]
        ),
        Ok(2)
    );
    assert_eq!(
        database.set_remove_many(
            "set",
            &[b"one".to_vec(), b"one".to_vec(), b"missing".to_vec()]
        ),
        Ok(1)
    );
    assert_eq!(database.set_remove_many("set", &[b"two".to_vec()]), Ok(1));
    assert!(!database.exists("set"));
}

#[test]
fn multi_membership_preserves_order_duplicates_and_missing_results() {
    let mut database = Database::new();
    database.set_set(b"set".to_vec(), vec![b"a".to_vec(), b"b".to_vec()]);
    assert_eq!(
        database.set_contains_many("set", &[b"b".to_vec(), b"x".to_vec(), b"b".to_vec()]),
        Ok(vec![true, false, true])
    );
    assert_eq!(
        database.set_contains_many("missing", &[b"a".to_vec()]),
        Ok(vec![false])
    );
}

#[test]
fn set_pop_is_sorted_preserves_ttl_and_removes_empty_keys() {
    let mut database = Database::new();
    database.set_set(
        b"set".to_vec(),
        vec![b"c".to_vec(), b"a".to_vec(), b"b".to_vec()],
    );
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("set", expires_at));
    assert_eq!(
        database.set_pop("set", 2),
        Ok(vec![b"a".to_vec(), b"b".to_vec()])
    );
    assert_eq!(database.expiration("set"), Some(expires_at));
    assert_eq!(database.set_pop("set", 10), Ok(vec![b"c".to_vec()]));
    assert!(!database.exists("set"));
}

#[test]
fn random_members_support_unique_and_repeated_counts_without_mutation() {
    let mut database = Database::new();
    database.set_set(b"set".to_vec(), vec![b"a".to_vec(), b"b".to_vec()]);
    let unique = database.set_random_members("set", 10).unwrap();
    assert_eq!(unique.len(), 2);
    assert_ne!(unique[0], unique[1]);
    let repeated = database.set_random_members("set", -5).unwrap();
    assert_eq!(repeated.len(), 5);
    assert_eq!(database.set_cardinality("set"), Ok(2));
    assert_eq!(database.set_random_members("set", 0), Ok(Vec::new()));
}

#[test]
fn move_is_atomic_and_preserves_collection_expirations() {
    let mut database = Database::new();
    database.set_set(b"source".to_vec(), vec![b"a".to_vec(), b"b".to_vec()]);
    database.set_set(b"destination".to_vec(), vec![b"c".to_vec()]);
    let source_expiration = Instant::now() + Duration::from_secs(60);
    let destination_expiration = Instant::now() + Duration::from_secs(120);
    assert!(database.expire_at("source", source_expiration));
    assert!(database.expire_at("destination", destination_expiration));

    assert_eq!(database.set_move("source", "destination", "a"), Ok(true));
    assert_eq!(database.set_members("source"), Ok(vec![b"b".to_vec()]));
    assert_eq!(
        database.set_members("destination"),
        Ok(vec![b"a".to_vec(), b"c".to_vec()])
    );
    assert_eq!(database.expiration("source"), Some(source_expiration));
    assert_eq!(
        database.expiration("destination"),
        Some(destination_expiration)
    );
    assert_eq!(database.set_move("source", "source", "b"), Ok(true));
    assert_eq!(
        database.set_move("source", "destination", "missing"),
        Ok(false)
    );
}

#[test]
fn move_validates_types_before_mutating_source() {
    let mut database = Database::new();
    database.set_set(b"source".to_vec(), vec![b"member".to_vec()]);
    database.set(b"destination".to_vec(), b"string".to_vec());

    assert_eq!(
        database.set_move("source", "destination", "member"),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.set_contains("source", "member"), Ok(true));
    assert_eq!(
        database.set_move("missing", "destination", "member"),
        Ok(false)
    );
}

#[test]
fn set_algebra_is_sorted_and_handles_missing_and_duplicate_keys() {
    let mut database = Database::new();
    database.set_set(
        b"first".to_vec(),
        vec![b"c".to_vec(), b"a".to_vec(), b"b".to_vec()],
    );
    database.set_set(b"second".to_vec(), vec![b"b".to_vec(), b"d".to_vec()]);

    assert_eq!(
        database.set_algebra(
            &[b"first".to_vec(), b"second".to_vec()],
            SetOperation::Difference
        ),
        Ok(vec![b"a".to_vec(), b"c".to_vec()])
    );
    assert_eq!(
        database.set_algebra(
            &[b"first".to_vec(), b"second".to_vec()],
            SetOperation::Intersection
        ),
        Ok(vec![b"b".to_vec()])
    );
    assert_eq!(
        database.set_algebra(
            &[b"first".to_vec(), b"missing".to_vec()],
            SetOperation::Union
        ),
        Ok(vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()])
    );
    assert_eq!(
        database.set_algebra(
            &[b"first".to_vec(), b"first".to_vec()],
            SetOperation::Difference
        ),
        Ok(Vec::new())
    );
}

#[test]
fn set_algebra_store_replaces_destination_clears_ttl_and_deletes_empty_results() {
    let mut database = Database::new();
    database.set_set(b"source".to_vec(), vec![b"a".to_vec(), b"b".to_vec()]);
    database.set_set(b"other".to_vec(), vec![b"b".to_vec()]);
    assert!(database.expire("source", 60));

    assert_eq!(
        database.set_algebra_store(
            b"source".to_vec(),
            &[b"source".to_vec(), b"other".to_vec()],
            SetOperation::Intersection
        ),
        Ok(1)
    );
    assert_eq!(database.set_members("source"), Ok(vec![b"b".to_vec()]));
    assert_eq!(database.ttl("source"), -1);
    database.set(b"destination".to_vec(), b"old value".to_vec());
    assert_eq!(
        database.set_algebra_store(
            b"destination".to_vec(),
            &[b"missing".to_vec()],
            SetOperation::Union
        ),
        Ok(0)
    );
    assert!(!database.exists("destination"));
}

#[test]
fn set_algebra_validates_every_source_before_store_mutation() {
    let mut database = Database::new();
    database.set_set(b"valid".to_vec(), vec![b"member".to_vec()]);
    database.set(b"wrong".to_vec(), b"string".to_vec());
    database.set_set(b"destination".to_vec(), vec![b"preserved".to_vec()]);

    assert_eq!(
        database.set_algebra_store(
            b"destination".to_vec(),
            &[b"missing".to_vec(), b"wrong".to_vec()],
            SetOperation::Intersection
        ),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.set_members("destination"),
        Ok(vec![b"preserved".to_vec()])
    );
}

#[test]
fn set_scan_uses_sorted_cursor_ranges_and_match_filters() {
    let mut database = Database::new();
    database.set_set(
        b"set".to_vec(),
        vec![b"c:2".to_vec(), b"a:1".to_vec(), b"b:1".to_vec()],
    );
    assert_eq!(
        database.set_scan("set", 0, Some(b"*:1"), 2),
        Ok((2, vec![b"a:1".to_vec(), b"b:1".to_vec()]))
    );
    assert_eq!(
        database.set_scan("set", 2, Some(b"*:1"), 2),
        Ok((0, Vec::new()))
    );
    assert_eq!(database.set_scan("set", 99, None, 2), Ok((0, Vec::new())));
    assert_eq!(
        database.set_scan("missing", 0, None, 2),
        Ok((0, Vec::new()))
    );
}
