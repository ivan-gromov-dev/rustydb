use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::server::{Shutdown, run_server_on_listener};

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
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    stream.write_all(b"SET key value\n").unwrap();
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    assert_eq!(response, "OK\n");

    stream.write_all(b"GET key").unwrap();
    shutdown.request();

    stream.write_all(b"\nEXIT\n").unwrap();
    response.clear();
    reader.read_line(&mut response).unwrap();
    assert_eq!(response, "value\n");
    response.clear();
    reader.read_line(&mut response).unwrap();
    assert_eq!(response, "Bye!\n");

    assert!(server.join().unwrap().is_ok());
}
