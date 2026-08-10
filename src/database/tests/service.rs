use crate::command::Command;
use crate::database::Database;
use crate::output::CommandOutput;

#[test]
fn preserves_state_between_commands() {
    let mut database = Database::default();

    assert_eq!(
        database.execute(Command::Set {
            key: "key".to_owned(),
            value: "value".to_owned(),
        }),
        CommandOutput::Ok
    );
    assert_eq!(
        database.execute(Command::Get {
            key: "key".to_owned(),
        }),
        CommandOutput::Value("value".to_owned())
    );
}

#[test]
fn propagates_wrong_type_errors() {
    let mut database = Database::default();

    assert_eq!(
        database.execute(Command::LPush {
            key: "key".to_owned(),
            value: "value".to_owned(),
        }),
        CommandOutput::Integer(1)
    );
    assert_eq!(
        database.execute(Command::Get {
            key: "key".to_owned(),
        }),
        CommandOutput::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}

#[test]
fn instances_have_independent_state() {
    let mut first = Database::default();
    let mut second = Database::default();

    first.execute(Command::Set {
        key: "key".to_owned(),
        value: "value".to_owned(),
    });

    assert_eq!(
        second.execute(Command::Get {
            key: "key".to_owned(),
        }),
        CommandOutput::Nil
    );
}
