use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::DEFAULT_SNAPSHOT_PATH;
use crate::config::MemoryConfig;
use crate::database::Database;
use crate::line_session::run_session;

pub fn run() -> io::Result<()> {
    run_with_snapshot(DEFAULT_SNAPSHOT_PATH, false)
}

pub fn run_with_snapshot(
    snapshot_path: impl AsRef<Path>,
    save_on_shutdown: bool,
) -> io::Result<()> {
    run_with_snapshot_config(snapshot_path, save_on_shutdown, MemoryConfig::default())
}

pub fn run_with_snapshot_config(
    snapshot_path: impl AsRef<Path>,
    save_on_shutdown: bool,
    memory_config: MemoryConfig,
) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let database =
        Database::open_with_config(snapshot_path, memory_config).map_err(io::Error::other)?;

    run_with_database(stdin.lock(), stdout.lock(), database, save_on_shutdown)
}

pub fn run_with_aof(aof_path: impl AsRef<Path>) -> io::Result<()> {
    run_with_aof_config(aof_path, MemoryConfig::default())
}

pub fn run_with_aof_config(
    aof_path: impl AsRef<Path>,
    memory_config: MemoryConfig,
) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let database =
        Database::open_aof_with_config(aof_path, memory_config).map_err(io::Error::other)?;
    run_with_database(stdin.lock(), stdout.lock(), database, false)
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
