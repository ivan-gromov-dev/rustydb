use std::io::{self, Cursor, Read, Write};

use crate::command::Command;
use crate::output::CommandOutput;

use super::super::run_session;

fn run(input: &[u8], execute: impl FnMut(Command) -> CommandOutput) -> Vec<u8> {
    let mut reader = Cursor::new(input);
    let mut output = Vec::new();
    run_session(&mut reader, &mut output, execute).unwrap();
    output
}

#[test]
fn executes_fragmented_binary_requests() {
    let input = b"*3\r\n$3\r\nSET\r\n$5\r\nkey\0\xff\r\n$5\r\na\r\n\0\xff\r\n";
    let mut reader = ChunkedReader::new(input, 1);
    let mut output = Vec::new();
    let mut commands = Vec::new();

    run_session(&mut reader, &mut output, |command| {
        commands.push(command);
        CommandOutput::Ok
    })
    .unwrap();

    assert_eq!(
        commands,
        vec![Command::Set {
            key: b"key\0\xff".to_vec(),
            value: b"a\r\n\0\xff".to_vec(),
        }]
    );
    assert_eq!(output, b"+OK\r\n");
}

#[test]
fn executes_every_pipelined_command_in_order() {
    let input = b"*2\r\n$3\r\nGET\r\n$1\r\na\r\n*2\r\n$3\r\nGET\r\n$1\r\nb\r\n";
    let mut seen = Vec::new();

    let output = run(input, |command| {
        seen.push(command);
        CommandOutput::Value(vec![b'0' + seen.len() as u8])
    });

    assert_eq!(
        seen,
        vec![
            Command::Get { key: b"a".to_vec() },
            Command::Get { key: b"b".to_vec() },
        ]
    );
    assert_eq!(output, b"$1\r\n1\r\n$1\r\n2\r\n");
}

#[test]
fn hello_switches_response_encoding_for_the_connection() {
    let input = concat!(
        "*2\r\n$5\r\nHELLO\r\n$1\r\n3\r\n",
        "*2\r\n$3\r\nGET\r\n$7\r\nmissing\r\n",
        "*3\r\n$11\r\nINCRBYFLOAT\r\n$7\r\ncounter\r\n$3\r\n1.5\r\n",
        "*2\r\n$5\r\nHELLO\r\n$1\r\n2\r\n",
        "*2\r\n$3\r\nGET\r\n$7\r\nmissing\r\n",
    );

    let output = run(input.as_bytes(), |command| match command {
        Command::Hello { protocol } => CommandOutput::Hello { protocol },
        Command::Get { .. } => CommandOutput::Nil,
        Command::IncrementByFloat { .. } => CommandOutput::Float(1.5),
        other => panic!("unexpected command: {other:?}"),
    });

    let version = env!("CARGO_PKG_VERSION");
    let expected = format!(
        concat!(
            "%6\r\n",
            "$6\r\nserver\r\n$7\r\nrustydb\r\n",
            "$7\r\nversion\r\n${}\r\n{}\r\n",
            "$5\r\nproto\r\n:3\r\n",
            "$4\r\nmode\r\n$10\r\nstandalone\r\n",
            "$4\r\nrole\r\n$6\r\nmaster\r\n",
            "$7\r\nmodules\r\n*0\r\n",
            "_\r\n",
            "$3\r\n1.5\r\n",
            "*12\r\n",
            "$6\r\nserver\r\n$7\r\nrustydb\r\n",
            "$7\r\nversion\r\n${}\r\n{}\r\n",
            "$5\r\nproto\r\n:2\r\n",
            "$4\r\nmode\r\n$10\r\nstandalone\r\n",
            "$4\r\nrole\r\n$6\r\nmaster\r\n",
            "$7\r\nmodules\r\n*0\r\n",
            "$-1\r\n",
        ),
        version.len(),
        version,
        version.len(),
        version,
    );
    assert_eq!(output, expected.as_bytes());
}

#[test]
fn unsupported_hello_protocol_does_not_change_or_close_the_connection() {
    let input = concat!("*2\r\n$5\r\nHELLO\r\n$1\r\n4\r\n", "*1\r\n$4\r\nPING\r\n",);
    let mut executed = Vec::new();

    let output = run(input.as_bytes(), |command| {
        executed.push(command);
        CommandOutput::Pong
    });

    assert_eq!(executed, vec![Command::Ping { message: None }]);
    assert_eq!(
        output,
        b"-NOPROTO unsupported protocol version: 4\r\n+PONG\r\n"
    );
}

#[test]
fn command_errors_do_not_stop_later_pipeline_entries() {
    let input = b"*1\r\n$3\r\nGET\r\n*2\r\n$3\r\nGET\r\n$2\r\nok\r\n";
    let mut calls = 0;

    let output = run(input, |_| {
        calls += 1;
        CommandOutput::Value(b"value".to_vec())
    });

    assert_eq!(calls, 1);
    assert_eq!(output, b"-ERR usage: GET key\r\n$5\r\nvalue\r\n");
}

#[test]
fn quit_replies_and_ignores_remaining_pipeline_entries() {
    let input = b"*1\r\n$4\r\nQUIT\r\n*1\r\n$3\r\nLEN\r\n";
    let mut calls = 0;

    let output = run(input, |command| {
        calls += 1;
        assert_eq!(command, Command::Exit);
        CommandOutput::Exit
    });

    assert_eq!(calls, 1);
    assert_eq!(output, b"+OK\r\n");
}

#[test]
fn malformed_and_truncated_frames_return_protocol_errors_without_execution() {
    for (input, expected) in [
        (
            &b"?bad\r\n"[..],
            &b"-ERR Protocol error: invalid RESP prefix: 0x3f\r\n"[..],
        ),
        (
            &b"*2\r\n$3\r\nGET\r\n"[..],
            &b"-ERR Protocol error: unexpected end of input\r\n"[..],
        ),
    ] {
        let mut executed = false;
        let output = run(input, |_| {
            executed = true;
            CommandOutput::Ok
        });

        assert!(!executed);
        assert_eq!(output, expected);
    }
}

#[test]
fn clean_end_of_input_is_silent() {
    assert_eq!(run(b"", |_| CommandOutput::Ok), b"");
}

struct ChunkedReader<'a> {
    input: &'a [u8],
    chunk_size: usize,
}

impl<'a> ChunkedReader<'a> {
    fn new(input: &'a [u8], chunk_size: usize) -> Self {
        Self { input, chunk_size }
    }
}

impl Read for ChunkedReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = self.input.len().min(buffer.len()).min(self.chunk_size);
        buffer[..length].copy_from_slice(&self.input[..length]);
        self.input = &self.input[length..];
        Ok(length)
    }
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
fn propagates_reader_and_writer_errors() {
    let mut failed_reader = FailingReader;
    let mut output = Vec::new();
    assert_eq!(
        run_session(&mut failed_reader, &mut output, |_| CommandOutput::Ok)
            .unwrap_err()
            .kind(),
        io::ErrorKind::Other
    );

    let mut reader = Cursor::new(b"*1\r\n$3\r\nLEN\r\n");
    assert_eq!(
        run_session(&mut reader, &mut FailingWriter, |_| CommandOutput::Integer(
            0
        ))
        .unwrap_err()
        .kind(),
        io::ErrorKind::Other
    );
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}
