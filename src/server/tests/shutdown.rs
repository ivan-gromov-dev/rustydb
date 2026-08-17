use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::command::Command;
use crate::database::Database;
use crate::output::CommandOutput;
use crate::server::listener::run_server_on_listener_with_database;
use crate::server::{Shutdown, run_server_on_listener};

static NEXT_SNAPSHOT: AtomicU64 = AtomicU64::new(0);

fn snapshot_path() -> PathBuf {
    let sequence = NEXT_SNAPSHOT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "rustydb-server-test-{}-{sequence}.snapshot",
        std::process::id()
    ))
}

fn start_server() -> (SocketAddr, Shutdown, JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = Shutdown::default();
    let server_shutdown = shutdown.clone();
    let server = thread::spawn(move || run_server_on_listener(listener, server_shutdown));

    (address, shutdown, server)
}

#[test]
fn cloned_token_observes_shutdown_request() {
    let shutdown = Shutdown::default();
    let clone = shutdown.clone();

    shutdown.request();

    assert!(clone.is_requested());
}

#[test]
fn shutdown_without_clients_stops_the_server() {
    let (_address, shutdown, server) = start_server();

    shutdown.request();

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn shutdown_waits_for_an_active_client_to_finish() {
    let (address, shutdown, server) = start_server();
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .write_all(b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n")
        .unwrap();
    let mut response = [0; 5];
    stream.read_exact(&mut response).unwrap();
    assert_eq!(&response, b"+OK\r\n");

    stream.write_all(b"*2\r\n$3\r\nGET\r\n$3\r\nke").unwrap();
    shutdown.request();

    stream.write_all(b"y\r\n*1\r\n$4\r\nQUIT\r\n").unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    assert_eq!(response, b"$5\r\nvalue\r\n+OK\r\n");

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn save_on_shutdown_persists_server_state() {
    let snapshot = snapshot_path();
    let mut database = Database::open(&snapshot).unwrap();
    assert_eq!(
        database.execute(Command::Set {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        }),
        CommandOutput::Ok
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let shutdown = Shutdown::default();
    shutdown.request();

    run_server_on_listener_with_database(listener, shutdown, database, true).unwrap();

    let mut restored = Database::open(&snapshot).unwrap();
    assert_eq!(
        restored.execute(Command::Get {
            key: b"key".to_vec(),
        }),
        CommandOutput::Value(b"value".to_vec())
    );
    std::fs::remove_file(snapshot).unwrap();
}
