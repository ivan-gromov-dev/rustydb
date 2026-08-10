use std::io::{self, Cursor, Write};

use super::run_with;

fn run_script(script: &str) -> String {
    let mut output = Vec::new();
    run_with(Cursor::new(script), &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn runs_commands_until_exit() {
    let output = run_script("SET key sample-value\nGET key\nEXIT\n");

    assert!(output.starts_with("Rusty DB\nType HELP"));
    assert!(output.contains("db> OK\ndb> sample-value\n"));
    assert!(output.ends_with("db> Bye!\n"));
}

#[test]
fn runs_list_commands() {
    let output = run_script(
        "LPUSH tasks second item\nLPUSH tasks first item\nRPUSH tasks third item\nLLEN tasks\nLRANGE tasks 0 -1\nLPOP tasks\nRPOP tasks\nEXIT\n",
    );

    assert!(output.contains(
        "db> 1\ndb> 2\ndb> 3\ndb> 3\ndb> first item\nsecond item\nthird item\ndb> first item\ndb> third item\n"
    ));
    assert!(output.ends_with("db> Bye!\n"));
}

#[test]
fn runs_set_collection_commands() {
    let output = run_script(
        "SADD tags zeta\nSADD tags alpha\nSADD tags alpha\nSISMEMBER tags alpha\nSCARD tags\nSMEMBERS tags\nSREM tags alpha\nSCARD tags\nEXIT\n",
    );

    assert!(output.contains("db> 1\ndb> 1\ndb> 0\ndb> 1\ndb> 2\ndb> alpha\nzeta\ndb> 1\ndb> 1\n"));
    assert!(output.ends_with("db> Bye!\n"));
}

#[test]
fn reports_parse_errors_and_ignores_empty_input() {
    let output = run_script("\nUNKNOWN\nEXIT\n");

    assert!(output.contains("ERR unknown command: UNKNOWN\n"));
    assert!(output.ends_with("db> Bye!\n"));
}

#[test]
fn exits_cleanly_on_end_of_input() {
    let output = run_script("");

    assert!(output.ends_with("db> \n"));
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
fn propagates_output_errors() {
    let result = run_with(Cursor::new("EXIT\n"), FailingWriter);

    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
}
