use std::io::{self, BufRead, Write};

use crate::command::{Command, CommandError};
use crate::executor::execute;
use crate::output::CommandOutput;
use crate::storage::InMemoryStore;

pub fn run() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    run_with(stdin.lock(), stdout.lock())
}

fn run_with(mut reader: impl BufRead, mut writer: impl Write) -> io::Result<()> {
    let mut store = InMemoryStore::new();

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

        let command = match Command::parse(&input) {
            Ok(command) => command,
            Err(CommandError::EmptyInput) => continue,
            Err(error) => {
                writeln!(writer, "ERR {error}")?;
                continue;
            }
        };

        let output = execute(command, &mut store);

        if output == CommandOutput::Exit {
            writeln!(writer, "Bye!")?;
            break;
        }

        output.write_to(&mut writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
