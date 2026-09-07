use super::{CommandOutput, PubSubAck};
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
    assert_eq!(render(CommandOutput::SimpleString("string")), "string\n");
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
fn renders_integer_lists() {
    assert_eq!(render(CommandOutput::IntegerList(vec![3, 1])), "3\n1\n");
    assert_eq!(render(CommandOutput::IntegerList(Vec::new())), "(nil)\n");
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
fn renders_scan_cursor_before_keys() {
    assert_eq!(
        render(CommandOutput::Scan {
            cursor: 2,
            keys: vec![b"alpha".to_vec(), b"beta".to_vec()]
        }),
        "2\nalpha\nbeta\n"
    );
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

#[test]
fn renders_sorted_set_scores_and_member_pairs() {
    assert_eq!(render(CommandOutput::Score(Some(1.5))), "1.5\n");
    assert_eq!(render(CommandOutput::Score(None)), "(nil)\n");
    assert_eq!(
        render(CommandOutput::Scores(vec![Some(1.5), None, Some(-2.0)])),
        "1.5\n(nil)\n-2\n"
    );
    assert_eq!(render(CommandOutput::ScoredMembers(vec![])), "(nil)\n");
    let mut bytes = vec![];
    CommandOutput::ScoredMembers(vec![(b"\xff\0".to_vec(), 1.5)])
        .write_to(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"\xff\0\n1.5\n");
}

#[test]
fn renders_single_sorted_set_pop() {
    assert_eq!(render(CommandOutput::PoppedMember(None)), "(nil)\n");
    assert_eq!(
        render(CommandOutput::PoppedMember(Some((b"job".to_vec(), 1.5)))),
        "job\n1.5\n"
    );
}

#[test]
fn renders_sorted_set_scan_cursor_and_pairs() {
    assert_eq!(
        render(CommandOutput::SortedSetScan {
            cursor: 2,
            entries: vec![]
        }),
        "2\n"
    );
    assert_eq!(
        render(CommandOutput::SortedSetScan {
            cursor: 0,
            entries: vec![(b"a".to_vec(), 1.5)]
        }),
        "0\na\n1.5\n"
    );
}

#[test]
fn renders_pubsub_outputs_for_the_interactive_boundary() {
    assert_eq!(
        render(CommandOutput::PubSubAcks(vec![
            PubSubAck {
                subscribed: true,
                pattern: false,
                channel: Some(b"direct".to_vec()),
                count: 2,
            },
            PubSubAck {
                subscribed: false,
                pattern: false,
                channel: None,
                count: 1,
            },
            PubSubAck {
                subscribed: true,
                pattern: true,
                channel: Some(b"news:*".to_vec()),
                count: 2,
            },
            PubSubAck {
                subscribed: false,
                pattern: true,
                channel: Some(b"news:*".to_vec()),
                count: 1,
            },
        ])),
        "subscribe direct 2\nunsubscribe (nil) 1\npsubscribe news:* 2\npunsubscribe news:* 1\n"
    );

    let mut bytes = Vec::new();
    CommandOutput::PubSubMessage {
        channel: b"channel\0\xff".to_vec(),
        message: b"message\0\xff".to_vec(),
    }
    .write_to(&mut bytes)
    .unwrap();
    assert_eq!(bytes, b"message channel\0\xff message\0\xff\n");

    let mut bytes = Vec::new();
    CommandOutput::PubSubPatternMessage {
        pattern: b"channel*".to_vec(),
        channel: b"channel\0\xff".to_vec(),
        message: b"message\0\xff".to_vec(),
    }
    .write_to(&mut bytes)
    .unwrap();
    assert_eq!(bytes, b"pmessage channel* channel\0\xff message\0\xff\n");

    assert_eq!(render(CommandOutput::PubSubPong(None)), "pong\n");
    let mut bytes = Vec::new();
    CommandOutput::PubSubPong(Some(b"binary\0\xff".to_vec()))
        .write_to(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"pong binary\0\xff\n");
}
