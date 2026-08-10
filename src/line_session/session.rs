use std::io::{self, BufRead, Write};

use crate::command::Command;
use crate::line_protocol::{self, ParsedLine};
use crate::output::CommandOutput;

pub(crate) fn run_session<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    prompt: Option<&str>,
    mut execute: F,
) -> io::Result<()>
where
    R: BufRead,
    W: Write,
    F: FnMut(Command) -> CommandOutput,
{
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

        match line_protocol::parse_line(&input) {
            ParsedLine::Empty => continue,
            ParsedLine::Error(output) => {
                output.write_to(writer)?;
            }
            ParsedLine::Command(command) => {
                let output = execute(command);

                match output {
                    CommandOutput::Exit => {
                        writeln!(writer, "Bye!")?;
                        return Ok(());
                    }
                    output => output.write_to(writer)?,
                }
            }
        }
    }
}
