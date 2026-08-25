use std::env;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use rustydb::{Shutdown, run_server_on_listener};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Workload {
    Get,
    Set,
    Mixed,
}

impl Workload {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "get" => Some(Self::Get),
            "set" => Some(Self::Set),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Set => "set",
            Self::Mixed => "mixed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Configuration {
    workload: Workload,
    operations: usize,
    value_size: usize,
    concurrency: usize,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            workload: Workload::Mixed,
            operations: 100_000,
            value_size: 64,
            concurrency: 1,
        }
    }
}

fn main() {
    let arguments: Vec<_> = env::args().skip(1).collect();
    let configuration = match parse_arguments(&arguments) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("{error}");
            eprintln!(
                "Usage: rustydb-benchmark [--workload get|set|mixed] [--operations count] [--value-size bytes] [--concurrency count]"
            );
            std::process::exit(2);
        }
    };

    if let Err(error) = run(configuration) {
        eprintln!("benchmark failed: {error}");
        std::process::exit(1);
    }
}

fn parse_arguments(arguments: &[String]) -> Result<Configuration, String> {
    let mut configuration = Configuration::default();
    let mut index = 0;
    while let Some(argument) = arguments.get(index) {
        index += 1;
        let value = arguments
            .get(index)
            .ok_or_else(|| format!("missing value for {argument}"))?;
        match argument.as_str() {
            "--workload" => {
                configuration.workload =
                    Workload::parse(value).ok_or_else(|| format!("invalid workload: {value}"))?;
            }
            "--operations" => configuration.operations = positive(value, "operations")?,
            "--value-size" => configuration.value_size = positive(value, "value size")?,
            "--concurrency" => configuration.concurrency = positive(value, "concurrency")?,
            _ => return Err(format!("unknown option: {argument}")),
        }
        index += 1;
    }
    Ok(configuration)
}

fn positive(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{name} must be a positive integer"))
}

fn run(configuration: Configuration) -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let shutdown = Shutdown::default();
    let server_shutdown = shutdown.clone();
    let server = thread::spawn(move || run_server_on_listener(listener, server_shutdown));
    let barrier = Arc::new(Barrier::new(configuration.concurrency + 1));
    let mut workers = Vec::with_capacity(configuration.concurrency);

    for worker in 0..configuration.concurrency {
        let barrier = Arc::clone(&barrier);
        let count = operations_for_worker(configuration, worker);
        workers.push(thread::spawn(move || {
            run_worker(address, configuration, worker, count, &barrier)
        }));
    }

    barrier.wait();
    let started = Instant::now();
    let mut worker_error = None;
    for worker in workers {
        match worker.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                worker_error.get_or_insert(error);
            }
            Err(_) => {
                worker_error.get_or_insert_with(|| io::Error::other("worker panicked"));
            }
        };
    }
    let elapsed = started.elapsed();
    shutdown.request();
    let server_result = server
        .join()
        .map_err(|_| io::Error::other("server panicked"))?;
    server_result?;
    if let Some(error) = worker_error {
        return Err(error);
    }

    let seconds = elapsed.as_secs_f64();
    let operations_per_second = configuration.operations as f64 / seconds;
    println!(
        "workload={} operations={} value_size_bytes={} concurrency={} duration_seconds={seconds:.6} operations_per_second={operations_per_second:.2} os={} arch={} logical_cpus={} package_version={}",
        configuration.workload.name(),
        configuration.operations,
        configuration.value_size,
        configuration.concurrency,
        env::consts::OS,
        env::consts::ARCH,
        thread::available_parallelism().map_or(1, usize::from),
        env!("CARGO_PKG_VERSION"),
    );
    Ok(())
}

fn operations_for_worker(configuration: Configuration, worker: usize) -> usize {
    configuration.operations / configuration.concurrency
        + usize::from(worker < configuration.operations % configuration.concurrency)
}

fn run_worker(
    address: SocketAddr,
    configuration: Configuration,
    worker: usize,
    operations: usize,
    barrier: &Barrier,
) -> io::Result<()> {
    let setup = (|| {
        let stream = connect(address)?;
        stream.set_nodelay(true)?;
        let mut client = BufReader::new(stream);
        let key = format!("benchmark:{worker}");
        let value = vec![b'x'; configuration.value_size];
        send(
            &mut client,
            &[b"SET", key.as_bytes(), &value],
            Response::Simple,
        )?;
        Ok::<_, io::Error>((client, key, value))
    })();
    barrier.wait();
    let (mut client, key, value) = setup?;

    for operation in 0..operations {
        let is_set = match configuration.workload {
            Workload::Get => false,
            Workload::Set => true,
            Workload::Mixed => operation % 5 == 0,
        };
        if is_set {
            send(
                &mut client,
                &[b"SET", key.as_bytes(), &value],
                Response::Simple,
            )?;
        } else {
            send(
                &mut client,
                &[b"GET", key.as_bytes()],
                Response::Bulk(configuration.value_size),
            )?;
        }
    }
    Ok(())
}

fn connect(address: SocketAddr) -> io::Result<TcpStream> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match TcpStream::connect(address) {
            Ok(stream) => return Ok(stream),
            Err(_) if Instant::now() < deadline => thread::yield_now(),
            Err(error) => return Err(error),
        }
    }
}

enum Response {
    Simple,
    Bulk(usize),
}

fn send(
    client: &mut BufReader<TcpStream>,
    arguments: &[&[u8]],
    response: Response,
) -> io::Result<()> {
    let request = encode_request(arguments);
    client.get_mut().write_all(&request)?;
    let mut header = String::new();
    client.read_line(&mut header)?;
    match response {
        Response::Simple if header == "+OK\r\n" => Ok(()),
        Response::Bulk(length) if header == format!("${length}\r\n") => {
            let mut body = vec![0; length + 2];
            client.read_exact(&mut body)?;
            if body.ends_with(b"\r\n") {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid bulk response",
                ))
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unexpected response: {header:?}"),
        )),
    }
}

fn encode_request(arguments: &[&[u8]]) -> Vec<u8> {
    let mut request = format!("*{}\r\n", arguments.len()).into_bytes();
    for argument in arguments {
        request.extend_from_slice(format!("${}\r\n", argument.len()).as_bytes());
        request.extend_from_slice(argument);
        request.extend_from_slice(b"\r\n");
    }
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_configuration_and_rejects_invalid_values() {
        assert_eq!(parse_arguments(&[]), Ok(Configuration::default()));
        assert_eq!(
            parse_arguments(&[
                "--workload".into(),
                "get".into(),
                "--operations".into(),
                "11".into(),
                "--value-size".into(),
                "8".into(),
                "--concurrency".into(),
                "3".into(),
            ]),
            Ok(Configuration {
                workload: Workload::Get,
                operations: 11,
                value_size: 8,
                concurrency: 3
            })
        );
        for arguments in [
            vec!["--workload".into(), "unknown".into()],
            vec!["--operations".into(), "0".into()],
            vec!["--concurrency".into()],
            vec!["--unknown".into(), "1".into()],
        ] {
            assert!(parse_arguments(&arguments).is_err());
        }
    }

    #[test]
    fn distributes_every_operation_and_encodes_binary_arguments() {
        let configuration = Configuration {
            operations: 11,
            concurrency: 3,
            ..Configuration::default()
        };
        assert_eq!(
            (0..3)
                .map(|worker| operations_for_worker(configuration, worker))
                .collect::<Vec<_>>(),
            vec![4, 4, 3]
        );
        assert_eq!(
            encode_request(&[b"SET", b"key", b"a\0b"]),
            b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$3\r\na\0b\r\n"
        );
    }
}
