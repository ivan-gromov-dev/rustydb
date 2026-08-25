use std::io::{self, Read, Write};

use crate::command::Command;
use crate::output::CommandOutput;
use crate::resp::decoder::{DecodeLimits, DecodeResult, decode};
use crate::resp::frame::RespFrame;
use crate::resp::request::command_from_frame;
use crate::resp::response::{error_frame, frame_from_output};

const READ_CHUNK_SIZE: usize = 8 * 1024;

pub(crate) fn run_session<R, W, F>(reader: &mut R, writer: &mut W, mut execute: F) -> io::Result<()>
where
    R: Read,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
    let limits = DecodeLimits::default();
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
                    let should_exit = process_frame(frame, writer, &mut execute)?;
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

fn process_frame<F>(frame: RespFrame, writer: &mut impl Write, execute: &mut F) -> io::Result<bool>
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
            error_frame(format_args!("ERR {error}")).write_to(writer)?;
            writer.flush()?;
            return Ok(false);
        }
    };

    let output = {
        #[cfg(feature = "profiling")]
        let _scope = crate::server::profiling::profile_scope(
            crate::server::profiling::ProfilePhase::Execute,
        );
        execute(command)
    };
    let should_exit = matches!(output, CommandOutput::Exit);
    {
        #[cfg(feature = "profiling")]
        let _scope = crate::server::profiling::profile_scope(
            crate::server::profiling::ProfilePhase::Response,
        );
        frame_from_output(output).write_to(writer)?;
        writer.flush()?;
    }

    Ok(should_exit)
}

fn write_protocol_error(writer: &mut impl Write, error: impl std::fmt::Display) -> io::Result<()> {
    error_frame(format_args!("ERR Protocol error: {error}")).write_to(writer)?;
    writer.flush()
}
