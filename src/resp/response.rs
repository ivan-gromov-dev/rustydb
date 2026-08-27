use std::fmt;

use crate::command::ProtocolVersion;
use crate::output::{CommandOutput, HELP_TEXT};

use super::frame::RespFrame;

#[cfg(test)]
pub(crate) fn frame_from_output(output: CommandOutput) -> RespFrame {
    frame_from_output_for_protocol(output, ProtocolVersion::Resp2)
}

pub(crate) fn frame_from_output_for_protocol(
    output: CommandOutput,
    protocol: ProtocolVersion,
) -> RespFrame {
    match output {
        CommandOutput::Ok | CommandOutput::Exit => RespFrame::SimpleString("OK".to_owned()),
        CommandOutput::Pong => RespFrame::SimpleString("PONG".to_owned()),
        CommandOutput::Hello { connection_id, .. } => hello_frame(protocol, connection_id),
        CommandOutput::Integer(value) => RespFrame::Integer(value),
        CommandOutput::Float(value) => RespFrame::BulkString(value.to_string().into_bytes()),
        CommandOutput::Value(value) => RespFrame::BulkString(value),
        CommandOutput::OptionalValues(values) => RespFrame::Array(
            values
                .into_iter()
                .map(|value| match value {
                    Some(value) => RespFrame::BulkString(value),
                    None if protocol == ProtocolVersion::Resp3 => RespFrame::Null,
                    None => RespFrame::NullBulkString,
                })
                .collect(),
        ),
        CommandOutput::Nil if protocol == ProtocolVersion::Resp3 => RespFrame::Null,
        CommandOutput::Nil => RespFrame::NullBulkString,
        CommandOutput::KeyList(values) => {
            RespFrame::Array(values.into_iter().map(RespFrame::BulkString).collect())
        }
        CommandOutput::Error(error) => error_frame(format_args!("ERR {error}")),
        CommandOutput::Help => RespFrame::BulkString(HELP_TEXT.as_bytes().to_vec()),
    }
}

fn hello_frame(protocol: ProtocolVersion, connection_id: Option<i64>) -> RespFrame {
    let bulk = |value: &str| RespFrame::BulkString(value.as_bytes().to_vec());
    let mut entries = vec![
        (bulk("server"), bulk("rustydb")),
        (bulk("version"), bulk(env!("CARGO_PKG_VERSION"))),
        (
            bulk("proto"),
            RespFrame::Integer(i64::from(protocol.number())),
        ),
    ];
    if let Some(connection_id) = connection_id {
        entries.push((bulk("id"), RespFrame::Integer(connection_id)));
    }
    entries.extend([
        (bulk("mode"), bulk("standalone")),
        (bulk("role"), bulk("master")),
        (bulk("modules"), RespFrame::Array(Vec::new())),
    ]);

    match protocol {
        ProtocolVersion::Resp2 => RespFrame::Array(
            entries
                .into_iter()
                .flat_map(|(key, value)| [key, value])
                .collect(),
        ),
        ProtocolVersion::Resp3 => RespFrame::Map(entries),
    }
}

pub(crate) fn error_frame(error: impl fmt::Display) -> RespFrame {
    RespFrame::Error(error.to_string().replace(['\r', '\n'], " "))
}
