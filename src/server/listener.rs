use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::command::Command;
use crate::config::MemoryConfig;
use crate::database::Database;
use crate::logging::{self, LogLevel};
use crate::output::CommandOutput;
use crate::resp_session::run_session;

use super::shutdown::Shutdown;

const ACTIVE_EXPIRATION_LIMIT: usize = 20;

pub(crate) struct DatabaseState {
    pub(crate) database: Mutex<Database>,
    changed: Condvar,
}

pub(crate) type SharedDatabase = Arc<DatabaseState>;

pub(crate) fn shared_database(database: Database) -> SharedDatabase {
    Arc::new(DatabaseState {
        database: Mutex::new(database),
        changed: Condvar::new(),
    })
}

pub fn run_server(bind_address: &str) -> io::Result<()> {
    run_server_until(bind_address, Shutdown::default())
}

pub fn run_server_until(bind_address: &str, shutdown: Shutdown) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;

    run_server_on_listener(listener, shutdown)
}

pub fn run_server_until_with_snapshot(
    bind_address: &str,
    shutdown: Shutdown,
    snapshot_path: impl AsRef<Path>,
    save_on_shutdown: bool,
) -> io::Result<()> {
    run_server_until_with_snapshot_config(
        bind_address,
        shutdown,
        snapshot_path,
        save_on_shutdown,
        MemoryConfig::default(),
    )
}

pub fn run_server_until_with_snapshot_config(
    bind_address: &str,
    shutdown: Shutdown,
    snapshot_path: impl AsRef<Path>,
    save_on_shutdown: bool,
    memory_config: MemoryConfig,
) -> io::Result<()> {
    let database =
        Database::open_with_config(snapshot_path, memory_config).map_err(io::Error::other)?;
    let listener = TcpListener::bind(bind_address)?;

    run_server_on_listener_with_database(listener, shutdown, database, save_on_shutdown)
}

pub fn run_server_until_with_aof(
    bind_address: &str,
    shutdown: Shutdown,
    aof_path: impl AsRef<Path>,
) -> io::Result<()> {
    run_server_until_with_aof_config(bind_address, shutdown, aof_path, MemoryConfig::default())
}

pub fn run_server_until_with_aof_config(
    bind_address: &str,
    shutdown: Shutdown,
    aof_path: impl AsRef<Path>,
    memory_config: MemoryConfig,
) -> io::Result<()> {
    let database =
        Database::open_aof_with_config(aof_path, memory_config).map_err(io::Error::other)?;
    let listener = TcpListener::bind(bind_address)?;
    run_server_on_listener_with_database(listener, shutdown, database, false)
}

pub fn run_server_on_listener(listener: TcpListener, shutdown: Shutdown) -> io::Result<()> {
    run_server_on_listener_with_database(listener, shutdown, Database::default(), false)
}

pub(crate) fn run_server_on_listener_with_database(
    listener: TcpListener,
    shutdown: Shutdown,
    database: Database,
    save_on_shutdown: bool,
) -> io::Result<()> {
    listener.set_nonblocking(true)?;

    let database = shared_database(database);
    let mut workers = Vec::new();

    let result = loop {
        if shutdown.is_requested() {
            break Ok(());
        }

        {
            let mut database = lock_database(&database);
            database.active_expire(ACTIVE_EXPIRATION_LIMIT);
        }

        match listener.accept() {
            Ok((stream, _)) => workers.push(spawn_worker(stream, &database)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => break Err(error),
        }

        reap_finished(&mut workers);
    };

    drop(listener);
    join_all(workers);

    if result.is_ok() && save_on_shutdown {
        let mut database = lock_database(&database);
        database.save().map_err(io::Error::other)?;
    }

    result
}

fn spawn_worker(stream: TcpStream, database: &SharedDatabase) -> JoinHandle<()> {
    let database = Arc::clone(database);

    thread::spawn(move || {
        #[cfg(feature = "profiling")]
        super::profiling::mark_server_thread();
        let _ = handle_client(stream, database);
    })
}

#[cfg(test)]
pub(crate) fn serve_incoming<I>(incoming: I, database: &SharedDatabase) -> io::Result<()>
where
    I: IntoIterator<Item = io::Result<TcpStream>>,
{
    let mut workers = Vec::new();

    for stream in incoming {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                join_all(workers);
                return Err(error);
            }
        };

        workers.push(spawn_worker(stream, database));
        reap_finished(&mut workers);
    }

    join_all(workers);

    Ok(())
}

fn reap_finished(workers: &mut Vec<JoinHandle<()>>) {
    let mut index = 0;

    while index < workers.len() {
        if workers[index].is_finished() {
            let worker = workers.swap_remove(index);
            let _ = worker.join();
        } else {
            index += 1;
        }
    }
}

fn join_all(workers: Vec<JoinHandle<()>>) {
    for worker in workers {
        let _ = worker.join();
    }
}

