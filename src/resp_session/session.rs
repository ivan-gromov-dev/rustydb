use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::Receiver;

use crate::command::{ClientInfoAttribute, Command, ProtocolVersion};
use crate::output::CommandOutput;
use crate::resp::decoder::{DecodeLimits, DecodeResult, decode};
use crate::resp::frame::RespFrame;
use crate::resp::request::command_from_frame;
use crate::resp::response::{error_frame, frame_from_output_for_protocol};

const READ_CHUNK_SIZE: usize = 8 * 1024;
static NEXT_CONNECTION_ID: AtomicI64 = AtomicI64::new(1);

struct ConnectionState {
    id: i64,
    protocol: ProtocolVersion,
    name: Option<Vec<u8>>,
    _library_name: Option<Vec<u8>>,
    _library_version: Option<Vec<u8>>,
    transaction: Option<TransactionState>,
    watched: HashMap<Vec<u8>, (u64, bool)>,
    subscriptions: HashSet<Vec<u8>>,
    pattern_subscriptions: HashSet<Vec<u8>>,
}

struct TransactionState {
    commands: Vec<Command>,
    dirty: bool,
}

impl ConnectionState {
    fn new(id: i64) -> Self {
        Self {
            id,
            protocol: ProtocolVersion::Resp2,
            name: None,
            _library_name: None,
            _library_version: None,
            transaction: None,
            watched: HashMap::new(),
            subscriptions: HashSet::new(),
            pattern_subscriptions: HashSet::new(),
        }
    }
}

#[cfg(test)]
pub(crate) fn run_session<R, W, F>(reader: &mut R, writer: &mut W, execute: F) -> io::Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
    let connection_id = next_connection_id()?;
    run_session_with_id(reader, writer, execute, connection_id)
}

pub(crate) fn next_connection_id() -> io::Result<i64> {
    NEXT_CONNECTION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| io::Error::other("connection ID space exhausted"))
}

#[cfg(test)]
pub(crate) fn run_session_with_id<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    execute: F,
    connection_id: i64,
) -> io::Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
    run_session_inner(reader, writer, execute, connection_id, None)
}

pub(crate) fn run_session_with_notifications<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    execute: F,
    connection_id: i64,
    notifications: &Receiver<CommandOutput>,
) -> io::Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
    run_session_inner(reader, writer, execute, connection_id, Some(notifications))
}

fn run_session_inner<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    mut execute: F,
    connection_id: i64,
    notifications: Option<&Receiver<CommandOutput>>,
) -> io::Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
    let limits = DecodeLimits::default();
    let mut state = ConnectionState::new(connection_id);
    let mut buffer = Vec::new();
    let mut read_chunk = [0; READ_CHUNK_SIZE];

    loop {
        drain_notifications(notifications, writer, state.protocol)?;
        while !buffer.is_empty() {
            let decoded = {
                #[cfg(feature = "profiling")]
                let _scope = crate::server::profiling::profile_scope(
                    crate::server::profiling::ProfilePhase::Decode,
                );
                decode(&buffer, limits)
            };
            match decoded {
                Ok(DecodeResult::Complete { frame, consumed }) => {
                    let should_exit = process_frame(frame, writer, &mut execute, &mut state)?;
                    buffer.drain(..consumed);

                    if should_exit {
                        return Ok(());
                    }
                }
                Ok(DecodeResult::Incomplete) => break,
                Err(error) => {
                    write_protocol_error(writer, error)?;
                    return Ok(());
                }
            }
        }

        let bytes_read = match reader.read(&mut read_chunk) {
            Ok(bytes_read) => bytes_read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if notifications.is_some()
                    && matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        };

        if bytes_read == 0 {
            if !buffer.is_empty() {
                write_protocol_error(writer, "unexpected end of input")?;
            }
            return Ok(());
        }

        buffer.extend_from_slice(&read_chunk[..bytes_read]);
    }
}

fn drain_notifications(
    notifications: Option<&Receiver<CommandOutput>>,
    writer: &mut impl Write,
    protocol: ProtocolVersion,
) -> io::Result<()> {
    let Some(notifications) = notifications else {
        return Ok(());
    };
    while let Ok(output) = notifications.try_recv() {
        frame_from_output_for_protocol(output, protocol).write_to(writer)?;
    }
    writer.flush()
}

fn process_frame<F>(
    frame: RespFrame,
    writer: &mut impl Write,
    execute: &mut F,
    state: &mut ConnectionState,
) -> io::Result<bool>
where
    F: FnMut(Command) -> CommandOutput,
{
    let parsed = {
        #[cfg(feature = "profiling")]
        let _scope = crate::server::profiling::profile_scope(
            crate::server::profiling::ProfilePhase::Command,
        );
        command_from_frame(frame)
    };
    let command = match parsed {
        Ok(command) => command,
        Err(error) => {
            if let Some(transaction) = &mut state.transaction {
                transaction.dirty = true;
            }
            error_frame(error.response_message()).write_to(writer)?;
            writer.flush()?;
            return Ok(false);
        }
    };

    let response_protocol = match &command {
        Command::Hello { protocol } => protocol.unwrap_or(state.protocol),
        _ => state.protocol,
    };
    #[cfg(feature = "profiling")]
    let _scope =
        crate::server::profiling::profile_scope(crate::server::profiling::ProfilePhase::Execute);
    let output = execute_connection_command(command, state, execute);
    let should_exit = matches!(output, CommandOutput::Exit);
    let is_hello = matches!(output, CommandOutput::Hello { .. });
    {
        #[cfg(feature = "profiling")]
        let _scope = crate::server::profiling::profile_scope(
            crate::server::profiling::ProfilePhase::Response,
        );
        frame_from_output_for_protocol(output, response_protocol).write_to(writer)?;
        writer.flush()?;
    }

    if is_hello {
        state.protocol = response_protocol;
    }

    Ok(should_exit)
}

