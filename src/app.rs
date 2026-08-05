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

            Command::Mset { entries } => {
                for entry in entries {
                    database.set(entry.0, entry.1);
                }
                println!("OK")
            }

            Command::SetNX { key, value } => {
                let inserted = database.set_if_absent(key, value);
                println!("{}", if inserted { 1 } else { 0 });
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

            Command::GetSet { key, value } => match database.get_and_set(key, value) {
                Some(old_value) => println!("{old_value}"),
                None => println!("(nil)"),
            },

            Command::Append { key, append_value } => {
                println!("{}", database.append(&key, append_value));
            }

            Command::Increment { key } => match database.increment(key) {
                Ok(value) => println!("{value}"),
                Err(err) => println!("ERR {err}"),
            },

            Command::IncrementBy { key, inc_value } => {
                match database.increment_by(key, inc_value) {
                    Ok(result) => println!("{result}"),
                    Err(err) => println!("ERR {err}"),
                }
            }

            Command::Decrement { key } => match database.decrement(key) {
                Ok(value) => println!("{value}"),
                Err(err) => println!("ERR {err}"),
            },

            Command::DecrementBy { key, decr_value } => {
                match database.decrement_by(key, decr_value) {
                    Ok(result) => println!("{result}"),
                    Err(err) => println!("ERR {err}"),
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
    println!("  MSET key value [key value ...]");
    println!("  SETNX key value");
    println!("  GET key");
    println!("  MGET key [key ...]");
    println!("  GETSET key value");
    println!("  APPEND key value");
    println!("  INCR key");
    println!("  INCRBY key inc_value");
    println!("  DECR key");
    println!("  DECRBY key decr_value");
    println!("  EXISTS key");
    println!("  DEL key");
    println!("  RENAME old_key new_key");
    println!("  LEN");
    println!("  CLEAR");
    println!("  HELP");
    println!("  EXIT");
}
