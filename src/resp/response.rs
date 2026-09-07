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
        CommandOutput::IntegerList(values) => {
            RespFrame::Array(values.into_iter().map(RespFrame::Integer).collect())
        }
        CommandOutput::Score(value) => score_frame(value, protocol),
        CommandOutput::Scores(values) => RespFrame::Array(
            values
                .into_iter()
                .map(|score| score_frame(score, protocol))
                .collect(),
        ),
        CommandOutput::SortedSetScan { cursor, entries } => RespFrame::Array(vec![
            RespFrame::BulkString(cursor.to_string().into_bytes()),
            RespFrame::Array(
                entries
                    .into_iter()
                    .flat_map(|(member, score)| {
                        [
                            RespFrame::BulkString(member),
                            RespFrame::BulkString(score.to_string().into_bytes()),
                        ]
                    })
                    .collect(),
            ),
        ]),
        CommandOutput::PoppedMember(entry) => RespFrame::Array(
            entry
                .into_iter()
                .flat_map(|(member, score)| {
                    [
                        RespFrame::BulkString(member),
                        score_frame(Some(score), protocol),
                    ]
                })
                .collect(),
        ),
        CommandOutput::ScoredMembers(entries) => {
            if protocol == ProtocolVersion::Resp3 {
                RespFrame::Array(
                    entries
                        .into_iter()
                        .map(|(member, score)| {
                            RespFrame::Array(vec![
                                RespFrame::BulkString(member),
                                score_frame(Some(score), protocol),
                            ])
                        })
                        .collect(),
                )
            } else {
                RespFrame::Array(
                    entries
                        .into_iter()
                        .flat_map(|(member, score)| {
                            [
                                RespFrame::BulkString(member),
                                score_frame(Some(score), protocol),
                            ]
                        })
                        .collect(),
                )
            }
        }
        CommandOutput::Float(value) => RespFrame::BulkString(value.to_string().into_bytes()),
        CommandOutput::SimpleString(value) => RespFrame::SimpleString(value.to_owned()),
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
        CommandOutput::NullArray if protocol == ProtocolVersion::Resp3 => RespFrame::Null,
        CommandOutput::NullArray => RespFrame::NullArray,
        CommandOutput::Transaction(outputs) => RespFrame::Array(
            outputs
                .into_iter()
                .map(|output| frame_from_output_for_protocol(output, protocol))
                .collect(),
        ),
        CommandOutput::KeyList(values) => {
            RespFrame::Array(values.into_iter().map(RespFrame::BulkString).collect())
        }
        CommandOutput::HashEntries(entries) if protocol == ProtocolVersion::Resp3 => {
            RespFrame::Map(
                entries
                    .into_iter()
                    .map(|(field, value)| {
                        (RespFrame::BulkString(field), RespFrame::BulkString(value))
                    })
                    .collect(),
            )
        }
        CommandOutput::HashEntries(entries) => RespFrame::Array(
            entries
                .into_iter()
                .flat_map(|(field, value)| {
                    [RespFrame::BulkString(field), RespFrame::BulkString(value)]
                })
                .collect(),
        ),
        CommandOutput::HashScan { cursor, entries } => RespFrame::Array(vec![
            RespFrame::BulkString(cursor.to_string().into_bytes()),
            RespFrame::Array(
                entries
                    .into_iter()
                    .flat_map(|(field, value)| {
                        [RespFrame::BulkString(field), RespFrame::BulkString(value)]
                    })
                    .collect(),
            ),
        ]),
        CommandOutput::Scan { cursor, keys } => RespFrame::Array(vec![
            RespFrame::BulkString(cursor.to_string().into_bytes()),
            RespFrame::Array(keys.into_iter().map(RespFrame::BulkString).collect()),
        ]),
        CommandOutput::CommandMetadata(entries) => RespFrame::Array(
            entries
                .into_iter()
                .map(|entry| match entry {
                    Some(metadata) => command_metadata_frame(metadata),
                    None if protocol == ProtocolVersion::Resp3 => RespFrame::Null,
                    None => RespFrame::NullBulkString,
                })
                .collect(),
        ),
        CommandOutput::Error(error) => error_frame(format_args!("ERR {error}")),
        CommandOutput::ExecAbort => {
            error_frame("EXECABORT Transaction discarded because of previous errors")
        }
        CommandOutput::WatchVersions(_) => RespFrame::SimpleString("OK".to_owned()),
        CommandOutput::PubSubAcks(acks) => {
            let frames = acks
                .into_iter()
                .map(|ack| {
                    let values = vec![
                        RespFrame::BulkString(
                            match (ack.pattern, ack.subscribed) {
                                (false, true) => b"subscribe".as_slice(),
                                (false, false) => b"unsubscribe".as_slice(),
                                (true, true) => b"psubscribe".as_slice(),
                                (true, false) => b"punsubscribe".as_slice(),
                            }
                            .to_vec(),
                        ),
                        ack.channel.map(RespFrame::BulkString).unwrap_or_else(|| {
                            if protocol == ProtocolVersion::Resp3 {
                                RespFrame::Null
                            } else {
                                RespFrame::NullBulkString
                            }
                        }),
                        RespFrame::Integer(i64::try_from(ack.count).unwrap_or(i64::MAX)),
                    ];
                    if protocol == ProtocolVersion::Resp3 {
                        RespFrame::Push(values)
                    } else {
                        RespFrame::Array(values)
                    }
                })
                .collect();
            RespFrame::Sequence(frames)
        }
        CommandOutput::PubSubMessage { channel, message } => pubsub_frame(
            vec![
                RespFrame::BulkString(b"message".to_vec()),
                RespFrame::BulkString(channel),
                RespFrame::BulkString(message),
            ],
            protocol,
        ),
        CommandOutput::PubSubPatternMessage {
            pattern,
            channel,
            message,
        } => pubsub_frame(
            vec![
                RespFrame::BulkString(b"pmessage".to_vec()),
                RespFrame::BulkString(pattern),
                RespFrame::BulkString(channel),
                RespFrame::BulkString(message),
            ],
            protocol,
        ),
        CommandOutput::PubSubPong(message) => pubsub_frame(
            vec![
                RespFrame::BulkString(b"pong".to_vec()),
                RespFrame::BulkString(message.unwrap_or_default()),
            ],
            protocol,
        ),
        CommandOutput::Help => RespFrame::BulkString(HELP_TEXT.as_bytes().to_vec()),
    }
}

