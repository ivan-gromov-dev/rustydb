use std::io::{self, BufReader};
use std::net::{TcpListener, TcpStream};

use crate::database::Database;
use crate::line_session::run_session;

pub fn run_server(bind_address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(bind_address)?;

    serve(listener)
}

pub(crate) fn serve(listener: TcpListener) -> io::Result<()> {
    let mut database = Database::default();
    let (stream, _) = listener.accept()?;

    handle_client(stream, &mut database)
}

fn handle_client(mut stream: TcpStream, database: &mut Database) -> io::Result<()> {
    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);

    run_session(&mut reader, &mut stream, database, None)
}
