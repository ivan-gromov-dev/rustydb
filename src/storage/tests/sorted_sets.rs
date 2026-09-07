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

#[test]
fn sorted_set_ranks_order_scores_then_binary_members_and_follow_updates() {
    let mut database = Database::new();
    let members = [
        b"low".to_vec(),
        vec![],
        vec![0],
        vec![255],
        b"high".to_vec(),
    ];
    database
        .sorted_set_add(
            "board",
            vec![
                (f64::MAX, members[4].clone()),
                (0.0, members[3].clone()),
                (-0.0, members[1].clone()),
                (-f64::MAX, members[0].clone()),
                (0.0, members[2].clone()),
            ],
        )
        .unwrap();
    for (rank, member) in members.iter().enumerate() {
        assert_eq!(
            database.sorted_set_rank("board", member, false),
            Ok(Some(rank))
        );
        assert_eq!(
            database.sorted_set_rank("board", member, true),
            Ok(Some(4 - rank))
        );
    }
    for reverse in [false, true] {
        assert_eq!(
            database.sorted_set_rank("board", "absent", reverse),
            Ok(None)
        );
        assert_eq!(
            database.sorted_set_rank("missing", "absent", reverse),
            Ok(None)
        );
    }
    database
        .sorted_set_add("board", vec![(1.5, vec![])])
        .unwrap();
    assert_eq!(database.sorted_set_rank("board", [], false), Ok(Some(3)));
    database
        .sorted_set_remove("board", &[b"low".to_vec()])
        .unwrap();
    assert_eq!(database.sorted_set_rank("board", [], false), Ok(Some(2)));
}

#[test]
fn sorted_set_stage_two_ranges_counts_and_ordered_scores() {
    use crate::storage::ScoreBound::*;
    let mut db = Database::new();
    db.sorted_set_add(
        "k",
        vec![
            (2.0, vec![255]),
            (2.0, vec![]),
            (-1.5, b"low".to_vec()),
            (2.0, vec![0]),
        ],
    )
    .unwrap();
    assert_eq!(
        db.sorted_set_scores("k", &[vec![255], b"absent".to_vec(), vec![], vec![255]]),
        Ok(vec![Some(2.0), None, Some(2.0), Some(2.0)])
    );
    assert_eq!(
        db.sorted_set_scores("missing", &[vec![], vec![]]),
        Ok(vec![None, None])
    );
    let expected = vec![
        (b"low".to_vec(), -1.5),
        (vec![], 2.0),
        (vec![0], 2.0),
        (vec![255], 2.0),
    ];
    assert_eq!(
        db.sorted_set_range("k", i64::MIN, i64::MAX, false),
        Ok(expected.clone())
    );
    let reversed: Vec<_> = expected.iter().rev().cloned().collect();
    assert_eq!(db.sorted_set_range("k", 0, -1, true), Ok(reversed.clone()));
    assert_eq!(
        db.sorted_set_range("k", -2, -1, true),
        Ok(reversed[2..].to_vec())
    );
    assert_eq!(
        db.sorted_set_range("k", 1, 2, false),
        Ok(expected[1..3].to_vec())
    );
    for (start, stop) in [
        (3, 2),
        (4, 8),
        (0, -5),
        (i64::MAX, i64::MAX),
        (i64::MIN, i64::MIN),
    ] {
        assert_eq!(db.sorted_set_range("k", start, stop, false), Ok(vec![]));
    }
    assert_eq!(db.sorted_set_range("missing", 0, -1, false), Ok(vec![]));
    for (min, max, count) in [
        (NegativeInfinity, PositiveInfinity, 4),
        (Inclusive(2.0), Inclusive(2.0), 3),
        (Exclusive(2.0), PositiveInfinity, 0),
        (NegativeInfinity, Exclusive(2.0), 1),
        (Exclusive(-1.5), Inclusive(2.0), 3),
        (Inclusive(-1.5), Exclusive(2.0), 1),
        (Inclusive(3.0), Inclusive(2.0), 0),
        (PositiveInfinity, PositiveInfinity, 0),
        (NegativeInfinity, NegativeInfinity, 0),
    ] {
        assert_eq!(db.sorted_set_count("k", min, max), Ok(count));
    }
    assert_eq!(
        db.sorted_set_count("missing", NegativeInfinity, PositiveInfinity),
        Ok(0)
    );
}

