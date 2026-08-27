use crate::command::ProtocolVersion;
use crate::output::{CommandOutput, HELP_TEXT};

use super::frame::RespFrame;
use super::response::{frame_from_output, frame_from_output_for_protocol};

#[test]
fn converts_scalar_outputs() {
    for (output, expected) in [
        (CommandOutput::Ok, RespFrame::SimpleString("OK".to_owned())),
        (
            CommandOutput::Pong,
            RespFrame::SimpleString("PONG".to_owned()),
        ),
        (CommandOutput::Integer(-2), RespFrame::Integer(-2)),
        (
            CommandOutput::SimpleString("string"),
            RespFrame::SimpleString("string".to_owned()),
        ),
        (
            CommandOutput::Float(1.5),
            RespFrame::BulkString(b"1.5".to_vec()),
        ),
        (
            CommandOutput::Value(b"a b\r\n\0\xff".to_vec()),
            RespFrame::BulkString(b"a b\r\n\0\xff".to_vec()),
        ),
        (CommandOutput::Nil, RespFrame::NullBulkString),
        (
            CommandOutput::Exit,
            RespFrame::SimpleString("OK".to_owned()),
        ),
    ] {
        assert_eq!(frame_from_output(output), expected);
    }
}

#[test]
fn converts_optional_values_to_an_array_with_null_bulk_strings() {
    assert_eq!(
        frame_from_output(CommandOutput::OptionalValues(vec![
            Some(b"first".to_vec()),
            None,
            Some(b"third\0\xff".to_vec()),
        ])),
        RespFrame::Array(vec![
            RespFrame::BulkString(b"first".to_vec()),
            RespFrame::NullBulkString,
            RespFrame::BulkString(b"third\0\xff".to_vec()),
        ])
    );
}

#[test]
fn converts_key_lists_to_arrays_including_the_empty_list() {
    assert_eq!(
        frame_from_output(CommandOutput::KeyList(Vec::new())),
        RespFrame::Array(Vec::new())
    );
    assert_eq!(
        frame_from_output(CommandOutput::KeyList(vec![
            b"alpha".to_vec(),
            b"binary\xff".to_vec(),
        ])),
        RespFrame::Array(vec![
            RespFrame::BulkString(b"alpha".to_vec()),
            RespFrame::BulkString(b"binary\xff".to_vec()),
        ])
    );
}

#[test]
fn converts_scan_to_cursor_and_nested_key_array() {
    assert_eq!(
        frame_from_output(CommandOutput::Scan {
            cursor: 12,
            keys: vec![b"a".to_vec(), b"b\xff".to_vec()]
        }),
        RespFrame::Array(vec![
            RespFrame::BulkString(b"12".to_vec()),
            RespFrame::Array(vec![
                RespFrame::BulkString(b"a".to_vec()),
                RespFrame::BulkString(b"b\xff".to_vec())
            ])
        ])
    );
}

#[test]
fn prefixes_errors_and_keeps_them_on_one_resp_line() {
    assert_eq!(
        frame_from_output(CommandOutput::Error("bad\r\nargument".to_owned())),
        RespFrame::Error("ERR bad  argument".to_owned())
    );
}

#[test]
fn converts_help_to_a_bulk_string_because_it_contains_newlines() {
    assert_eq!(
        frame_from_output(CommandOutput::Help),
        RespFrame::BulkString(HELP_TEXT.as_bytes().to_vec())
    );
}

#[test]
fn converted_outputs_encode_as_expected_resp2_frames() {
    let mut encoded = Vec::new();
    frame_from_output(CommandOutput::OptionalValues(vec![
        Some(b"a\0\xff".to_vec()),
        None,
    ]))
    .write_to(&mut encoded)
    .unwrap();

    assert_eq!(encoded, b"*2\r\n$3\r\na\0\xff\r\n$-1\r\n");
}

#[test]
fn converts_existing_outputs_to_resp3_semantic_types() {
    assert_eq!(
        frame_from_output_for_protocol(CommandOutput::Float(1.5), ProtocolVersion::Resp3),
        RespFrame::BulkString(b"1.5".to_vec())
    );
    assert_eq!(
        frame_from_output_for_protocol(CommandOutput::Nil, ProtocolVersion::Resp3),
        RespFrame::Null
    );
    assert_eq!(
        frame_from_output_for_protocol(
            CommandOutput::OptionalValues(vec![None]),
            ProtocolVersion::Resp3,
        ),
        RespFrame::Array(vec![RespFrame::Null])
    );
}

#[test]
fn hello_uses_a_flat_array_in_resp2_and_a_map_in_resp3() {
    let output = || CommandOutput::Hello {
        protocol: Some(ProtocolVersion::Resp3),
        connection_id: None,
    };

    assert!(matches!(
        frame_from_output_for_protocol(output(), ProtocolVersion::Resp2),
        RespFrame::Array(values) if values.len() == 12
    ));
    assert!(matches!(
        frame_from_output_for_protocol(output(), ProtocolVersion::Resp3),
        RespFrame::Map(entries) if entries.len() == 6
    ));
}

#[test]
fn command_metadata_uses_six_field_entries_and_protocol_nulls() {
    let get = crate::command::command_metadata(b"GET");
    let output = || CommandOutput::CommandMetadata(vec![get, None]);

    assert!(matches!(
        frame_from_output_for_protocol(output(), ProtocolVersion::Resp2),
        RespFrame::Array(entries)
            if matches!(&entries[0], RespFrame::Array(fields) if fields.len() == 6)
                && entries[1] == RespFrame::NullBulkString
    ));
    assert!(matches!(
        frame_from_output_for_protocol(output(), ProtocolVersion::Resp3),
        RespFrame::Array(entries) if entries[1] == RespFrame::Null
    ));
}
