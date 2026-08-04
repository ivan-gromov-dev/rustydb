use super::*;

#[test]
fn set_and_get_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    assert_eq!(database.get("name"), Some("Ivan"));
}

#[test]
fn set_overwrites_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("name".to_owned(), "Alex".to_owned());

    assert_eq!(database.get("name"), Some("Alex"));
    assert_eq!(database.len(), 1);
}

#[test]
fn delete_value() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());

    assert!(database.delete("name"));
    assert!(!database.exists("name"));
}

#[test]
fn clear() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("surname".to_owned(), "Gromov".to_owned());

    database.clear();

    assert_eq!(database.len(), 0);
}

#[test]
fn get_keys() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Ivan".to_owned());
    database.set("surname".to_owned(), "Gromov".to_owned());

    assert_eq!(database.keys(), ["name", "surname"])
}

#[test]
fn delete_missing_value_returns_false() {
    let mut database = Database::new();

    assert!(!database.delete("missing"));
}

#[test]
fn rename_key() {
    let mut database = Database::new();

    database.set("name".to_owned(), "Gromov".to_owned());

    assert!(database.rename_key("name", "surname".to_owned()))
}
