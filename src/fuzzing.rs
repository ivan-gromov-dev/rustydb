//! Test-only entry points for `cargo fuzz` targets.

use std::str;

use crate::command::Command;
use crate::resp::decoder::{DecodeLimits, DecodeResult, decode};
use crate::resp::request::command_from_frame;

pub fn resp_request(input: &[u8]) {
    if let Ok(DecodeResult::Complete { frame, .. }) = decode(input, DecodeLimits::default()) {
        let _ = command_from_frame(frame);
    }
}

pub fn command(input: &[u8]) {
    if let Ok(text) = str::from_utf8(input) {
        let _ = Command::parse(text);
    }
    let arguments: Vec<&[u8]> = input.split(|byte| *byte == 0).collect();
    let _ = Command::from_bytes(&arguments);
}

pub fn persistence(input: &[u8]) {
    let Some((&kind, payload)) = input.split_first() else {
        return;
    };
    if kind & 1 == 0 {
        crate::snapshot::fuzz_decode(payload);
    } else {
        crate::aof::fuzz_decode_payload(payload);
    }
}
