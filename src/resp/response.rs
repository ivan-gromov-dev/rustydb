use std::fmt;

use crate::output::{CommandOutput, HELP_TEXT};

use super::frame::RespFrame;

pub(crate) fn frame_from_output(output: CommandOutput) -> RespFrame {
    match output {
        CommandOutput::Ok | CommandOutput::Exit => RespFrame::SimpleString("OK".to_owned()),
        CommandOutput::Pong => RespFrame::SimpleString("PONG".to_owned()),
        CommandOutput::Integer(value) => RespFrame::Integer(value),
        CommandOutput::Float(value) => RespFrame::BulkString(value.to_string().into_bytes()),
        CommandOutput::Value(value) => RespFrame::BulkString(value),
        CommandOutput::OptionalValues(values) => RespFrame::Array(
            values
                .into_iter()
                .map(|value| match value {
                    Some(value) => RespFrame::BulkString(value),
                    None => RespFrame::NullBulkString,
                })
                .collect(),
        ),
        CommandOutput::Nil => RespFrame::NullBulkString,
        CommandOutput::KeyList(values) => {
            RespFrame::Array(values.into_iter().map(RespFrame::BulkString).collect())
        }
        CommandOutput::Error(error) => error_frame(format_args!("ERR {error}")),
        CommandOutput::Help => RespFrame::BulkString(HELP_TEXT.as_bytes().to_vec()),
    }
}

pub(crate) fn error_frame(error: impl fmt::Display) -> RespFrame {
    RespFrame::Error(error.to_string().replace(['\r', '\n'], " "))
}