#[test]
fn sorted_set_increment_validates_before_mutation_or_eviction() {
    let mut db = Database::with_max_keys(Some(1));
    db.set(b"old".to_vec(), b"value".to_vec());
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            db.sorted_set_increment("new", vec![], invalid),
            Err(StoreError::FloatIsNotFinite)
        );
        assert_eq!(db.get("old"), Ok(Some(b"value".as_slice())));
    }
    assert_eq!(db.sorted_set_increment("new", vec![], -0.0), Ok(0.0));
    assert_eq!(
        db.sorted_set_score("new", []).unwrap().unwrap().to_bits(),
        0.0f64.to_bits()
    );
    assert_eq!(db.get("old"), Ok(None));
    assert_eq!(db.sorted_set_increment("new", vec![], 1.25), Ok(1.25));
    assert_eq!(db.sorted_set_increment("new", vec![], -2.5), Ok(-1.25));
    assert_eq!(
        db.sorted_set_increment("new", b"large".to_vec(), f64::MAX),
        Ok(f64::MAX)
    );
    assert_eq!(
        db.sorted_set_increment("new", b"large".to_vec(), f64::MAX),
        Err(StoreError::FloatIsNotFinite)
    );
    assert_eq!(db.sorted_set_score("new", "large"), Ok(Some(f64::MAX)));
    assert_eq!(db.sorted_set_cardinality("new"), Ok(2));
}

#[test]
fn sorted_set_score_ranges_filter_before_pagination_and_reverse_ties() {
    use crate::storage::ScoreBound::*;
    let mut db = Database::new();
    db.sorted_set_add(
        "k",
        vec![
            (-2.0, b"low".to_vec()),
            (1.0, vec![]),
            (1.0, vec![0]),
            (1.0, vec![255]),
            (3.0, b"high".to_vec()),
        ],
    )
    .unwrap();
    assert_eq!(
        db.sorted_set_score_range("k", Exclusive(-2.0), Exclusive(3.0), false, Some((1, 1))),
        Ok(vec![(vec![0], 1.0)])
    );
    assert_eq!(
        db.sorted_set_score_range("k", Inclusive(1.0), Inclusive(1.0), true, Some((1, -2))),
        Ok(vec![(vec![0], 1.0), (vec![], 1.0)])
    );
    assert_eq!(
        db.sorted_set_score_range("k", NegativeInfinity, PositiveInfinity, false, None)
            .unwrap()
            .len(),
        5
    );
    for limit in [Some((0, 0)), Some((usize::MAX, -1)), Some((3, 2))] {
        assert_eq!(
            db.sorted_set_score_range("k", Inclusive(1.0), Inclusive(1.0), false, limit),
            Ok(vec![])
        );
    }
    for (min, max) in [
        (Inclusive(2.0), Inclusive(1.0)),
        (Exclusive(1.0), Inclusive(1.0)),
        (PositiveInfinity, PositiveInfinity),
        (NegativeInfinity, NegativeInfinity),
    ] {
        assert_eq!(
            db.sorted_set_score_range("k", min, max, false, None),
            Ok(vec![])
        );
    }
    assert_eq!(
        db.sorted_set_score_range("missing", NegativeInfinity, PositiveInfinity, false, None),
        Ok(vec![])
    );
}

