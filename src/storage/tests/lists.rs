use super::super::in_memory::{InMemoryStore as Database, StoreError};
use std::time::{Duration, Instant};

#[test]
fn push_left_creates_a_list_and_prepends_values() {
    let mut database = Database::new();

    assert_eq!(database.push_left("key", "one".to_owned().into()), Ok(1));
    assert_eq!(database.push_left("key", "two".to_owned().into()), Ok(2));

    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["two".to_owned().into(), "one".to_owned().into()]))
    );
}

#[test]
fn push_right_creates_a_list_and_appends_values() {
    let mut database = Database::new();

    assert_eq!(database.push_right("key", "one".to_owned().into()), Ok(1));
    assert_eq!(database.push_right("key", "two".to_owned().into()), Ok(2));

    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["one".to_owned().into(), "two".to_owned().into()]))
    );
}

#[test]
fn pushes_at_opposite_ends_preserve_list_order() {
    let mut database = Database::new();

    database
        .push_right("key", "middle".to_owned().into())
        .unwrap();
    database
        .push_left("key", "first".to_owned().into())
        .unwrap();
    database
        .push_right("key", "last".to_owned().into())
        .unwrap();

    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec![
            "first".to_owned().into(),
            "middle".to_owned().into(),
            "last".to_owned().into(),
        ]))
    );
}

#[test]
fn list_length_handles_existing_and_missing_lists() {
    let mut database = Database::new();

    assert_eq!(database.list_length("missing"), Ok(0));

    database.push_right("key", "one".to_owned().into()).unwrap();
    database.push_right("key", "two".to_owned().into()).unwrap();

    assert_eq!(database.list_length("key"), Ok(2));
}

#[test]
fn list_commands_reject_strings_without_mutating_them() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set("key".to_owned().into(), "value".to_owned().into());
    assert!(database.expire_at("key", expires_at));

    assert_eq!(
        database.push_left("key", "left".to_owned().into()),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.push_right("key", "right".to_owned().into()),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.list_length("key"), Err(StoreError::WrongType));

    assert_eq!(database.get("key"), Ok(Some(b"value".as_slice())));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn pushing_to_an_existing_list_preserves_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.push_right("key", "one".to_owned().into()).unwrap();
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.push_left("key", "zero".to_owned().into()), Ok(2));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn list_commands_treat_expired_keys_as_missing() {
    let mut database = Database::new();
    database.push_right("key", "old".to_owned().into()).unwrap();
    assert!(database.expire("key", 0));

    assert_eq!(database.list_length("key"), Ok(0));
    assert_eq!(database.push_left("key", "new".to_owned().into()), Ok(1));
    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["new".to_owned().into()]))
    );
    assert_eq!(database.ttl("key"), -1);
}

#[test]
fn pop_left_and_right_remove_values_from_opposite_ends() {
    let mut database = Database::new();
    database.set_list(
        "key".to_owned().into(),
        vec![
            "first".to_owned().into(),
            "middle".to_owned().into(),
            "last".to_owned().into(),
        ],
    );

    assert_eq!(
        database.pop_left("key"),
        Ok(Some("first".to_owned().into()))
    );
    assert_eq!(
        database.pop_right("key"),
        Ok(Some("last".to_owned().into()))
    );
    assert_eq!(
        database.list_values("key"),
        Ok(Some(vec!["middle".to_owned().into()]))
    );
}

#[test]
fn pop_returns_none_for_missing_and_expired_keys() {
    let mut database = Database::new();

    assert_eq!(database.pop_left("missing"), Ok(None));
    assert_eq!(database.pop_right("missing"), Ok(None));

    database.set_list("expired".to_owned().into(), vec!["value".to_owned().into()]);
    assert!(database.expire("expired", 0));

    assert_eq!(database.pop_left("expired"), Ok(None));
    assert!(!database.exists("expired"));
}

#[test]
fn pop_rejects_strings_without_changing_value_or_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set("key".to_owned().into(), "value".to_owned().into());
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.pop_left("key"), Err(StoreError::WrongType));
    assert_eq!(database.pop_right("key"), Err(StoreError::WrongType));
    assert_eq!(database.get("key"), Ok(Some(b"value".as_slice())));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn pop_preserves_expiration_while_the_list_is_not_empty() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set_list(
        "key".to_owned().into(),
        vec!["first".to_owned().into(), "last".to_owned().into()],
    );
    assert!(database.expire_at("key", expires_at));

    assert_eq!(
        database.pop_left("key"),
        Ok(Some("first".to_owned().into()))
    );
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn pop_removes_the_key_after_the_last_value() {
    let mut database = Database::new();
    database.set_list("key".to_owned().into(), vec!["only".to_owned().into()]);
    assert!(database.expire("key", 60));

    assert_eq!(
        database.pop_right("key"),
        Ok(Some("only".to_owned().into()))
    );
    assert!(!database.exists("key"));
    assert_eq!(database.ttl("key"), -2);
}

