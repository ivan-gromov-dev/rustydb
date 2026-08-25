use std::env;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use rustydb::{Shutdown, run_server_on_listener};

#[cfg(feature = "profiling")]
use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(feature = "profiling")]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[cfg(feature = "profiling")]
struct CountingAllocator;

#[cfg(feature = "profiling")]
static COUNT_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "profiling")]
static ALLOCATION_EVENTS: [AtomicU64; rustydb::ProfilePhase::COUNT] =
    [const { AtomicU64::new(0) }; rustydb::ProfilePhase::COUNT];
#[cfg(feature = "profiling")]
static ALLOCATED_BYTES: [AtomicU64; rustydb::ProfilePhase::COUNT] =
    [const { AtomicU64::new(0) }; rustydb::ProfilePhase::COUNT];

#[cfg(feature = "profiling")]
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[cfg(feature = "profiling")]
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            record_allocation(layout.size());
        }
        // SAFETY: the layout is forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer and layout came from the system allocator.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            record_allocation(new_size);
        }
        // SAFETY: the pointer and layout came from the system allocator.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[cfg(feature = "profiling")]
fn record_allocation(bytes: usize) {
    let index = rustydb::profiling_phase().index();
    ALLOCATION_EVENTS[index].fetch_add(1, Ordering::Relaxed);
    ALLOCATED_BYTES[index].fetch_add(bytes as u64, Ordering::Relaxed);
}

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
    #[cfg(feature = "profiling")]
    {
        for counter in ALLOCATION_EVENTS.iter().chain(&ALLOCATED_BYTES) {
            counter.store(0, Ordering::Relaxed);
        }
        rustydb::reset_lock_profile();
        COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    }
    let started = Instant::now();
    barrier.wait();
    barrier.wait();
    let mut worker_error = None;
    let elapsed = started.elapsed();
    #[cfg(feature = "profiling")]
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);
    barrier.wait();
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
    #[cfg(feature = "profiling")]
    let profile = {
        let lock = rustydb::lock_profile();
        let events: [u64; rustydb::ProfilePhase::COUNT] =
            std::array::from_fn(|index| ALLOCATION_EVENTS[index].load(Ordering::Relaxed));
        let bytes: [u64; rustydb::ProfilePhase::COUNT] =
            std::array::from_fn(|index| ALLOCATED_BYTES[index].load(Ordering::Relaxed));
        let client = rustydb::ProfilePhase::ClientRunner.index();
        let other = rustydb::ProfilePhase::ServerOther.index();
        let decode = rustydb::ProfilePhase::Decode.index();
        let command = rustydb::ProfilePhase::Command.index();
        let execute = rustydb::ProfilePhase::Execute.index();
        let response = rustydb::ProfilePhase::Response.index();
        let server_events: u64 = events[other..].iter().sum();
        let server_bytes: u64 = bytes[other..].iter().sum();
        format!(
            " allocation_events={} allocated_bytes={} server_allocation_events={} server_allocated_bytes={} client_runner_allocation_events={} client_runner_allocated_bytes={} server_other_allocation_events={} server_other_allocated_bytes={} decode_allocation_events={} decode_allocated_bytes={} command_allocation_events={} command_allocated_bytes={} execute_allocation_events={} execute_allocated_bytes={} response_allocation_events={} response_allocated_bytes={} lock_acquisitions={} lock_wait_nanoseconds={} lock_max_wait_nanoseconds={}",
            server_events.saturating_add(events[client]),
            server_bytes.saturating_add(bytes[client]),
            server_events,
            server_bytes,
            events[client],
            bytes[client],
            events[other],
            bytes[other],
            events[decode],
            bytes[decode],
            events[command],
            bytes[command],
            events[execute],
            bytes[execute],
            events[response],
            bytes[response],
            lock.acquisitions,
            lock.wait_nanoseconds,
            lock.max_wait_nanoseconds,
        )
    };
    #[cfg(not(feature = "profiling"))]
    let profile = "";
    println!(
        "workload={} operations={} value_size_bytes={} concurrency={} duration_seconds={seconds:.6} operations_per_second={operations_per_second:.2} os={} arch={} logical_cpus={} package_version={}{}",
        configuration.workload.name(),
        configuration.operations,
        configuration.value_size,
        configuration.concurrency,
        env::consts::OS,
        env::consts::ARCH,
        thread::available_parallelism().map_or(1, usize::from),
        env!("CARGO_PKG_VERSION"),
        profile,
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
    barrier.wait();
    let result = setup.and_then(|(mut client, key, value)| {
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
    });
    barrier.wait();
    barrier.wait();
    result
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