#[test]
fn sorted_set_pops_and_removals_obey_order_and_delete_final_keys() {
    use crate::storage::ScoreBound::*;
    for reverse in [false, true] {
        let mut db = Database::new();
        db.sorted_set_add("k", vec![(1.0, vec![]), (1.0, vec![0]), (1.0, vec![255])])
            .unwrap();
        assert_eq!(db.sorted_set_pop("k", 0, reverse), Ok(vec![]));
        let expected = if reverse { vec![255] } else { vec![] };
        assert_eq!(
            db.sorted_set_pop("k", 1, reverse),
            Ok(vec![(expected, 1.0)])
        );
        let expected = if reverse {
            vec![(vec![0], 1.0), (vec![], 1.0)]
        } else {
            vec![(vec![0], 1.0), (vec![255], 1.0)]
        };
        assert_eq!(db.sorted_set_pop("k", usize::MAX, reverse), Ok(expected));
        assert_eq!(db.type_name("k"), "none");
        assert_eq!(db.sorted_set_pop("k", 1, reverse), Ok(vec![]));
    }
    let mut db = Database::new();
    db.sorted_set_add(
        "k",
        vec![
            (1.0, b"a".to_vec()),
            (2.0, b"b".to_vec()),
            (2.0, b"c".to_vec()),
            (3.0, b"d".to_vec()),
        ],
    )
    .unwrap();
    assert_eq!(db.sorted_set_remove_rank_range("k", 3, 0), Ok(0));
    assert_eq!(db.sorted_set_remove_rank_range("k", -2, -1), Ok(2));
    assert_eq!(db.sorted_set_score("k", "b"), Ok(Some(2.0)));
    assert_eq!(
        db.sorted_set_remove_score_range("k", Exclusive(2.0), PositiveInfinity),
        Ok(0)
    );
    assert_eq!(
        db.sorted_set_remove_score_range("k", Inclusive(2.0), Inclusive(2.0)),
        Ok(1)
    );
    assert_eq!(
        db.sorted_set_remove_score_range("k", NegativeInfinity, PositiveInfinity),
        Ok(1)
    );
    assert_eq!(db.type_name("k"), "none");
    assert_eq!(
        db.sorted_set_remove_score_range("missing", NegativeInfinity, PositiveInfinity),
        Ok(0)
    );
    assert_eq!(
        db.sorted_set_remove_rank_range("missing", i64::MIN, i64::MAX),
        Ok(0)
    );
    db.sorted_set_add("k", vec![(1.0, vec![])]).unwrap();
    assert_eq!(
        db.sorted_set_remove_rank_range("k", i64::MIN, i64::MAX),
        Ok(1)
    );
    assert_eq!(db.type_name("k"), "none");
}

#[test]
fn sorted_set_scan_uses_member_order_and_examined_count() {
    let mut db = Database::new();
    db.sorted_set_add("k", vec![(3.0, vec![]), (2.0, vec![0]), (1.0, vec![255])])
        .unwrap();
    assert_eq!(
        db.sorted_set_scan("k", 0, None, 2),
        Ok((2, vec![(vec![], 3.0), (vec![0], 2.0)]))
    );
    assert_eq!(
        db.sorted_set_scan("k", 0, Some(b"\xff*"), 2),
        Ok((2, vec![]))
    );
    assert_eq!(
        db.sorted_set_scan("k", 2, Some(b"\xff*"), 2),
        Ok((0, vec![(vec![255], 1.0)]))
    );
    assert_eq!(
        db.sorted_set_scan("k", 1, None, usize::MAX),
        Ok((0, vec![(vec![0], 2.0), (vec![255], 1.0)]))
    );
    assert_eq!(
        db.sorted_set_scan("k", usize::MAX, None, 1),
        Ok((0, vec![]))
    );
    assert_eq!(db.sorted_set_scan("missing", 0, None, 1), Ok((0, vec![])));
    db.sorted_set_remove("k", &[vec![0]]).unwrap();
    assert_eq!(db.sorted_set_scan("k", 2, None, 1), Ok((0, vec![])));
}