fn pubsub_frame(values: Vec<RespFrame>, protocol: ProtocolVersion) -> RespFrame {
    if protocol == ProtocolVersion::Resp3 {
        RespFrame::Push(values)
    } else {
        RespFrame::Array(values)
    }
}

fn score_frame(score: Option<f64>, protocol: ProtocolVersion) -> RespFrame {
    match (score, protocol) {
        (Some(score), ProtocolVersion::Resp3) => RespFrame::Double(score.to_string()),
        (Some(score), ProtocolVersion::Resp2) => {
            RespFrame::BulkString(score.to_string().into_bytes())
        }
        (None, ProtocolVersion::Resp3) => RespFrame::Null,
        (None, ProtocolVersion::Resp2) => RespFrame::NullBulkString,
    }
}

fn command_metadata_frame(metadata: crate::command::CommandMetadata) -> RespFrame {
    RespFrame::Array(vec![
        RespFrame::BulkString(metadata.name.as_bytes().to_vec()),
        RespFrame::Integer(metadata.arity),
        RespFrame::Array(
            metadata
                .flags
                .iter()
                .map(|flag| RespFrame::BulkString(flag.as_bytes().to_vec()))
                .collect(),
        ),
        RespFrame::Integer(metadata.first_key),
        RespFrame::Integer(metadata.last_key),
        RespFrame::Integer(metadata.key_step),
    ])
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
