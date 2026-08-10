use std::io::{self, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::command::Command;
use crate::database::Database;
use crate::line_session::run_session;
use crate::output::CommandOutput;

use super::shutdown::Shutdown;

pub(crate) type SharedDatabase = Arc<Mutex<Database>>;

pub fn run_server(bind_address: &str) -> io::Result<()> {
    run_server_until(bind_address, Shutdown::default())
}

pub fn run_server_until(bind_address: &str, shutdown: Shutdown) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;

    run_server_on_listener(listener, shutdown)
}

pub fn run_server_on_listener(listener: TcpListener, shutdown: Shutdown) -> io::Result<()> {
    listener.set_nonblocking(true)?;

    let database = Arc::new(Mutex::new(Database::default()));
    let mut workers = Vec::new();

    let result = loop {
        if shutdown.is_requested() {
            break Ok(());
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

    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);

    run_session(&mut reader, &mut stream, None, move |command| {
        execute_shared(&database, command)
    })
}

pub(crate) fn execute_shared(database: &SharedDatabase, command: Command) -> CommandOutput {
    let mut database = match database.lock() {
        Ok(database) => database,
        Err(poisoned) => poisoned.into_inner(),
    };

    database.execute(command)
}
