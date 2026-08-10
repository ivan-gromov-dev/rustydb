use std::io::{self, BufRead, Write};

use crate::database::Database;
use crate::line_protocol;
use crate::output::CommandOutput;

pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    run_with(stdin.lock(), stdout.lock())
}

fn run_with(mut reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let mut database = Database::default();

    writeln!(writer, "Rusty DB")?;
    writeln!(writer, "Type HELP to see available commands.")?;

    loop {
        write!(writer, "db> ")?;
        writer.flush()?;

        let mut input = String::new();
        let bytes_read = reader.read_line(&mut input)?;

        if bytes_read == 0 {
            writeln!(writer)?;
            break;
        }

        match line_protocol::process_line(&mut database, &input) {
            Some(CommandOutput::Exit) => {
                writeln!(writer, "Bye!")?;
                break;
            }
            Some(command) => command.write_to(&mut writer)?,
            None => continue,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
