use crate::output::{CommandOutput, HELP_TEXT};

use super::frame::RespFrame;

pub(crate) fn frame_from_output(output: CommandOutput) -> RespFrame {
    match output {
        CommandOutput::Ok | CommandOutput::Exit => RespFrame::SimpleString("OK".to_owned()),
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
        CommandOutput::Error(error) => RespFrame::Error(format!("ERR {}", single_line(error))),
        CommandOutput::Help => RespFrame::BulkString(HELP_TEXT.as_bytes().to_vec()),
    }
}

fn single_line(error: String) -> String {
    error.replace(['\r', '\n'], " ")
}
