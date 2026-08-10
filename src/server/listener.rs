use std::io::{self, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::command::Command;
use crate::database::Database;
use crate::line_session::run_session;
use crate::output::CommandOutput;

pub(crate) type SharedDatabase = Arc<Mutex<Database>>;

pub fn run_server(bind_address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;

    serve(listener)
}

fn serve(listener: TcpListener) -> io::Result<()> {
    let database = Arc::new(Mutex::new(Database::default()));

    serve_incoming(listener.incoming(), &database)
}

pub(crate) fn serve_incoming<I>(incoming: I, database: &SharedDatabase) -> io::Result<()>
where
    I: IntoIterator<Item = io::Result<TcpStream>>,
{
    for stream in incoming {
        let stream = stream?;
        let database = Arc::clone(database);

        let _ = thread::spawn(move || {
            let _ = handle_client(stream, database);
        });
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream, database: SharedDatabase) -> io::Result<()> {
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
