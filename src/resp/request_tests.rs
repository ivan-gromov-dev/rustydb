use crate::command::{Command, CommandError};

use super::decoder::{DecodeLimits, DecodeResult, decode};
use super::frame::RespFrame;
use super::request::{RequestError, command_from_frame};

fn bulk(value: &[u8]) -> RespFrame {
    RespFrame::BulkString(value.to_vec())
}

#[test]
fn converts_bulk_string_arrays_to_typed_commands() {
    assert_eq!(
        command_from_frame(RespFrame::Array(vec![
            bulk(b"SET"),
            bulk(b"binary\xff-key"),
            bulk(b"line one\r\nline two\0\xff"),
        ])),
        Ok(Command::Set {
            key: b"binary\xff-key".to_vec(),
            value: b"line one\r\nline two\0\xff".to_vec(),
        })
    );
    assert_eq!(
        command_from_frame(RespFrame::Array(vec![bulk(b"SAVE")])),
        Ok(Command::Save)
    );
    assert_eq!(
        command_from_frame(RespFrame::Array(vec![bulk(b"AOFREWRITE")])),
        Ok(Command::AofRewrite)
    );
}

#[test]
fn converts_decoded_wire_bytes_without_losing_binary_data() {
    let input = b"*3\r\n$3\r\nSET\r\n$5\r\nkey\0\xff\r\n$7\r\na b\r\n\0\xff\r\n";
    let DecodeResult::Complete { frame, consumed } =
        decode(input, DecodeLimits::default()).unwrap()
    else {
        panic!("expected a complete request");
    };

    assert_eq!(consumed, input.len());
    assert_eq!(
        command_from_frame(frame),
        Ok(Command::Set {
            key: b"key\0\xff".to_vec(),
            value: b"a b\r\n\0\xff".to_vec(),
        })
    );
}

#[test]
fn delegates_command_validation_to_the_command_parser() {
    assert_eq!(
        command_from_frame(RespFrame::Array(vec![bulk(b"GET")])),
        Err(RequestError::InvalidCommand(
            CommandError::InvalidArguments("GET key")
        ))
    );
    assert_eq!(
        command_from_frame(RespFrame::Array(vec![bulk(b"NOPE")])),
        Err(RequestError::InvalidCommand(CommandError::UnknownCommand(
            "NOPE".to_owned()
        )))
    );
}

#[test]
fn rejects_non_array_and_empty_requests() {
    assert_eq!(
        command_from_frame(bulk(b"GET")),
        Err(RequestError::ExpectedArray)
    );
    assert_eq!(
        command_from_frame(RespFrame::NullArray),
        Err(RequestError::ExpectedArray)
    );
    assert_eq!(
        command_from_frame(RespFrame::Array(Vec::new())),
        Err(RequestError::EmptyArray)
    );
}

#[test]
fn rejects_every_non_bulk_array_element_with_its_index() {
    for element in [
        RespFrame::SimpleString("key".to_owned()),
        RespFrame::Error("ERR".to_owned()),
        RespFrame::Integer(1),
        RespFrame::Array(Vec::new()),
        RespFrame::NullBulkString,
        RespFrame::NullArray,
    ] {
        assert_eq!(
            command_from_frame(RespFrame::Array(vec![bulk(b"GET"), element])),
            Err(RequestError::ExpectedBulkString { index: 1 })
        );
    }
}

#[test]
fn request_errors_have_stable_messages() {
    assert_eq!(
        RequestError::ExpectedArray.to_string(),
        "expected a RESP array command"
    );
    assert_eq!(
        RequestError::EmptyArray.to_string(),
        "command array cannot be empty"
    );
    assert_eq!(
        RequestError::ExpectedBulkString { index: 2 }.to_string(),
        "command argument 2 must be a bulk string"
    );
    assert_eq!(
        RequestError::InvalidCommand(CommandError::InvalidInteger("x".to_owned())).to_string(),
        "invalid integer: x"
    );
}
