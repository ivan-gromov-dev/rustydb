use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::command::Command;
use crate::config::MemoryConfig;
use crate::database::Database;
use crate::output::CommandOutput;
use crate::resp_session::run_session;

use super::shutdown::Shutdown;

const ACTIVE_EXPIRATION_LIMIT: usize = 20;

pub(crate) type SharedDatabase = Arc<Mutex<Database>>;

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

    let database = Arc::new(Mutex::new(database));
    let mut workers = Vec::new();

    let result = loop {
        if shutdown.is_requested() {
            break Ok(());
        }

        {
            let mut database = match database.lock() {
                Ok(database) => database,
                Err(poisoned) => poisoned.into_inner(),
            };
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
        let mut database = match database.lock() {
            Ok(database) => database,
            Err(poisoned) => poisoned.into_inner(),
        };
        database.save().map_err(io::Error::other)?;
    }

    result
}

fn spawn_worker(stream: TcpStream, database: &SharedDatabase) -> JoinHandle<()> {
    let database = Arc::clone(database);

    thread::spawn(move || {
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

    with_database(&database, Database::client_connected);

    let result = run_session(&mut reader, &mut stream, |command| {
        execute_shared(&database, command)
    });

    with_database(&database, Database::client_disconnected);

    result
}

pub(crate) fn execute_shared(database: &SharedDatabase, command: Command) -> CommandOutput {
    with_database(database, |database| database.execute(command))
}

fn with_database<T>(database: &SharedDatabase, operation: impl FnOnce(&mut Database) -> T) -> T {
    let mut database = match database.lock() {
        Ok(database) => database,
        Err(poisoned) => poisoned.into_inner(),
    };

    operation(&mut database)
}
