use std::io::{self, BufRead, Write};

use crate::database::Database;
use crate::line_session::run_session;

pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    run_with(stdin.lock(), stdout.lock())
}

fn run_with(mut reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let mut database = Database::default();

    writeln!(writer, "Rusty DB")?;
    writeln!(writer, "Type HELP to see available commands.")?;

    run_session(&mut reader, &mut writer, Some("db> "), |command| {
        database.execute(command)
    })
}

#[cfg(test)]
mod tests;
