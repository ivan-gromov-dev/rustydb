use std::io::{self, BufRead, Write};

use crate::database::Database;
use crate::line_protocol;
use crate::output::CommandOutput;

pub(crate) fn run_session(
    reader: &mut impl BufRead,
    writer: &mut impl Write,
    database: &mut Database,
    prompt: Option<&str>,
) -> io::Result<()> {
    let mut input = String::new();

    loop {
        if let Some(prompt) = prompt {
            write!(writer, "{prompt}")?;
            writer.flush()?;
        }

        input.clear();
        let bytes_read = reader.read_line(&mut input)?;

        if bytes_read == 0 {
            if prompt.is_some() {
                writeln!(writer)?;
            }
            return Ok(());
        }

        match line_protocol::process_line(database, &input) {
            Some(CommandOutput::Exit) => {
                writeln!(writer, "Bye!")?;
                return Ok(());
            }
            Some(output) => output.write_to(writer)?,
            None => continue,
        }
    }
}
