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
fn transaction_keeps_execution_errors_and_runs_later_commands() {
    let mut database = Database::default();

    assert_eq!(
        database.execute(Command::Transaction {
            watched: Vec::new(),
            commands: vec![
                Command::LPush {
                    key: b"key".to_vec(),
                    value: b"value".to_vec(),
                },
                Command::Get {
                    key: b"key".to_vec(),
                },
                Command::LLen {
                    key: b"key".to_vec(),
                },
            ],
        }),
        CommandOutput::Transaction(vec![
            CommandOutput::Integer(1),
            CommandOutput::Error(
                "operation against a key holding the wrong kind of value".to_owned()
            ),
            CommandOutput::Integer(1),
        ])
    );
}

#[test]
fn watched_transaction_aborts_only_after_a_watched_key_changes() {
    let mut database = Database::default();
    database.execute(Command::Set {
        key: b"watched".to_vec(),
        value: b"old".to_vec(),
    });
    let CommandOutput::WatchVersions(watched) = database.execute(Command::Watch {
        keys: vec![b"watched".to_vec()],
    }) else {
        panic!("WATCH should return key versions");
    };

    database.execute(Command::Set {
        key: b"unrelated".to_vec(),
        value: b"value".to_vec(),
    });
    assert_eq!(
        database.execute(Command::Transaction {
            commands: vec![Command::Get {
                key: b"watched".to_vec(),
            }],
            watched: watched.clone(),
        }),
        CommandOutput::Transaction(vec![CommandOutput::Value(b"old".to_vec())])
    );

    database.execute(Command::Delete {
        keys: vec![b"watched".to_vec()],
    });
    assert_eq!(
        database.execute(Command::Transaction {
            commands: vec![Command::Set {
                key: b"result".to_vec(),
                value: b"not-written".to_vec(),
            }],
            watched,
        }),
        CommandOutput::NullArray
    );
    assert_eq!(
        database.execute(Command::Get {
            key: b"result".to_vec(),
        }),
        CommandOutput::Nil
    );
}

#[test]
fn watched_transaction_detects_expiration_flush_and_eviction() {
    use std::num::NonZeroUsize;

    use crate::config::MemoryConfig;

    let path = std::env::temp_dir().join(format!(
        "rustydb-watch-{}-{}.snapshot",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut database = Database::open_with_config(
        path,
        MemoryConfig::with_max_keys(NonZeroUsize::new(1).unwrap()),
    )
    .unwrap();

    database.execute(Command::Set {
        key: b"a".to_vec(),
        value: b"value".to_vec(),
    });
    let CommandOutput::WatchVersions(eviction_watch) = database.execute(Command::Watch {
        keys: vec![b"a".to_vec()],
    }) else {
        panic!("WATCH should return key versions");
    };
    database.execute(Command::Set {
        key: b"z".to_vec(),
        value: b"value".to_vec(),
    });
    assert_eq!(
        database.execute(Command::Transaction {
            commands: Vec::new(),
            watched: eviction_watch,
        }),
        CommandOutput::NullArray
    );

    let CommandOutput::WatchVersions(expiration_watch) = database.execute(Command::Watch {
        keys: vec![b"z".to_vec()],
    }) else {
        panic!("WATCH should return key versions");
    };
    database.execute(Command::PExpire {
        key: b"z".to_vec(),
        milliseconds: 0,
    });
    assert_eq!(
        database.execute(Command::Transaction {
            commands: Vec::new(),
            watched: expiration_watch,
        }),
        CommandOutput::NullArray
    );

    database.execute(Command::Set {
        key: b"key".to_vec(),
        value: b"value".to_vec(),
    });
    let CommandOutput::WatchVersions(flush_watch) = database.execute(Command::Watch {
        keys: vec![b"key".to_vec()],
    }) else {
        panic!("WATCH should return key versions");
    };
    database.execute(Command::FlushDb);
    assert_eq!(
        database.execute(Command::Transaction {
            commands: Vec::new(),
            watched: flush_watch,
        }),
        CommandOutput::NullArray
    );
}

#[test]
fn aof_replays_an_executed_transaction_as_one_batch() {
    use crate::config::MemoryConfig;

    let path = std::env::temp_dir().join(format!(
        "rustydb-transaction-{}-{}.aof",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        let mut database = Database::open_aof_with_config(&path, MemoryConfig::default()).unwrap();
        assert_eq!(
            database.execute(Command::Transaction {
                commands: vec![
                    Command::Set {
                        key: b"counter".to_vec(),
                        value: b"1".to_vec(),
                    },
                    Command::Increment {
                        key: b"counter".to_vec(),
                    },
                ],
                watched: Vec::new(),
            }),
            CommandOutput::Transaction(vec![CommandOutput::Ok, CommandOutput::Integer(2)])
        );
    }

    let bytes = std::fs::read(&path).unwrap();
    let record_length = u64::from_le_bytes(bytes[10..18].try_into().unwrap()) as usize;
    assert_eq!(bytes.len(), 10 + 8 + record_length + 8);

    let mut reopened = Database::open_aof_with_config(&path, MemoryConfig::default()).unwrap();
    assert_eq!(
        reopened.execute(Command::Get {
            key: b"counter".to_vec(),
        }),
        CommandOutput::Value(b"2".to_vec())
    );
    drop(reopened);
    std::fs::remove_file(path).unwrap();
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
