use super::super::in_memory::{InMemoryStore as Database, StoreError};
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
