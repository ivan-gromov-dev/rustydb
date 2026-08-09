use super::super::stored_value::StoredValue as Entry;
use std::time::Instant;

#[test]
fn new_entry_has_no_expiration() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.expires_at(), None);
}

#[test]
fn new_entry_exposes_its_string_value() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.value(), "value");
}

#[test]
fn string_value_can_be_mutated_in_place() {
    let mut entry = Entry::new("value".to_owned());

    entry.value_mut().push_str(" appended");

    assert_eq!(entry.value(), "value appended");
}

#[test]
fn string_value_can_be_replaced() {
    let mut entry = Entry::new("old".to_owned());

    entry.set_value("new".to_owned());

    assert_eq!(entry.value(), "new");
}

#[test]
fn string_value_can_be_moved_out() {
    let entry = Entry::new("value".to_owned());

    assert_eq!(entry.into_value(), "value");
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
