use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::server::{run_server, serve};

fn start_server() -> (SocketAddr, JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || serve(listener));

    (address, handle)
}

fn exchange(address: SocketAddr, input: &str) -> String {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(input.as_bytes()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();

    let mut output = String::new();
    stream.read_to_string(&mut output).unwrap();
    output
}

#[test]
fn executes_multiple_commands_without_interactive_output() {
    let (address, server) = start_server();

    let output = exchange(address, "SET key value\nGET key\nEXIT\n");

    assert_eq!(output, "OK\nvalue\nBye!\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn malformed_input_does_not_end_the_connection() {
    let (address, server) = start_server();

    let output = exchange(address, "UNKNOWN\nSET key value\nEXIT\n");

    assert_eq!(output, "ERR unknown command: UNKNOWN\nOK\nBye!\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn end_of_input_stops_the_server_without_a_response() {
    let (address, server) = start_server();

    let output = exchange(address, "");

    assert_eq!(output, "");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn reports_bind_errors() {
    let error = run_server("not a valid socket address").unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}
