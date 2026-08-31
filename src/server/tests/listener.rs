use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::command::Command;
use crate::database::Database;
use crate::output::CommandOutput;
use crate::resp::frame::RespFrame;
use crate::server::{
    SharedDatabase, Shutdown as ServerShutdown, execute_shared, run_server,
    run_server_until_with_snapshot, serve_incoming, shared_database,
};

fn start_server(client_count: usize) -> (SocketAddr, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let database = shared_database(Database::default());
        serve_incoming(listener.incoming().take(client_count), &database)
    });

    (address, handle)
}

#[test]
fn snapshot_server_wrapper_honors_pre_requested_shutdown() {
    let reserved = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reserved.local_addr().unwrap();
    drop(reserved);
    let shutdown = ServerShutdown::default();
    shutdown.request();
    let snapshot = std::env::temp_dir().join(format!(
        "rustydb-info-server-{}-{}.snapshot",
        std::process::id(),
        address.port()
    ));

    assert!(
        run_server_until_with_snapshot(address.to_string().as_str(), shutdown, snapshot, false)
            .is_ok()
    );
}

fn request(arguments: &[&[u8]]) -> Vec<u8> {
    let frame = RespFrame::Array(
        arguments
            .iter()
            .map(|argument| RespFrame::BulkString(argument.to_vec()))
            .collect(),
    );
    let mut encoded = Vec::new();
    frame.write_to(&mut encoded).unwrap();
    encoded
}

fn pipeline(commands: &[&[&[u8]]]) -> Vec<u8> {
    let mut encoded = Vec::new();
    for command in commands {
        encoded.extend(request(command));
    }
    encoded
}

fn exchange(address: SocketAddr, input: &[u8]) -> Vec<u8> {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(input).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();

    let mut output = Vec::new();
    stream.read_to_end(&mut output).unwrap();
    output
}

fn exchange_without_half_close(address: SocketAddr, input: &[u8], response_len: usize) -> Vec<u8> {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(input).unwrap();
    let mut output = vec![0; response_len];
    stream.read_exact(&mut output).unwrap();
    stream.write_all(&request(&[b"QUIT"])).unwrap();
    let mut quit = [0; 5];
    stream.read_exact(&mut quit).unwrap();
    assert_eq!(&quit, b"+OK\r\n");
    output
}

