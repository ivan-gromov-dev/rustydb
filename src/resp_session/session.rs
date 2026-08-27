use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicI64, Ordering};

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
}

impl ConnectionState {
    fn new(id: i64) -> Self {
        Self {
            id,
            protocol: ProtocolVersion::Resp2,
            name: None,
            _library_name: None,
            _library_version: None,
        }
    }
}

pub(crate) fn run_session<R, W, F>(reader: &mut R, writer: &mut W, execute: F) -> io::Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
    let connection_id = NEXT_CONNECTION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map_err(|_| io::Error::other("connection ID space exhausted"))?;
    run_session_with_id(reader, writer, execute, connection_id)
}

pub(crate) fn run_session_with_id<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    mut execute: F,
    connection_id: i64,
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
            error_frame(error.response_message()).write_to(writer)?;
            writer.flush()?;
            return Ok(false);
        }
    };

    let response_protocol = match &command {
        Command::Hello { protocol } => protocol.unwrap_or(state.protocol),
        _ => state.protocol,
    };
    let connection_output = execute_connection_command(&command, state);
    let output = if let Some(output) = connection_output {
        output
    } else {
        #[cfg(feature = "profiling")]
        let _scope = crate::server::profiling::profile_scope(
            crate::server::profiling::ProfilePhase::Execute,
        );
        execute(command)
    };
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

fn execute_connection_command(
    command: &Command,
    state: &mut ConnectionState,
) -> Option<CommandOutput> {
    match command {
        Command::Hello { protocol } => Some(CommandOutput::Hello {
            protocol: *protocol,
            connection_id: Some(state.id),
        }),
        Command::ClientId => Some(CommandOutput::Integer(state.id)),
        Command::ClientSetName { name } => {
            state.name = (!name.is_empty()).then(|| name.clone());
            Some(CommandOutput::Ok)
        }
        Command::ClientGetName => Some(match &state.name {
            Some(name) => CommandOutput::Value(name.clone()),
            None => CommandOutput::Nil,
        }),
        Command::ClientSetInfo { attribute, value } => {
            match attribute {
                ClientInfoAttribute::LibraryName => state._library_name = Some(value.clone()),
                ClientInfoAttribute::LibraryVersion => state._library_version = Some(value.clone()),
            }
            Some(CommandOutput::Ok)
        }
        _ => None,
    }
}

fn write_protocol_error(writer: &mut impl Write, error: impl std::fmt::Display) -> io::Result<()> {
    error_frame(format_args!("ERR Protocol error: {error}")).write_to(writer)?;
    writer.flush()
}
