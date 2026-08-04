use super::*;

#[test]
fn parses_set() {
    let command = Command::parse("SET name Ivan");

    assert_eq!(
        command,
        Ok(Command::Set {
            key: "name".to_owned(),
            value: "Ivan".to_owned(),
        })
    );
}

#[test]
fn parses_complex_string_set() {
    let command = Command::parse("SET name Ivan Gromov");

    assert_eq!(
        command,
        Ok(Command::Set {
            key: "name".to_owned(),
            value: "Ivan Gromov".to_owned(),
        })
    );
}

#[test]
fn commands_are_case_insensitive() {
    let command = Command::parse("get name");

    assert_eq!(
        command,
        Ok(Command::Get {
            key: "name".to_owned(),
        })
    );
}

#[test]
fn rejects_missing_argument() {
    let command = Command::parse("SET name");

    assert_eq!(
        command,
        Err(CommandError::InvalidArguments("SET key value"))
    );
}

#[test]
fn rejects_extra_argument() {
    let command = Command::parse("GET name extra");

    assert_eq!(command, Err(CommandError::InvalidArguments("GET key")));
}
