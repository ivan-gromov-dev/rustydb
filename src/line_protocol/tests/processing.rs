use crate::command::Command;
use crate::line_protocol::{ParsedLine, parse_line};
use crate::output::CommandOutput;

#[test]
fn parses_valid_commands_without_executing_them() {
    assert_eq!(
        parse_line("SET key value\n"),
        ParsedLine::Command(Command::Set {
            key: "key".to_owned(),
            value: "value".to_owned(),
        })
    );
}

#[test]
fn identifies_empty_input() {
    assert_eq!(parse_line("\n"), ParsedLine::Empty);
    assert_eq!(parse_line("   \t\n"), ParsedLine::Empty);
}

#[test]
fn converts_parse_errors_to_command_output() {
    assert_eq!(
        parse_line("UNKNOWN\n"),
        ParsedLine::Error(CommandOutput::Error("unknown command: UNKNOWN".to_owned()))
    );
}

#[test]
fn parses_exit_as_a_regular_command() {
    assert_eq!(parse_line("EXIT\n"), ParsedLine::Command(Command::Exit));
}
