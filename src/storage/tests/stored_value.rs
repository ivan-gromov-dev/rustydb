use super::super::in_memory::StoreError;
use super::super::stored_value::StoredValue as Entry;
use std::time::Instant;

#[test]
fn wrong_type_error_has_a_stable_message() {
    assert_eq!(
        StoreError::WrongType.to_string(),
        "operation against a key holding the wrong kind of value"
    );
}

#[test]
fn new_entry_has_no_expiration() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.expires_at(), None);
}

#[test]
fn new_entry_exposes_its_string_value() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.value(), Ok("value"));
}

#[test]
fn string_value_can_be_mutated_in_place() {
    let mut entry = Entry::new("value".to_owned());

    entry
        .value_mut()
        .expect("a string entry should expose its mutable string")
        .push_str(" appended");

    assert_eq!(entry.value(), Ok("value appended"));
}

#[test]
fn string_value_can_be_replaced() {
    let mut entry = Entry::new("old".to_owned());

    entry.set_value("new".to_owned());

    assert_eq!(entry.value(), Ok("new"));
}

#[test]
fn string_value_can_be_moved_out() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.into_value(), Ok("value".to_owned()));
}

#[test]
fn list_rejects_immutable_string_access() {
    let entry = Entry::new_list();

    assert_eq!(entry.value(), Err(StoreError::WrongType));
}

#[test]
fn list_rejects_mutable_string_access() {
    let mut entry = Entry::new_list();

    assert_eq!(entry.value_mut(), Err(StoreError::WrongType));
}

#[test]
fn list_rejects_string_extraction() {
    let entry = Entry::new_list();

    assert_eq!(entry.into_value(), Err(StoreError::WrongType));
}

#[test]
fn set_exposes_only_set_access() {
    let mut entry = Entry::new_set();

    assert_eq!(entry.value(), Err(StoreError::WrongType));
    assert_eq!(entry.list(), Err(StoreError::WrongType));
    assert!(
        entry
            .set_mut()
            .expect("a set entry should expose its set")
            .insert("member".to_owned())
    );
    assert!(
        entry
            .set()
            .expect("a set entry should expose its set")
            .contains("member")
    );
}

#[test]
fn entry_without_expiration_is_not_expired() {
    let entry = Entry::new("value".to_owned());

    assert!(!entry.is_expired(Instant::now()));
}

#[test]
fn entry_is_expired_after_expiration_time() {
    let now = Instant::now();

    let mut entry = Entry::new("value".to_owned());
    entry.set_expires_at(now);

    assert!(entry.is_expired(now));
}