fn handle_client(mut stream: TcpStream, database: SharedDatabase) -> io::Result<()> {
    stream.set_nonblocking(false)?;

    let mut reader = stream.try_clone()?;
    let disconnect_probe = stream.try_clone()?;

    with_database(&database, Database::client_connected);
    logging::event(LogLevel::Debug, "client_connected", &[]);

    let result = run_session(&mut reader, &mut stream, |command| {
        execute_server_command(&database, &disconnect_probe, command)
    });

    with_database(&database, Database::client_disconnected);
    logging::event(LogLevel::Debug, "client_disconnected", &[]);

    result
}

pub(crate) fn execute_shared(database: &SharedDatabase, command: Command) -> CommandOutput {
    #[cfg(feature = "profiling")]
    let started = std::time::Instant::now();
    let output = with_database(database, |database| {
        #[cfg(feature = "profiling")]
        super::profiling::record_lock_wait(started.elapsed());
        database.execute(command)
    });
    if !output.is_error() {
        database.changed.notify_all();
    }
    output
}

fn with_database<T>(database: &SharedDatabase, operation: impl FnOnce(&mut Database) -> T) -> T {
    let mut database = lock_database(database);

    operation(&mut database)
}

fn lock_database(database: &SharedDatabase) -> MutexGuard<'_, Database> {
    match database.database.lock() {
        Ok(database) => database,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn execute_server_command(
    database: &SharedDatabase,
    disconnect_probe: &TcpStream,
    command: Command,
) -> CommandOutput {
    match command {
        Command::BLPop { keys, timeout } => {
            execute_blocking_pop(database, disconnect_probe, keys, false, timeout)
        }
        Command::BRPop { keys, timeout } => {
            execute_blocking_pop(database, disconnect_probe, keys, true, timeout)
        }
        Command::BLMove {
            source,
            destination,
            source_end,
            destination_end,
            timeout,
        } => execute_blocking_move(
            database,
            disconnect_probe,
            source,
            destination,
            source_end,
            destination_end,
            timeout,
        ),
        command => execute_shared(database, command),
    }
}

fn execute_blocking_pop(
    database: &SharedDatabase,
    disconnect_probe: &TcpStream,
    keys: Vec<Vec<u8>>,
    right: bool,
    timeout: f64,
) -> CommandOutput {
    let deadline = blocking_deadline(timeout);
    let mut guard = lock_database(database);
    loop {
        let output = guard.try_blocking_pop(&keys, right);
        if output != CommandOutput::Nil {
            database.changed.notify_all();
            return output;
        }
        let Some(wait) = blocking_wait(deadline, disconnect_probe) else {
            return CommandOutput::NullArray;
        };
        guard = wait_for_change(database, guard, wait);
    }
}

fn execute_blocking_move(
    database: &SharedDatabase,
    disconnect_probe: &TcpStream,
    source: Vec<u8>,
    destination: Vec<u8>,
    source_end: crate::command::ListEnd,
    destination_end: crate::command::ListEnd,
    timeout: f64,
) -> CommandOutput {
    let deadline = blocking_deadline(timeout);
    let mut guard = lock_database(database);
    loop {
        let output = guard.try_blocking_move(
            source.clone(),
            destination.clone(),
            source_end,
            destination_end,
        );
        if output != CommandOutput::Nil {
            database.changed.notify_all();
            return output;
        }
        let Some(wait) = blocking_wait(deadline, disconnect_probe) else {
            return CommandOutput::Nil;
        };
        guard = wait_for_change(database, guard, wait);
    }
}

fn blocking_deadline(timeout: f64) -> Option<Instant> {
    (timeout > 0.0).then(|| Instant::now() + Duration::from_secs_f64(timeout))
}

fn blocking_wait(deadline: Option<Instant>, stream: &TcpStream) -> Option<Duration> {
    if connection_closed(stream) {
        return None;
    }
    let poll = Duration::from_millis(25);
    match deadline {
        Some(deadline) => {
            let remaining = deadline.checked_duration_since(Instant::now())?;
            Some(remaining.min(poll))
        }
        None => Some(poll),
    }
}

fn wait_for_change<'a>(
    database: &SharedDatabase,
    guard: MutexGuard<'a, Database>,
    wait: Duration,
) -> MutexGuard<'a, Database> {
    match database.changed.wait_timeout(guard, wait) {
        Ok((guard, _)) => guard,
        Err(poisoned) => poisoned.into_inner().0,
    }
}

fn connection_closed(stream: &TcpStream) -> bool {
    if stream.set_nonblocking(true).is_err() {
        return true;
    }
    let result = match stream.peek(&mut [0; 1]) {
        Ok(0) => true,
        Ok(_) => false,
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => false,
        Err(error) if error.kind() == io::ErrorKind::Interrupted => false,
        Err(_) => true,
    };
    let _ = stream.set_nonblocking(false);
    result
}
