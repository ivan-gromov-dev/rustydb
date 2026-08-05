use std::io::{self, Write};

use crate::command::{Command, CommandError};
use crate::database::Database;
use crate::executor::execute;
use crate::response::Response;

pub fn run() -> io::Result<()> {
    let mut database = Database::new();

    println!("Rusty DB");
    println!("Type HELP to see available commands.");

    loop {
        print!("db> ");
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;

        if bytes_read == 0 {
            println!();
            break;
        }

        let command = match Command::parse(&input) {
            Ok(command) => command,

            Err(CommandError::EmptyInput) => {
                continue;
            }

            Err(error) => {
                println!("ERR {error}");
                continue;
            }
        };

        let response = execute(command, &mut database);

        if response == Response::Exit {
            println!("Bye!");
            break;
        }

        response.print();
    }

    Ok(())
}