#[test]
fn executes_pipelined_commands_without_interactive_output() {
    let (address, server) = start_server(1);
    let input = pipeline(&[&[b"SET", b"key", b"value"], &[b"GET", b"key"], &[b"QUIT"]]);

    assert_eq!(exchange(address, &input), b"+OK\r\n$5\r\nvalue\r\n+OK\r\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn clients_share_binary_database_state() {
    let (address, server) = start_server(2);
    let set = pipeline(&[
        &[b"SET", b"key\0\xff", b"line 1\r\nline 2\0\xff"],
        &[b"QUIT"],
    ]);
    let get = pipeline(&[&[b"GET", b"key\0\xff"], &[b"QUIT"]]);

    assert_eq!(exchange(address, &set), b"+OK\r\n+OK\r\n");
    assert_eq!(
        exchange(address, &get),
        b"$16\r\nline 1\r\nline 2\0\xff\r\n+OK\r\n"
    );
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn idle_client_does_not_block_another_client() {
    let (address, server) = start_server(2);
    let idle_client = TcpStream::connect(address).unwrap();
    let input = pipeline(&[&[b"SET", b"key", b"value"], &[b"QUIT"]]);

    assert_eq!(exchange(address, &input), b"+OK\r\n+OK\r\n");
    drop(idle_client);
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn concurrent_increments_are_atomic_per_command() {
    const INCREMENTS_PER_CLIENT: usize = 100;

    let (address, server) = start_server(4);
    let initialize = pipeline(&[&[b"SET", b"counter", b"0"], &[b"QUIT"]]);
    assert_eq!(exchange(address, &initialize), b"+OK\r\n+OK\r\n");

    let mut script = Vec::new();
    for _ in 0..INCREMENTS_PER_CLIENT {
        script.extend(request(&[b"INCR", b"counter"]));
    }
    script.extend(request(&[b"QUIT"]));
    let first_script = script.clone();
    let first = thread::spawn(move || exchange(address, &first_script));
    let second = thread::spawn(move || exchange(address, &script));

    assert!(first.join().unwrap().ends_with(b"+OK\r\n"));
    assert!(second.join().unwrap().ends_with(b"+OK\r\n"));
    let read = pipeline(&[&[b"GET", b"counter"], &[b"QUIT"]]);
    let expected = format!("$3\r\n{}\r\n+OK\r\n", INCREMENTS_PER_CLIENT * 2);
    assert_eq!(exchange(address, &read), expected.as_bytes());

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn blocking_pop_times_out_and_wakes_after_push() {
    let (address, server) = start_server(3);
    assert_eq!(
        exchange_without_half_close(address, &request(&[b"BLPOP", b"missing", b"0.05"]), 5),
        b"*-1\r\n"
    );

    let blocked = thread::spawn(move || {
        exchange_without_half_close(address, &request(&[b"BLPOP", b"queue", b"1"]), 26)
    });
    thread::sleep(Duration::from_millis(50));
    assert_eq!(
        exchange(
            address,
            &pipeline(&[&[b"RPUSH", b"queue", b"value"], &[b"QUIT"]])
        ),
        b":1\r\n+OK\r\n"
    );
    assert_eq!(
        blocked.join().unwrap(),
        b"*2\r\n$5\r\nqueue\r\n$5\r\nvalue\r\n"
    );
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn competing_blocking_consumers_do_not_duplicate_values() {
    let (address, server) = start_server(3);
    let first = thread::spawn(move || {
        exchange_without_half_close(address, &request(&[b"BRPOP", b"queue", b"1"]), 22)
    });
    let second = thread::spawn(move || {
        exchange_without_half_close(address, &request(&[b"BRPOP", b"queue", b"1"]), 22)
    });
    thread::sleep(Duration::from_millis(50));
    assert_eq!(
        exchange(
            address,
            &pipeline(&[&[b"RPUSH", b"queue", b"a", b"b"], &[b"QUIT"]])
        ),
        b":2\r\n+OK\r\n"
    );
    let mut values = vec![first.join().unwrap(), second.join().unwrap()];
    values.sort();
    assert_eq!(
        values,
        vec![
            b"*2\r\n$5\r\nqueue\r\n$1\r\na\r\n".to_vec(),
            b"*2\r\n$5\r\nqueue\r\n$1\r\nb\r\n".to_vec()
        ]
    );
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn blocking_move_wakes_and_moves_exactly_once() {
    let (address, server) = start_server(3);
    let blocked = thread::spawn(move || {
        exchange_without_half_close(
            address,
            &request(&[
                b"BLMOVE",
                b"source",
                b"destination",
                b"RIGHT",
                b"LEFT",
                b"1",
            ]),
            7,
        )
    });
    thread::sleep(Duration::from_millis(50));
    assert_eq!(
        exchange(
            address,
            &pipeline(&[&[b"RPUSH", b"source", b"x"], &[b"QUIT"]])
        ),
        b":1\r\n+OK\r\n"
    );
    assert_eq!(blocked.join().unwrap(), b"$1\r\nx\r\n");
    assert_eq!(
        exchange(
            address,
            &pipeline(&[&[b"LRANGE", b"destination", b"0", b"-1"], &[b"QUIT"]])
        ),
        b"*1\r\n$1\r\nx\r\n+OK\r\n"
    );
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn disconnected_infinite_blocking_client_releases_worker() {
    let (address, server) = start_server(2);
    let mut blocked = TcpStream::connect(address).unwrap();
    blocked
        .write_all(&request(&[b"BLPOP", b"queue", b"0"]))
        .unwrap();
    thread::sleep(Duration::from_millis(30));
    drop(blocked);
    assert_eq!(
        exchange(address, &pipeline(&[&[b"PING"], &[b"QUIT"]])),
        b"+PONG\r\n+OK\r\n"
    );
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn fragmented_request_survives_until_end_of_input() {
    let (address, server) = start_server(2);
    let set = request(&[b"SET", b"key", b"value"]);
    let split = set.len() - 2;
    let mut stream = TcpStream::connect(address).unwrap();
    stream.write_all(&set[..split]).unwrap();
    stream.write_all(&set[split..]).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut output = Vec::new();
    stream.read_to_end(&mut output).unwrap();
    assert_eq!(output, b"+OK\r\n");

    let get = pipeline(&[&[b"GET", b"key"], &[b"QUIT"]]);
    assert_eq!(exchange(address, &get), b"$5\r\nvalue\r\n+OK\r\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn command_error_does_not_end_the_connection() {
    let (address, server) = start_server(1);
    let input = pipeline(&[&[b"UNKNOWN"], &[b"SET", b"key", b"value"], &[b"QUIT"]]);

    assert_eq!(
        exchange(address, &input),
        b"-ERR unknown command: UNKNOWN\r\n+OK\r\n+OK\r\n"
    );
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn malformed_frame_closes_only_the_bad_connection() {
    let (address, server) = start_server(2);

    assert_eq!(
        exchange(address, b"?bad\r\n"),
        b"-ERR Protocol error: invalid RESP prefix: 0x3f\r\n"
    );
    let input = pipeline(&[&[b"SET", b"key", b"value"], &[b"QUIT"]]);
    assert_eq!(exchange(address, &input), b"+OK\r\n+OK\r\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn incoming_connection_errors_stop_the_server() {
    let error = io::Error::other("accept failed");
    let incoming: Vec<io::Result<TcpStream>> = vec![Err(error)];
    let database = shared_database(Database::default());

    let error = serve_incoming(incoming, &database).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "accept failed");
}

#[test]
fn poisoned_database_lock_is_recovered() {
    let database: SharedDatabase = shared_database(Database::default());
    let poisoned_database = Arc::clone(&database);

    assert!(
        thread::spawn(move || {
            let _guard = poisoned_database.database.lock().unwrap();
            panic!("poison database lock");
        })
        .join()
        .is_err()
    );

    assert_eq!(
        execute_shared(
            &database,
            Command::Set {
                key: b"key".to_vec(),
                value: b"value".to_vec(),
            }
        ),
        CommandOutput::Ok
    );
    assert_eq!(
        execute_shared(
            &database,
            Command::Get {
                key: b"key".to_vec(),
            }
        ),
        CommandOutput::Value(b"value".to_vec())
    );
}

#[test]
fn reports_bind_errors() {
    let error = run_server("not a valid socket address").unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}
