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

#[test]
fn parses_expire() {
    let result = Command::parse("EXPIRE key 60");

    assert_eq!(
        result,
        Ok(Command::Expire {
            key: "key".to_owned(),
            seconds: 60,
        })
    );
}

#[test]
fn expire_requires_key_and_seconds() {
    assert_eq!(
        Command::parse("EXPIRE"),
        Err(CommandError::InvalidArguments("EXPIRE key seconds"))
    );

    assert_eq!(
        Command::parse("EXPIRE key"),
        Err(CommandError::InvalidArguments("EXPIRE key seconds"))
    );
}

#[test]
fn expire_rejects_invalid_seconds() {
    assert_eq!(
        Command::parse("EXPIRE key abc"),
        Err(CommandError::InvalidInteger("abc".to_owned()))
    );
}

#[test]
fn expire_rejects_negative_seconds() {
    assert_eq!(
        Command::parse("EXPIRE key -1"),
        Err(CommandError::InvalidInteger("-1".to_owned()))
    );
}

#[test]
fn expire_rejects_extra_arguments() {
    assert_eq!(
        Command::parse("EXPIRE key 10 extra"),
        Err(CommandError::InvalidArguments("EXPIRE key seconds"))
    );
}
