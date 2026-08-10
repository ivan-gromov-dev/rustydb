use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::database::Database;
use crate::server::{run_server, serve_incoming};

fn start_server(client_count: usize) -> (SocketAddr, JoinHandle<io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let mut database = Database::default();
        serve_incoming(listener.incoming().take(client_count), &mut database)
    });

    (address, handle)
}

fn exchange(address: SocketAddr, input: &str) -> String {
    exchange_bytes(address, input.as_bytes())
}

fn exchange_bytes(address: SocketAddr, input: &[u8]) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(input).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();

    let mut output = String::new();
    stream.read_to_string(&mut output).unwrap();
    output
}

#[test]
fn executes_multiple_commands_without_interactive_output() {
    let (address, server) = start_server(1);

    let output = exchange(address, "SET key value\nGET key\nEXIT\n");

    assert_eq!(output, "OK\nvalue\nBye!\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn sequential_clients_share_database_state() {
    let (address, server) = start_server(2);

    assert_eq!(exchange(address, "SET key value\nEXIT\n"), "OK\nBye!\n");
    assert_eq!(exchange(address, "GET key\nEXIT\n"), "value\nBye!\n");

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn end_of_input_closes_only_the_current_connection() {
    let (address, server) = start_server(2);

    assert_eq!(exchange(address, "SET key value\n"), "OK\n");
    assert_eq!(exchange(address, "GET key\nEXIT\n"), "value\nBye!\n");

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn malformed_input_does_not_end_the_connection() {
    let (address, server) = start_server(1);

    let output = exchange(address, "UNKNOWN\nSET key value\nEXIT\n");

    assert_eq!(output, "ERR unknown command: UNKNOWN\nOK\nBye!\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn invalid_utf8_closes_only_the_bad_connection() {
    let (address, server) = start_server(2);

    assert_eq!(exchange_bytes(address, &[0xff, b'\n']), "");
    assert_eq!(exchange(address, "SET key value\nEXIT\n"), "OK\nBye!\n");

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn incoming_connection_errors_stop_the_server() {
    let error = io::Error::other("accept failed");
    let incoming: Vec<io::Result<TcpStream>> = vec![Err(error)];
    let mut database = Database::default();

    let error = serve_incoming(incoming, &mut database).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert_eq!(error.to_string(), "accept failed");
}

#[test]
fn reports_bind_errors() {
    let error = run_server("not a valid socket address").unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}
