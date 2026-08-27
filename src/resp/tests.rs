use std::io::{self, Write};

use super::frame::RespFrame;

fn encode(frame: RespFrame) -> Vec<u8> {
    let mut output = Vec::new();
    frame.write_to(&mut output).unwrap();
    output
}

#[test]
fn encodes_scalar_frames() {
    assert_eq!(encode(RespFrame::SimpleString("OK".to_owned())), b"+OK\r\n");
    assert_eq!(
        encode(RespFrame::Error("ERR failure".to_owned())),
        b"-ERR failure\r\n"
    );
    assert_eq!(encode(RespFrame::Integer(-42)), b":-42\r\n");
    assert_eq!(encode(RespFrame::Null), b"_\r\n");
}

#[test]
fn encodes_empty_and_binary_bulk_strings() {
    assert_eq!(encode(RespFrame::BulkString(Vec::new())), b"$0\r\n\r\n");
    assert_eq!(
        encode(RespFrame::BulkString(b"a b\r\n\0\xff".to_vec())),
        b"$7\r\na b\r\n\0\xff\r\n"
    );
}

#[test]
fn encodes_nested_and_empty_arrays() {
    assert_eq!(encode(RespFrame::Array(Vec::new())), b"*0\r\n");
    assert_eq!(
        encode(RespFrame::Array(vec![
            RespFrame::BulkString(b"GET".to_vec()),
            RespFrame::Array(vec![
                RespFrame::Integer(1),
                RespFrame::SimpleString("OK".to_owned()),
            ]),
        ])),
        b"*2\r\n$3\r\nGET\r\n*2\r\n:1\r\n+OK\r\n"
    );
}

#[test]
fn encodes_resp3_maps() {
    assert_eq!(
        encode(RespFrame::Map(vec![(
            RespFrame::BulkString(b"proto".to_vec()),
            RespFrame::Integer(3),
        )])),
        b"%1\r\n$5\r\nproto\r\n:3\r\n"
    );
}

#[test]
fn encodes_both_resp2_null_representations() {
    assert_eq!(encode(RespFrame::NullBulkString), b"$-1\r\n");
    assert_eq!(encode(RespFrame::NullArray), b"*-1\r\n");
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("write failed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn propagates_writer_errors() {
    let error = RespFrame::Integer(1)
        .write_to(&mut FailingWriter)
        .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
}
