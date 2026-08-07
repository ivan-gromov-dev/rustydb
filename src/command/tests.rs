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

#[test]
fn parses_increment_by_float() {
    let result = Command::parse("INCRBYFLOAT counter 1.5");

    assert_eq!(
        result,
        Ok(Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: 1.5,
        })
    );
}

#[test]
fn parses_increment_by_float_with_negative_amount() {
    let result = Command::parse("INCRBYFLOAT counter -2.25");

    assert_eq!(
        result,
        Ok(Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: -2.25,
        })
    );
}

#[test]
fn parses_increment_by_float_case_insensitively() {
    let result = Command::parse("incrbyfloat counter 1.5");

    assert_eq!(
        result,
        Ok(Command::IncrementByFloat {
            key: "counter".to_owned(),
            amount: 1.5,
        })
    );
}

#[test]
fn increment_by_float_requires_key() {
    let result = Command::parse("INCRBYFLOAT");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("INCRBYFLOAT key amount"))
    );
}

#[test]
fn increment_by_float_requires_amount() {
    let result = Command::parse("INCRBYFLOAT counter");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("INCRBYFLOAT key amount"))
    );
}

#[test]
fn increment_by_float_rejects_invalid_float() {
    let result = Command::parse("INCRBYFLOAT counter abc");

    assert_eq!(result, Err(CommandError::InvalidFloat("abc".to_owned())));
}

#[test]
fn increment_by_float_rejects_nan() {
    let result = Command::parse("INCRBYFLOAT counter NaN");

    assert_eq!(result, Err(CommandError::InvalidFloat("NaN".to_owned())));
}

#[test]
fn increment_by_float_rejects_infinity() {
    let result = Command::parse("INCRBYFLOAT counter inf");

    assert_eq!(result, Err(CommandError::InvalidFloat("inf".to_owned())));
}

#[test]
fn increment_by_float_rejects_extra_arguments() {
    let result = Command::parse("INCRBYFLOAT counter 1.5 extra");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("INCRBYFLOAT key amount"))
    );
}

#[test]
fn parses_delete_with_single_key() {
    let result = Command::parse("DEL a");

    assert_eq!(
        result,
        Ok(Command::Delete {
            keys: vec!["a".to_owned()],
        })
    );
}

#[test]
fn parses_delete_with_multiple_keys() {
    let result = Command::parse("DEL a b c");

    assert_eq!(
        result,
        Ok(Command::Delete {
            keys: vec!["a".to_owned(), "b".to_owned(), "c".to_owned(),],
        })
    );
}

#[test]
fn delete_requires_at_least_one_key() {
    let result = Command::parse("DEL");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("DEL key [key ...]"))
    );
}

#[test]
fn parses_exists_with_single_key() {
    let result = Command::parse("EXISTS a");

    assert_eq!(
        result,
        Ok(Command::Exists {
            keys: vec!["a".to_owned()],
        })
    );
}

#[test]
fn parses_exists_with_multiple_keys() {
    let result = Command::parse("EXISTS a b c");

    assert_eq!(
        result,
        Ok(Command::Exists {
            keys: vec!["a".to_owned(), "b".to_owned(), "c".to_owned(),],
        })
    );
}

#[test]
fn exists_requires_at_least_one_key() {
    let result = Command::parse("EXISTS");

    assert_eq!(
        result,
        Err(CommandError::InvalidArguments("EXISTS key [key ...]"))
    );
}
