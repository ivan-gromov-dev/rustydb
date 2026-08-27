use super::CommandOutput;
use crate::command::{ProtocolVersion, command_metadata};

fn render(output: CommandOutput) -> String {
    let mut bytes = Vec::new();
    output.write_to(&mut bytes).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn renders_scalar_outputs() {
    assert_eq!(render(CommandOutput::Ok), "OK\n");
    assert_eq!(render(CommandOutput::Pong), "PONG\n");
    assert!(
        render(CommandOutput::Hello {
            protocol: Some(ProtocolVersion::Resp3),
            connection_id: Some(7),
        })
        .contains("proto:3\n")
    );
    assert_eq!(render(CommandOutput::Integer(-2)), "-2\n");
    assert_eq!(render(CommandOutput::Float(1.5)), "1.5\n");
    assert_eq!(
        render(CommandOutput::Value("sample".to_owned().into())),
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
fn renders_binary_values_without_utf8_conversion() {
    let mut bytes = Vec::new();
    CommandOutput::Value(b"a\0\xff".to_vec())
        .write_to(&mut bytes)
        .unwrap();

    assert_eq!(bytes, b"a\0\xff\n");
}

#[test]
fn renders_optional_values_and_keys() {
    assert_eq!(
        render(CommandOutput::OptionalValues(vec![
            Some("first".to_owned().into()),
            None,
            Some("third".to_owned().into()),
        ])),
        "first\n(nil)\nthird\n"
    );
    assert_eq!(render(CommandOutput::KeyList(Vec::new())), "(nil)\n");
    assert_eq!(
        render(CommandOutput::KeyList(vec![
            "first".to_owned().into(),
            "second".to_owned().into()
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
        "SADD",
        "SREM",
        "SISMEMBER",
        "SMEMBERS",
        "SCARD",
        "PING",
        "ECHO",
        "HELLO",
        "CLIENT ID",
        "CLIENT SETNAME",
        "CLIENT GETNAME",
        "CLIENT SETINFO",
        "COMMAND",
        "SELECT",
        "DBSIZE",
        "FLUSHDB",
        "FLUSHALL",
        "KEYS",
        "LEN",
        "CLEAR",
        "SAVE",
        "AOFREWRITE",
        "HELP",
        "EXIT",
    ] {
        assert!(help.lines().any(|line| line.trim().starts_with(command)));
    }
}

#[test]
fn renders_command_metadata_for_the_interactive_cli() {
    assert_eq!(
        render(CommandOutput::CommandMetadata(vec![
            command_metadata(b"GET"),
            None,
        ])),
        "get arity:2 flags:readonly,fast keys:1/1/1\n(nil)\n"
    );
}
