use crate::command::Command;
use crate::database::Database;
use crate::response::Response;

pub(crate) fn execute(command: Command, database: &mut Database) -> Response {
    match command {
        Command::Set { key, value } => {
            database.set(key, value);
            Response::Ok
        }

        Command::Mset { entries } => {
            for (key, value) in entries {
                database.set(key, value);
            }

            Response::Ok
        }

        Command::SetNX { key, value } => Response::Integer(if database.set_if_absent(key, value) {
            1
        } else {
            0
        }),

        Command::Get { key } => match database.get(&key) {
            Some(value) => Response::Value(value.to_owned()),
            None => Response::Nil,
        },

        Command::MGet { keys } => {
            let values = keys
                .into_iter()
                .map(|key| database.get(&key).map(str::to_owned))
                .collect();

            Response::Values(values)
        }

        Command::GetSet { key, value } => match database.get_and_set(key, value) {
            Some(old_value) => Response::Value(old_value),
            None => Response::Nil,
        },

        Command::GetDel { key } => match database.get_and_delete(key) {
            Some(old_value) => Response::Value(old_value),
            None => Response::Nil,
        },

        Command::Append { key, append_value } => {
            Response::Integer(database.append(&key, append_value) as i64)
        }

        Command::Increment { key } => match database.increment(key) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::IncrementBy { key, inc_value } => match database.increment_by(key, inc_value) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::Decrement { key } => match database.decrement(key) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::DecrementBy { key, decr_value } => match database.decrement_by(key, decr_value) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::Exists { key } => Response::Integer(if database.exists(&key) { 1 } else { 0 }),

        Command::Delete { key } => Response::Integer(if database.delete(&key) { 1 } else { 0 }),

        Command::Keys => {
            let keys = database.keys().into_iter().map(str::to_owned).collect();

            Response::Lines(keys)
        }

        Command::Rename { old_key, new_key } => {
            Response::Integer(if database.rename_key(&old_key, new_key) {
                1
            } else {
                0
            })
        }

        Command::Len => Response::Integer(database.len() as i64),

        Command::Clear => {
            database.clear();
            Response::Ok
        }

        Command::Help => Response::Help,

        Command::Exit => Response::Exit,
    }
}
