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
