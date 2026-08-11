use super::decoder::{DecodeError, DecodeLimits, DecodeResult, decode};
use super::frame::RespFrame;

fn complete(input: &[u8]) -> (RespFrame, usize) {
    match decode(input, DecodeLimits::default()).unwrap() {
        DecodeResult::Complete { frame, consumed } => (frame, consumed),
        DecodeResult::Incomplete => panic!("expected a complete frame"),
    }
}

#[test]
fn decodes_every_resp2_frame_type() {
    assert_eq!(
        complete(b"+OK\r\n"),
        (RespFrame::SimpleString("OK".to_owned()), 5)
    );
    assert_eq!(
        complete(b"-ERR failure\r\n"),
        (RespFrame::Error("ERR failure".to_owned()), 14)
    );
    assert_eq!(complete(b":-42\r\n"), (RespFrame::Integer(-42), 6));
    assert_eq!(
        complete(b"$7\r\na b\r\n\0\xff\r\n"),
        (RespFrame::BulkString(b"a b\r\n\0\xff".to_vec()), 13)
    );
    assert_eq!(complete(b"$-1\r\n"), (RespFrame::NullBulkString, 5));
    assert_eq!(complete(b"*-1\r\n"), (RespFrame::NullArray, 5));
}

#[test]
fn decodes_nested_and_empty_arrays() {
    assert_eq!(complete(b"*0\r\n"), (RespFrame::Array(Vec::new()), 4));
    assert_eq!(
        complete(b"*2\r\n$3\r\nGET\r\n*2\r\n:1\r\n+OK\r\n"),
        (
            RespFrame::Array(vec![
                RespFrame::BulkString(b"GET".to_vec()),
                RespFrame::Array(vec![
                    RespFrame::Integer(1),
                    RespFrame::SimpleString("OK".to_owned()),
                ]),
            ]),
            26,
        )
    );
}

#[test]
fn returns_incomplete_for_every_fragment_of_a_frame() {
    let input = b"*2\r\n$3\r\nGET\r\n$5\r\nvalue\r\n";

    for length in 0..input.len() {
        assert_eq!(
            decode(&input[..length], DecodeLimits::default()),
            Ok(DecodeResult::Incomplete),
            "fragment length {length}"
        );
    }

    assert_eq!(complete(input).1, input.len());
}

#[test]
fn consumes_only_the_first_of_multiple_frames() {
    assert_eq!(complete(b":1\r\n:2\r\n"), (RespFrame::Integer(1), 4));
}

#[test]
fn rejects_malformed_prefixes_lines_numbers_and_lengths() {
    for (input, expected) in [
        (&b"?value\r\n"[..], DecodeError::InvalidPrefix(b'?')),
        (&b"+value\n"[..], DecodeError::InvalidLineEnding),
        (&b"+value\rX"[..], DecodeError::InvalidLineEnding),
        (&b":abc\r\n"[..], DecodeError::InvalidInteger),
        (
            &b":9223372036854775808\r\n"[..],
            DecodeError::InvalidInteger,
        ),
        (&b"$-2\r\n"[..], DecodeError::InvalidLength),
        (&b"*x\r\n"[..], DecodeError::InvalidLength),
        (&b"$1\r\naXX"[..], DecodeError::InvalidLineEnding),
        (&b"+\xff\r\n"[..], DecodeError::InvalidUtf8),
    ] {
        assert_eq!(decode(input, DecodeLimits::default()), Err(expected));
    }
}

#[test]
fn enforces_frame_array_and_depth_limits() {
    let limits = DecodeLimits {
        max_frame_size: 8,
        max_array_len: 1,
        max_depth: 1,
    };

    assert_eq!(
        decode(b"$4\r\ndata\r\n", limits),
        Err(DecodeError::FrameTooLarge)
    );
    assert_eq!(
        decode(b"+12345678", limits),
        Err(DecodeError::FrameTooLarge)
    );
    assert_eq!(
        decode(b"*2\r\n:1\r\n:2\r\n", limits),
        Err(DecodeError::ArrayTooLong)
    );
    assert_eq!(
        decode(b"*1\r\n*1\r\n:1\r\n", limits),
        Err(DecodeError::TooDeep)
    );
}

#[test]
fn accepts_values_exactly_at_configured_limits() {
    let limits = DecodeLimits {
        max_frame_size: 10,
        max_array_len: 1,
        max_depth: 1,
    };

    assert_eq!(
        decode(b"$4\r\ndata\r\n", limits),
        Ok(DecodeResult::Complete {
            frame: RespFrame::BulkString(b"data".to_vec()),
            consumed: 10,
        })
    );
    assert_eq!(
        decode(b"*1\r\n:1\r\n", limits),
        Ok(DecodeResult::Complete {
            frame: RespFrame::Array(vec![RespFrame::Integer(1)]),
            consumed: 8,
        })
    );
}

#[test]
fn decode_errors_have_stable_messages() {
    assert_eq!(
        DecodeError::InvalidPrefix(0xff).to_string(),
        "invalid RESP prefix: 0xff"
    );

    for (error, message) in [
        (DecodeError::InvalidLineEnding, "invalid RESP line ending"),
        (DecodeError::InvalidInteger, "invalid RESP integer"),
        (DecodeError::InvalidLength, "invalid RESP length"),
        (DecodeError::InvalidUtf8, "invalid UTF-8 in RESP text frame"),
        (DecodeError::FrameTooLarge, "RESP frame exceeds size limit"),
        (
            DecodeError::ArrayTooLong,
            "RESP array exceeds element limit",
        ),
        (DecodeError::TooDeep, "RESP frame exceeds nesting limit"),
    ] {
        assert_eq!(error.to_string(), message);
    }
}
