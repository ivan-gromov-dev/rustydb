use super::super::in_memory::{InMemoryStore as Database, StoreError};

#[test]
fn add_updates_scores_and_counts_only_new_members() {
    let mut database = Database::new();
    assert_eq!(
        database.sorted_set_add("board", vec![(10.0, b"b".to_vec()), (10.0, b"a".to_vec())]),
        Ok(2)
    );
    assert_eq!(
        database.sorted_set_add(
            "board",
            vec![
                (12.5, b"a".to_vec()),
                (8.0, b"c".to_vec()),
                (9.0, b"c".to_vec())
            ]
        ),
        Ok(1)
    );
    assert_eq!(database.sorted_set_score("board", b"a"), Ok(Some(12.5)));
    assert_eq!(database.sorted_set_score("board", b"c"), Ok(Some(9.0)));
    assert_eq!(database.sorted_set_cardinality("board"), Ok(3));
}

#[test]
fn remove_deletes_the_key_and_missing_reads_are_empty() {
    let mut database = Database::new();
    database
        .sorted_set_add("queue", vec![(1.0, b"job".to_vec())])
        .unwrap();
    assert_eq!(
        database.sorted_set_remove("queue", &[b"job".to_vec(), b"job".to_vec()]),
        Ok(1)
    );
    assert_eq!(database.type_name("queue"), "none");
    assert_eq!(database.sorted_set_score("missing", b"job"), Ok(None));
    assert_eq!(database.sorted_set_cardinality("missing"), Ok(0));
}

#[test]
fn invalid_scores_and_wrong_types_do_not_mutate() {
    let mut database = Database::new();
    database.set(b"value".to_vec(), b"text".to_vec());
    assert_eq!(
        database.sorted_set_add("board", vec![(f64::INFINITY, b"x".to_vec())]),
        Err(StoreError::FloatIsNotFinite)
    );
    assert_eq!(database.type_name("board"), "none");
    assert_eq!(
        database.sorted_set_add("value", vec![(1.0, b"x".to_vec())]),
        Err(StoreError::WrongType)
    );
    assert_eq!(database.get("value"), Ok(Some(b"text".as_slice())));
}
