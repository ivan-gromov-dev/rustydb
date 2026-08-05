#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Response {
    Ok,
    Integer(i64),
    Value(String),
    Values(Vec<Option<String>>),
    Nil,
    Lines(Vec<String>),
    Error(String),
    Help,
    Exit,
}

impl Response {
    pub(crate) fn print(self) {
        match self {
            Self::Ok => println!("OK"),

            Self::Integer(value) => {
                println!("{value}");
            }

            Self::Value(value) => {
                println!("{value}");
            }

            Self::Values(values) => {
                for value in values {
                    match value {
                        Some(value) => println!("{value}"),
                        None => println!("(nil)"),
                    }
                }
            }

            Self::Nil => {
                println!("(nil)");
            }

            Self::Lines(lines) => {
                if lines.is_empty() {
                    println!("(nil)");
                } else {
                    for line in lines {
                        println!("{line}");
                    }
                }
            }

            Self::Error(error) => {
                println!("ERR {error}");
            }

            Self::Help => {
                print_help();
            }

            Self::Exit => {}
        }
    }
}

fn print_help() {
    println!("Available commands:");
    println!("  SET key value");
    println!("  MSET key value [key value ...]");
    println!("  SETNX key value");
    println!("  GET key");
    println!("  MGET key [key ...]");
    println!("  GETSET key value");
    println!("  GETDEL key");
    println!("  APPEND key value");
    println!("  INCR key");
    println!("  INCRBY key inc_value");
    println!("  DECR key");
    println!("  DECRBY key decr_value");
    println!("  EXISTS key");
    println!("  DEL key");
    println!("  RENAME old_key new_key");
    println!("  KEYS");
    println!("  LEN");
    println!("  CLEAR");
    println!("  HELP");
    println!("  EXIT");
}
