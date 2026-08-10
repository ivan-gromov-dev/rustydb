use std::io::{self, BufRead, Cursor, Read, Write};

use crate::database::Database;
use crate::line_session::run_session;

fn run_script(script: &str, prompt: Option<&str>) -> String {
    let mut reader = Cursor::new(script);
    let mut writer = Vec::new();
    let mut database = Database::default();

    run_session(&mut reader, &mut writer, prompt, |command| {
        database.execute(command)
    })
    .unwrap();

    String::from_utf8(writer).unwrap()
}

#[test]
fn executes_multiple_commands_with_shared_state() {
    let output = run_script("SET key value\nGET key\n", None);

    assert_eq!(output, "OK\nvalue\n");
}

#[test]
fn empty_and_malformed_lines_do_not_end_the_session() {
    let output = run_script("\nUNKNOWN\nSET key value\n", None);

    assert_eq!(output, "ERR unknown command: UNKNOWN\nOK\n");
}

#[test]
fn empty_and_malformed_lines_do_not_invoke_the_executor() {
    let mut reader = Cursor::new("\nUNKNOWN\n");
    let mut writer = Vec::new();
    let mut executions = 0;

    run_session(&mut reader, &mut writer, None, |_| {
        executions += 1;
        crate::output::CommandOutput::Ok
    })
    .unwrap();

    assert_eq!(executions, 0);
    assert_eq!(
        String::from_utf8(writer).unwrap(),
        "ERR unknown command: UNKNOWN\n"
    );
}

#[test]
fn exit_stops_processing_remaining_lines() {
    let output = run_script("SET key value\nEXIT\nGET key\n", None);

    assert_eq!(output, "OK\nBye!\n");
}

#[test]
fn end_of_input_is_a_successful_silent_shutdown_without_prompt() {
    assert_eq!(run_script("", None), "");
}

#[test]
fn interactive_prompt_is_written_before_each_read_and_eof_gets_a_newline() {
    let output = run_script("SET key value\n", Some("db> "));

    assert_eq!(output, "db> OK\ndb> \n");
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

impl BufRead for FailingReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Err(io::Error::other("read failed"))
    }

    fn consume(&mut self, _amount: usize) {}
}

#[test]
fn propagates_reader_errors() {
    let mut reader = FailingReader;
    let mut writer = Vec::new();
    let mut database = Database::default();

    let error = run_session(&mut reader, &mut writer, None, |command| {
        database.execute(command)
    })
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
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
fn propagates_writer_errors() {
    let mut reader = Cursor::new("GET missing\n");
    let mut writer = FailingWriter;
    let mut database = Database::default();

    let error = run_session(&mut reader, &mut writer, None, |command| {
        database.execute(command)
    })
    .unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
}
