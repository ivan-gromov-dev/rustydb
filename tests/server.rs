use std::io::{Read, Write};
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

fn request(arguments: &[&[u8]]) -> Vec<u8> {
    let mut encoded = format!("*{}\r\n", arguments.len()).into_bytes();
    for argument in arguments {
        encoded.extend(format!("${}\r\n", argument.len()).bytes());
        encoded.extend_from_slice(argument);
        encoded.extend_from_slice(b"\r\n");
    }
    encoded
}

fn pipeline(commands: &[&[&[u8]]]) -> Vec<u8> {
    let mut encoded = Vec::new();
    for command in commands {
        encoded.extend(request(command));
    }
    encoded
}

fn exchange(mut stream: TcpStream, input: &[u8]) -> Vec<u8> {
    stream.write_all(input).unwrap();
    stream.shutdown(NetShutdown::Write).unwrap();

    let mut output = Vec::new();
    stream.read_to_end(&mut output).unwrap();
    output
}

#[test]
fn public_server_api_shares_binary_state_between_clients() {
    let (address, shutdown, server) = start_server();
    let set = pipeline(&[&[b"SET", b"key\0\xff", b"value\r\n\0\xff"], &[b"QUIT"]]);
    let get = pipeline(&[&[b"GET", b"key\0\xff"], &[b"QUIT"]]);

    assert_eq!(exchange(connect(address), &set), b"+OK\r\n+OK\r\n");
    assert_eq!(
        exchange(connect(address), &get),
        b"$9\r\nvalue\r\n\0\xff\r\n+OK\r\n"
    );

    shutdown.request();
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn public_shutdown_api_allows_an_active_session_to_finish() {
    let (address, shutdown, server) = start_server();
    let mut stream = connect(address);

    stream
        .write_all(&request(&[b"SET", b"key", b"value"]))
        .unwrap();
    let mut response = [0; 5];
    stream.read_exact(&mut response).unwrap();
    assert_eq!(&response, b"+OK\r\n");

    shutdown.request();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match TcpStream::connect_timeout(&address, Duration::from_millis(50)) {
            Ok(new_client) if Instant::now() < deadline => {
                drop(new_client);
                thread::sleep(Duration::from_millis(10));
            }
            Ok(new_client) => {
                drop(new_client);
                panic!("server continued accepting connections after shutdown");
            }
            Err(_) => break,
        }
    }

    stream
        .write_all(&pipeline(&[&[b"GET", b"key"], &[b"QUIT"]]))
        .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    assert_eq!(response, b"$5\r\nvalue\r\n+OK\r\n");
    assert!(server.join().unwrap().is_ok());
}

#[test]
fn public_server_api_isolates_a_bad_client() {
    let (address, shutdown, server) = start_server();

    assert_eq!(
        exchange(connect(address), b"?bad\r\n"),
        b"-ERR Protocol error: invalid RESP prefix: 0x3f\r\n"
    );
    let valid = pipeline(&[&[b"SET", b"key", b"value"], &[b"QUIT"]]);
    assert_eq!(exchange(connect(address), &valid), b"+OK\r\n+OK\r\n");

    shutdown.request();
    assert!(server.join().unwrap().is_ok());
}
