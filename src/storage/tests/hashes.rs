use super::super::in_memory::{InMemoryStore as Database, StoreError};
use std::time::{Duration, Instant};

#[test]
fn hashes_set_get_and_iterate_fields_deterministically() {
    let mut database = Database::new();
    assert_eq!(
        database.hash_set(
            "hash",
            vec![
                (b"z".to_vec(), b"1".to_vec()),
                (b"a".to_vec(), b"2".to_vec())
            ]
        ),
        Ok(2)
    );
    assert_eq!(
        database.hash_set("hash", vec![(b"z".to_vec(), b"3".to_vec())]),
        Ok(0)
    );
    assert_eq!(database.hash_get("hash", "z"), Ok(Some(b"3".to_vec())));
    assert_eq!(
        database.hash_get_many("hash", &[b"a".to_vec(), b"missing".to_vec()]),
        Ok(vec![Some(b"2".to_vec()), None])
    );
    assert_eq!(
        database.hash_entries("hash"),
        Ok(vec![
            (b"a".to_vec(), b"2".to_vec()),
            (b"z".to_vec(), b"3".to_vec())
        ])
    );
}

#[test]
fn hash_setnx_delete_and_empty_hash_semantics() {
    let mut database = Database::new();
    assert_eq!(
        database.hash_set_if_absent("hash", b"field".to_vec(), b"one".to_vec()),
        Ok(true)
    );
    assert_eq!(
        database.hash_set_if_absent("hash", b"field".to_vec(), b"two".to_vec()),
        Ok(false)
    );
    assert_eq!(database.hash_contains("hash", "field"), Ok(true));
    assert_eq!(database.hash_length("hash"), Ok(1));
    assert_eq!(
        database.hash_delete("hash", &[b"field".to_vec(), b"field".to_vec()]),
        Ok(1)
    );
    assert!(!database.exists("hash"));
    assert_eq!(database.hash_length("missing"), Ok(0));
}

#[test]
fn hash_mutations_preserve_ttl_and_expired_hashes_are_missing() {
    let mut database = Database::new();
    database
        .hash_set(
            "hash",
            vec![
                (b"one".to_vec(), b"1".to_vec()),
                (b"two".to_vec(), b"2".to_vec()),
            ],
        )
        .unwrap();
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("hash", expires_at));
    database
        .hash_set("hash", vec![(b"three".to_vec(), b"3".to_vec())])
        .unwrap();
    database.hash_delete("hash", &[b"one".to_vec()]).unwrap();
    assert_eq!(database.expiration("hash"), Some(expires_at));
    assert!(database.expire("hash", 0));
    assert_eq!(database.hash_get("hash", "two"), Ok(None));
}

#[test]
fn hash_commands_reject_wrong_types_without_mutation() {
    let mut database = Database::new();
    database.set(b"string".to_vec(), b"value".to_vec());
    assert_eq!(
        database.hash_set("string", vec![(b"field".to_vec(), b"value".to_vec())]),
        Err(StoreError::WrongType)
    );
    assert_eq!(
        database.hash_get("string", "field"),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.get("string"), Ok(Some(b"value".as_slice())));
}

#[test]
fn hash_numeric_mutations_validate_before_mutation_and_preserve_ttl() {
    let mut database = Database::new();
    database
        .hash_set(
            "hash",
            vec![
                (b"integer".to_vec(), i64::MAX.to_string().into_bytes()),
                (b"float".to_vec(), b"1.5".to_vec()),
                (b"invalid".to_vec(), b"nope".to_vec()),
            ],
        )
        .unwrap();
    let expires_at = Instant::now() + Duration::from_secs(60);
    assert!(database.expire_at("hash", expires_at));
    assert_eq!(
        database.hash_increment_by("hash", b"integer".to_vec(), 1),
        Err(StoreError::IntegerOverflow)
    );
    assert_eq!(
        database.hash_increment_by("hash", b"invalid".to_vec(), 1),
        Err(StoreError::ValueIsNotInteger)
    );
    assert_eq!(
        database.hash_increment_by_float("hash", b"float".to_vec(), 0.25),
        Ok(1.75)
    );
    assert_eq!(
        database.hash_increment_by_float("hash", b"invalid".to_vec(), 1.0),
        Err(StoreError::ValueIsNotFloat)
    );
    assert_eq!(database.expiration("hash"), Some(expires_at));
    assert_eq!(
        database.hash_get("hash", "integer"),
        Ok(Some(i64::MAX.to_string().into_bytes()))
    );
}

#[test]
fn hash_keys_values_and_scan_follow_sorted_field_order() {
    let mut database = Database::new();
    database
        .hash_set(
            "hash",
            vec![
                (b"z:2".to_vec(), b"last".to_vec()),
                (b"a:1".to_vec(), b"first".to_vec()),
                (b"m:1".to_vec(), b"middle".to_vec()),
            ],
        )
        .unwrap();
    assert_eq!(
        database.hash_keys("hash"),
        Ok(vec![b"a:1".to_vec(), b"m:1".to_vec(), b"z:2".to_vec()])
    );
    assert_eq!(
        database.hash_values("hash"),
        Ok(vec![
            b"first".to_vec(),
            b"middle".to_vec(),
            b"last".to_vec()
        ])
    );
    assert_eq!(
        database.hash_scan("hash", 0, Some(b"*:1"), 2),
        Ok((
            2,
            vec![
                (b"a:1".to_vec(), b"first".to_vec()),
                (b"m:1".to_vec(), b"middle".to_vec())
            ]
        ))
    );
    assert_eq!(
        database.hash_scan("hash", 2, Some(b"*:1"), 2),
        Ok((0, Vec::new()))
    );
    assert_eq!(database.hash_scan("hash", 99, None, 2), Ok((0, Vec::new())));
}
