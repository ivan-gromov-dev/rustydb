use crate::database::Database;
use crate::line_protocol::process_line;
use crate::output::CommandOutput;

#[test]
fn executes_commands_and_preserves_database_state() {
    let mut database = Database::default();

    assert_eq!(
        process_line(&mut database, "SET key value\n"),
        Some(CommandOutput::Ok)
    );
    assert_eq!(
        process_line(&mut database, "GET key\n"),
        Some(CommandOutput::Value("value".to_owned()))
    );
}

#[test]
fn ignores_empty_input() {
    let mut database = Database::default();

    assert_eq!(process_line(&mut database, "\n"), None);
    assert_eq!(process_line(&mut database, "   \t\n"), None);
}

#[test]
fn converts_parse_errors_to_command_output() {
    let mut database = Database::default();

    assert_eq!(
        process_line(&mut database, "UNKNOWN\n"),
        Some(CommandOutput::Error("unknown command: UNKNOWN".to_owned()))
    );
}

#[test]
fn returns_exit_as_a_control_output() {
    let mut database = Database::default();

    assert_eq!(
        process_line(&mut database, "EXIT\n"),
        Some(CommandOutput::Exit)
    );
}
