use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::DEFAULT_SNAPSHOT_PATH;
use crate::database::Database;
use crate::line_session::run_session;

pub fn run() -> io::Result<()> {
    run_with_snapshot(DEFAULT_SNAPSHOT_PATH, false)
}

pub fn run_with_snapshot(
    snapshot_path: impl AsRef<Path>,
    save_on_shutdown: bool,
) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let database = Database::open(snapshot_path).map_err(io::Error::other)?;

    run_with_database(stdin.lock(), stdout.lock(), database, save_on_shutdown)
}

#[cfg(test)]
fn run_with(reader: impl BufRead, writer: impl Write) -> io::Result<()> {
    run_with_database(reader, writer, Database::default(), false)
}

fn run_with_database(
    mut reader: impl BufRead,
    mut writer: impl Write,
    mut database: Database,
    save_on_shutdown: bool,
) -> io::Result<()> {
    writeln!(writer, "Rusty DB")?;
    writeln!(writer, "Type HELP to see available commands.")?;

    let result = run_session(&mut reader, &mut writer, Some("db> "), |command| {
        database.execute(command)
    });

    if result.is_ok() && save_on_shutdown {
        database.save().map_err(io::Error::other)?;
    }

    result
}

#[cfg(test)]
mod tests;
