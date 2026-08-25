use crate::command::Command;
use crate::database::Database;
use crate::output::CommandOutput;

#[test]
fn preserves_state_between_commands() {
    let mut database = Database::default();

    assert_eq!(
        database.execute(Command::Set {
            key: "key".to_owned().into(),
            value: "value".to_owned().into(),
        }),
        CommandOutput::Ok
    );
    assert_eq!(
        database.execute(Command::Get {
            key: "key".to_owned().into(),
        }),
        CommandOutput::Value("value".to_owned().into())
    );
}

#[test]
fn propagates_wrong_type_errors() {
    let mut database = Database::default();

    assert_eq!(
        database.execute(Command::LPush {
            key: "key".to_owned().into(),
            value: "value".to_owned().into(),
        }),
        CommandOutput::Integer(1)
    );
    assert_eq!(
        database.execute(Command::Get {
            key: "key".to_owned().into(),
        }),
        CommandOutput::Error("operation against a key holding the wrong kind of value".to_owned())
    );
}

#[test]
fn instances_have_independent_state() {
    let mut first = Database::default();
    let mut second = Database::default();

    first.execute(Command::Set {
        key: "key".to_owned().into(),
        value: "value".to_owned().into(),
    });

    assert_eq!(
        second.execute(Command::Get {
            key: "key".to_owned().into(),
        }),
        CommandOutput::Nil
    );
}

#[test]
fn info_reports_documented_database_counters() {
    let mut database = Database::default();
    database.client_connected();
    database.execute(Command::Set {
        key: b"key".to_vec(),
        value: b"value".to_vec(),
    });
    database.execute(Command::Get {
        key: b"key".to_vec(),
    });
    database.execute(Command::MGet {
        keys: vec![b"key".to_vec(), b"missing".to_vec()],
    });
    database.execute(Command::Exists {
        keys: vec![b"key".to_vec(), b"key".to_vec(), b"missing".to_vec()],
    });
    assert!(database.execute(Command::Save).is_error());

    assert_eq!(
        database.execute(Command::Info),
        CommandOutput::Value(
            concat!(
                "connected_clients:1\n",
                "total_connections:1\n",
                "commands_processed:6\n",
                "keyspace_hits:4\n",
                "keyspace_misses:2\n",
                "expired_keys:0\n",
                "evicted_keys:0\n",
                "persistence_successes:0\n",
                "persistence_failures:1\n",
            )
            .as_bytes()
            .to_vec()
        )
    );

    database.client_disconnected();
    let CommandOutput::Value(info) = database.execute(Command::Info) else {
        panic!("INFO should return a value");
    };
    let info = String::from_utf8(info).unwrap();
    assert!(info.contains("connected_clients:0\n"));
    assert!(info.contains("total_connections:1\n"));
    assert!(info.contains("commands_processed:7\n"));
}