fn execute_connection_command<F>(
    command: Command,
    state: &mut ConnectionState,
    execute: &mut F,
) -> CommandOutput
where
    F: FnMut(Command) -> CommandOutput,
{
    if state.protocol == ProtocolVersion::Resp2
        && (!state.subscriptions.is_empty() || !state.pattern_subscriptions.is_empty())
    {
        match &command {
            Command::Subscribe { .. }
            | Command::Unsubscribe { .. }
            | Command::PSubscribe { .. }
            | Command::PUnsubscribe { .. }
            | Command::Exit => {}
            Command::Ping { message } => return CommandOutput::PubSubPong(message.clone()),
            _ => {
                return CommandOutput::Error(
                    "only SUBSCRIBE, UNSUBSCRIBE, PSUBSCRIBE, PUNSUBSCRIBE, PING, and QUIT are allowed in subscribed mode"
                        .to_owned(),
                );
            }
        }
    }
    match command {
        Command::Multi => {
            if state.transaction.is_some() {
                CommandOutput::Error("MULTI calls can not be nested".to_owned())
            } else {
                state.transaction = Some(TransactionState {
                    commands: Vec::new(),
                    dirty: false,
                });
                CommandOutput::Ok
            }
        }
        Command::Exec => {
            let Some(transaction) = state.transaction.take() else {
                return CommandOutput::Error("EXEC without MULTI".to_owned());
            };
            let watched = std::mem::take(&mut state.watched)
                .into_iter()
                .map(|(key, (version, existed))| (key, version, existed))
                .collect();
            if transaction.dirty {
                CommandOutput::ExecAbort
            } else {
                execute(Command::Transaction {
                    commands: transaction.commands,
                    watched,
                })
            }
        }
        Command::Discard => {
            if state.transaction.take().is_some() {
                state.watched.clear();
                CommandOutput::Ok
            } else {
                CommandOutput::Error("DISCARD without MULTI".to_owned())
            }
        }
        Command::Watch { keys } => {
            if state.transaction.is_some() {
                return CommandOutput::Error("WATCH inside MULTI is not allowed".to_owned());
            }
            match execute(Command::Watch { keys }) {
                CommandOutput::WatchVersions(versions) => {
                    for (key, version, existed) in versions {
                        state.watched.entry(key).or_insert((version, existed));
                    }
                    CommandOutput::Ok
                }
                output => output,
            }
        }
        Command::Unwatch => {
            state.watched.clear();
            CommandOutput::Ok
        }
        command if state.transaction.is_some() => {
            if matches!(
                command,
                Command::Hello { .. }
                    | Command::ClientId
                    | Command::ClientSetName { .. }
                    | Command::ClientGetName
                    | Command::ClientSetInfo { .. }
                    | Command::Watch { .. }
                    | Command::Unwatch
                    | Command::Publish { .. }
                    | Command::Subscribe { .. }
                    | Command::Unsubscribe { .. }
                    | Command::PSubscribe { .. }
                    | Command::PUnsubscribe { .. }
                    | Command::PubSubChannels { .. }
                    | Command::PubSubNumSub { .. }
                    | Command::AofRewrite
                    | Command::Exit
            ) {
                if let Some(transaction) = &mut state.transaction {
                    transaction.dirty = true;
                }
                CommandOutput::Error("command cannot be used inside MULTI".to_owned())
            } else if let Some(transaction) = &mut state.transaction {
                transaction.commands.push(command);
                CommandOutput::SimpleString("QUEUED")
            } else {
                CommandOutput::Error("transaction state is unavailable".to_owned())
            }
        }
        Command::Hello { protocol } => CommandOutput::Hello {
            protocol,
            connection_id: Some(state.id),
        },
        Command::ClientId => CommandOutput::Integer(state.id),
        Command::ClientSetName { name } => {
            state.name = (!name.is_empty()).then_some(name);
            CommandOutput::Ok
        }
        Command::ClientGetName => match &state.name {
            Some(name) => CommandOutput::Value(name.clone()),
            None => CommandOutput::Nil,
        },
        Command::ClientSetInfo { attribute, value } => {
            match attribute {
                ClientInfoAttribute::LibraryName => state._library_name = Some(value),
                ClientInfoAttribute::LibraryVersion => state._library_version = Some(value),
            }
            CommandOutput::Ok
        }
        Command::Subscribe { channels } => {
            let output = execute(Command::Subscribe {
                channels: channels.clone(),
            });
            for channel in channels {
                state.subscriptions.insert(channel);
            }
            output
        }
        Command::Unsubscribe { channels } => {
            let output = execute(Command::Unsubscribe {
                channels: channels.clone(),
            });
            if channels.is_empty() {
                state.subscriptions.clear();
            } else {
                for channel in channels {
                    state.subscriptions.remove(&channel);
                }
            }
            output
        }
        Command::PSubscribe { patterns } => {
            let output = execute(Command::PSubscribe {
                patterns: patterns.clone(),
            });
            for pattern in patterns {
                state.pattern_subscriptions.insert(pattern);
            }
            output
        }
        Command::PUnsubscribe { patterns } => {
            let output = execute(Command::PUnsubscribe {
                patterns: patterns.clone(),
            });
            if patterns.is_empty() {
                state.pattern_subscriptions.clear();
            } else {
                for pattern in patterns {
                    state.pattern_subscriptions.remove(&pattern);
                }
            }
            output
        }
        command => execute(command),
    }
}

fn write_protocol_error(writer: &mut impl Write, error: impl std::fmt::Display) -> io::Result<()> {
    error_frame(format_args!("ERR Protocol error: {error}")).write_to(writer)?;
    writer.flush()
}