fn database_with_range_values() -> Database {
    let mut database = Database::new();
    database.set_list(
        "key".to_owned().into(),
        vec![
            "zero".to_owned().into(),
            "one".to_owned().into(),
            "two".to_owned().into(),
            "three".to_owned().into(),
        ],
    );
    database
}

#[test]
fn list_range_uses_inclusive_indices_and_preserves_order() {
    let mut database = database_with_range_values();

    assert_eq!(
        database.list_range("key", 1, 2),
        Ok(vec!["one".to_owned().into(), "two".to_owned().into()])
    );
}

#[test]
fn list_range_supports_negative_and_out_of_bounds_indices() {
    let mut database = database_with_range_values();

    assert_eq!(
        database.list_range("key", -2, -1),
        Ok(vec!["two".to_owned().into(), "three".to_owned().into()])
    );
    assert_eq!(
        database.list_range("key", -100, 1),
        Ok(vec!["zero".to_owned().into(), "one".to_owned().into()])
    );
    assert_eq!(
        database.list_range("key", 2, 100),
        Ok(vec!["two".to_owned().into(), "three".to_owned().into()])
    );
}

#[test]
fn list_range_handles_empty_and_extreme_ranges() {
    let mut database = database_with_range_values();

    assert_eq!(database.list_range("key", 3, 1), Ok(Vec::new()));
    assert_eq!(database.list_range("key", 10, 20), Ok(Vec::new()));
    assert_eq!(database.list_range("key", i64::MIN, -1).unwrap().len(), 4);
    assert_eq!(
        database.list_range("key", i64::MAX, i64::MAX),
        Ok(Vec::new())
    );
}

#[test]
fn list_range_returns_empty_for_missing_and_expired_keys() {
    let mut database = Database::new();

    assert_eq!(database.list_range("missing", 0, -1), Ok(Vec::new()));

    database.set_list("expired".to_owned().into(), vec!["value".to_owned().into()]);
    assert!(database.expire("expired", 0));
    assert_eq!(database.list_range("expired", 0, -1), Ok(Vec::new()));
}

#[test]
fn list_range_rejects_strings_without_changing_value_or_expiration() {
    let mut database = Database::new();
    let expires_at = Instant::now() + Duration::from_secs(60);
    database.set("key".to_owned().into(), "value".to_owned().into());
    assert!(database.expire_at("key", expires_at));

    assert_eq!(
        database.list_range("key", 0, -1),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.get("key"), Ok(Some(b"value".as_slice())));
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn list_range_does_not_change_a_lists_expiration() {
    let mut database = database_with_range_values();
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("key", expires_at));

    assert_eq!(database.list_range("key", 0, 1).unwrap().len(), 2);
    assert_eq!(database.expiration("key"), Some(expires_at));
}

#[test]
fn variadic_pushes_follow_redis_argument_order_and_preserve_ttl() {
    let mut database = Database::new();
    assert_eq!(
        database.push_left_many("list", vec![b"one".to_vec(), b"two".to_vec()]),
        Ok(2)
    );
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("list", expires_at));
    assert_eq!(
        database.push_right_many("list", vec![b"three".to_vec(), b"four".to_vec()]),
        Ok(4)
    );
    assert_eq!(
        database.list_values("list"),
        Ok(Some(vec![
            b"two".to_vec(),
            b"one".to_vec(),
            b"three".to_vec(),
            b"four".to_vec()
        ]))
    );
    assert_eq!(database.expiration("list"), Some(expires_at));
}

#[test]
fn counted_pops_return_removal_order_preserve_ttl_and_remove_empty_keys() {
    let mut database = Database::new();
    database.set_list(
        b"list".to_vec(),
        vec![b"one".to_vec(), b"two".to_vec(), b"three".to_vec()],
    );
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("list", expires_at));

    assert_eq!(
        database.pop_right_many("list", 2),
        Ok(vec![b"three".to_vec(), b"two".to_vec()])
    );
    assert_eq!(database.expiration("list"), Some(expires_at));
    assert_eq!(
        database.pop_left_many("list", 10),
        Ok(vec![b"one".to_vec()])
    );
    assert!(!database.exists("list"));
    assert_eq!(database.pop_left_many("missing", 2), Ok(Vec::new()));
}

