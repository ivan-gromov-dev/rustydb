use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown as NetShutdown, SocketAddr, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustydb::{Shutdown, run_server_on_listener};

fn start_server() -> (SocketAddr, Shutdown, JoinHandle<std::io::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let shutdown = Shutdown::default();
    let server_shutdown = shutdown.clone();
    let server = thread::spawn(move || run_server_on_listener(listener, server_shutdown));

    (address, shutdown, server)
}

fn connect(address: SocketAddr) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(2);

    loop {
        match TcpStream::connect(address) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                return stream;
            }
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("server did not accept connections: {error}"),
        }
    }
}

fn exchange(mut stream: TcpStream, input: &str) -> String {
    stream.write_all(input.as_bytes()).unwrap();
    stream.shutdown(NetShutdown::Write).unwrap();

    let mut output = String::new();
    stream.read_to_string(&mut output).unwrap();
    output
}

#[test]
fn public_server_api_shares_state_between_clients() {
    let (address, shutdown, server) = start_server();

    assert_eq!(
        exchange(connect(address), "SET key value\nEXIT\n"),
        "OK\nBye!\n"
    );
    assert_eq!(
        exchange(connect(address), "GET key\nEXIT\n"),
        "value\nBye!\n"
    );

    shutdown.request();
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn public_shutdown_api_allows_an_active_session_to_finish() {
    let (address, shutdown, server) = start_server();
    let mut stream = connect(address);
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    stream.write_all(b"SET key value\n").unwrap();
    let mut response = String::new();
    reader.read_line(&mut response).unwrap();
    assert_eq!(response, "OK\n");

    shutdown.request();
    stream.write_all(b"GET key\nEXIT\n").unwrap();

    response.clear();
    reader.read_line(&mut response).unwrap();
    assert_eq!(response, "value\n");
    response.clear();
    reader.read_line(&mut response).unwrap();
    assert_eq!(response, "Bye!\n");

    assert!(server.join().unwrap().is_ok());
}

#[test]
fn public_server_api_isolates_a_bad_client() {
    let (address, shutdown, server) = start_server();

    assert_eq!(
        exchange(connect(address), "UNKNOWN\nEXIT\n"),
        "ERR unknown command: UNKNOWN\nBye!\n"
    );
    assert_eq!(
        exchange(connect(address), "SET key value\nEXIT\n"),
        "OK\nBye!\n"
    );

    shutdown.request();
    assert!(server.join().unwrap().is_ok());
}
