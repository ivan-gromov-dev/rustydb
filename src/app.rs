use std::io::{self, Write};

use crate::command::{Command, CommandError};
use crate::database::Database;

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

            Err(err) => {
                println!("ERR {err}");
                continue;
            }
        };

        match command {
            Command::Set { key, value } => {
                database.set(key, value);
                println!("OK");
            }

            Command::Get { key } => match database.get(&key) {
                Some(value) => println!("{value}"),
                None => println!("(nil)"),
            },

            Command::MGet { keys } => {
                for key in keys {
                    match database.get(&key) {
                        Some(value) => println!("{value}"),
                        None => println!("(nil)"),
                    }
                }
            }

            Command::Exists { key } => {
                println!("{}", if database.exists(&key) { 1 } else { 0 });
            }

            Command::Delete { key } => {
                println!("{}", if database.delete(&key) { 1 } else { 0 });
            }

            Command::Keys => {
                let keys = database.keys();

                if keys.is_empty() {
                    println!("(nil)")
                }

                for key in keys {
                    println!("{key}")
                }
            }

            Command::Rename { old_key, new_key } => {
                println!(
                    "{}",
                    if database.rename_key(&old_key, new_key) {
                        1
                    } else {
                        0
                    }
                );
            }

            Command::Len => {
                println!("{}", database.len());
            }

            Command::Clear => {
                database.clear();
                println!("OK")
            }

            Command::Help => {
                print_help();
            }

            Command::Exit => {
                println!("Bye!");
                break;
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("Available commands:");
    println!("  SET key value");
    println!("  GET key");
    println!("  EXISTS key");
    println!("  DEL key");
    println!("  RENAME old_key new_key");
    println!("  LEN");
    println!("  CLEAR");
    println!("  HELP");
    println!("  EXIT");
}