#[test]
fn zero_count_does_not_mutate_a_list_and_still_validates_type() {
    let mut database = Database::new();
    database.set_list(b"list".to_vec(), vec![b"one".to_vec()]);
    database.set(b"string".to_vec(), b"value".to_vec());

    assert_eq!(database.pop_left_many("list", 0), Ok(Vec::new()));
    assert_eq!(
        database.list_values("list"),
        Ok(Some(vec![b"one".to_vec()]))
    );
    assert_eq!(
        database.pop_right_many("string", 0),
        Err(StoreError::WrongType)
    );
}

#[test]
fn conditional_pushes_require_a_live_list_and_preserve_ttl() {
    let mut database = Database::new();
    assert_eq!(
        database.push_left_if_exists("missing", vec![b"value".to_vec()]),
        Ok(0)
    );
    assert!(!database.exists("missing"));

    database.set_list(b"list".to_vec(), vec![b"middle".to_vec()]);
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("list", expires_at));
    assert_eq!(
        database.push_left_if_exists("list", vec![b"one".to_vec(), b"two".to_vec()]),
        Ok(3)
    );
    assert_eq!(
        database.push_right_if_exists("list", vec![b"three".to_vec(), b"four".to_vec()]),
        Ok(5)
    );
    assert_eq!(
        database.list_values("list"),
        Ok(Some(vec![
            b"two".to_vec(),
            b"one".to_vec(),
            b"middle".to_vec(),
            b"three".to_vec(),
            b"four".to_vec(),
        ]))
    );
    assert_eq!(database.expiration("list"), Some(expires_at));

    database.set_list(b"expired".to_vec(), vec![b"old".to_vec()]);
    assert!(database.expire("expired", 0));
    assert_eq!(
        database.push_right_if_exists("expired", vec![b"new".to_vec()]),
        Ok(0)
    );
    assert!(!database.exists("expired"));
}

#[test]
fn conditional_pushes_reject_non_lists_without_mutation() {
    let mut database = Database::new();
    database.set(b"string".to_vec(), b"value".to_vec());
    assert_eq!(
        database.push_left_if_exists("string", vec![b"one".to_vec(), b"two".to_vec()]),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.get("string"), Ok(Some(b"value".as_slice())));
}

#[test]
fn list_index_supports_negative_indexes_and_missing_boundaries() {
    let mut database = Database::new();
    database.set_list(
        b"list".to_vec(),
        vec![b"zero".to_vec(), b"one".to_vec(), b"two".to_vec()],
    );
    assert_eq!(database.list_index("list", 0), Ok(Some(b"zero".to_vec())));
    assert_eq!(database.list_index("list", -1), Ok(Some(b"two".to_vec())));
    assert_eq!(database.list_index("list", -3), Ok(Some(b"zero".to_vec())));
    assert_eq!(database.list_index("list", 3), Ok(None));
    assert_eq!(database.list_index("list", -4), Ok(None));
    assert_eq!(database.list_index("missing", 0), Ok(None));
}

#[test]
fn list_set_validates_before_mutation_and_preserves_ttl() {
    let mut database = Database::new();
    database.set_list(
        b"list".to_vec(),
        vec![b"zero".to_vec(), b"one".to_vec(), b"two".to_vec()],
    );
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("list", expires_at));
    assert_eq!(database.list_set("list", -2, b"changed".to_vec()), Ok(()));
    assert_eq!(
        database.list_index("list", 1),
        Ok(Some(b"changed".to_vec()))
    );
    assert_eq!(database.expiration("list"), Some(expires_at));

    assert_eq!(
        database.list_set("list", 3, b"invalid".to_vec()),
        Err(StoreError::IndexOutOfRange)
    );
    assert_eq!(
        database.list_index("list", 1),
        Ok(Some(b"changed".to_vec()))
    );
    assert_eq!(
        database.list_set("missing", 0, b"invalid".to_vec()),
        Err(StoreError::NoSuchKey)
    );
}

#[test]
fn list_index_and_set_reject_wrong_types_without_mutation() {
    let mut database = Database::new();
    database.set(b"string".to_vec(), b"value".to_vec());
    assert_eq!(database.list_index("string", 0), Err(StoreError::WrongType));
    assert_eq!(
        database.list_set("string", 0, b"changed".to_vec()),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.get("string"), Ok(Some(b"value".as_slice())));
}
