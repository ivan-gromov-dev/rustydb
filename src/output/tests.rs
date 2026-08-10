use super::CommandOutput;

fn render(output: CommandOutput) -> String {
    let mut bytes = Vec::new();
    output.write_to(&mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn renders_scalar_outputs() {
    assert_eq!(render(CommandOutput::Ok), "OK\n");
    assert_eq!(render(CommandOutput::Integer(-2)), "-2\n");
    assert_eq!(render(CommandOutput::Float(1.5)), "1.5\n");
    assert_eq!(
        render(CommandOutput::Value("sample".to_owned())),
        "sample\n"
    );
    assert_eq!(render(CommandOutput::Nil), "(nil)\n");
    assert_eq!(
        render(CommandOutput::Error("failure".to_owned())),
        "ERR failure\n"
    );
    assert_eq!(render(CommandOutput::Exit), "");
}

#[test]
fn renders_optional_values_and_keys() {
    assert_eq!(
        render(CommandOutput::OptionalValues(vec![
            Some("first".to_owned()),
            None,
            Some("third".to_owned()),
        ])),
        "first\n(nil)\nthird\n"
    );
    assert_eq!(render(CommandOutput::KeyList(Vec::new())), "(nil)\n");
    assert_eq!(
        render(CommandOutput::KeyList(vec![
            "first".to_owned(),
            "second".to_owned()
        ])),
        "first\nsecond\n"
    );
}

#[test]
fn help_lists_every_supported_command() {
    let help = render(CommandOutput::Help);

    for command in [
        "SET",
        "MSET",
        "SETNX",
        "GET",
        "MGET",
        "GETSET",
        "GETDEL",
        "APPEND",
        "INCR",
        "INCRBY",
        "DECR",
        "DECRBY",
        "INCRBYFLOAT",
        "EXISTS",
        "DEL",
        "RENAME",
        "EXPIRE",
        "PEXPIRE",
        "TTL",
        "PTTL",
        "PERSIST",
        "STRLEN",
        "GETRANGE",
        "SETRANGE",
        "LPUSH",
        "RPUSH",
        "LLEN",
        "LPOP",
        "RPOP",
        "LRANGE",
        "KEYS",
        "LEN",
        "CLEAR",
        "HELP",
        "EXIT",
    ] {
        assert!(help.lines().any(|line| line.trim().starts_with(command)));
    }
}
