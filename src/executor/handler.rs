use crate::command::Command;
use crate::database::Database;
use crate::response::Response;

pub(crate) fn execute(command: Command, database: &mut Database) -> Response {
    match command {
        Command::Set { key, value } => {
            database.set(key, value);
            Response::Ok
        }

        Command::MSet { entries } => {
            for (key, value) in entries {
                database.set(key, value);
            }

            Response::Ok
        }

        Command::SetNx { key, value } => Response::Integer(if database.set_if_absent(key, value) {
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

        Command::IncrementBy {
            key,
            amount: inc_value,
        } => match database.increment_by(key, inc_value) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::Decrement { key } => match database.decrement(key) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::DecrementBy {
            key,
            amount: decr_value,
        } => match database.decrement_by(key, decr_value) {
            Ok(value) => Response::Integer(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::IncrementByFloat { key, amount } => match database.incr_by_float(key, amount) {
            Ok(value) => Response::Float(value),
            Err(error) => Response::Error(error.to_string()),
        },

        Command::Delete { keys } => Response::Integer(database.delete_many(&keys) as i64),

        Command::Exists { keys } => Response::Integer(database.exists_many(&keys) as i64),

        Command::Keys => Response::Lines(database.keys()),

        Command::Rename { old_key, new_key } => {
            Response::Integer(if database.rename_key(&old_key, new_key) {
                1
            } else {
                0
            })
        }

        Command::Expire { key, seconds } => {
            let result = database.expire(&key, seconds);

            Response::Integer(if result { 1 } else { 0 })
        }

        Command::PExpire { key, milliseconds } => {
            let result = database.pexpire(&key, milliseconds);

            Response::Integer(if result { 1 } else { 0 })
        }

        Command::Ttl { key } => Response::Integer(database.ttl(&key)),

        Command::PTtl { key } => Response::Integer(database.pttl(&key)),

        Command::Persist { key } => Response::Integer(if database.persist(&key) { 1 } else { 0 }),

        Command::StrLen { key } => Response::Integer(database.string_length(&key) as i64),

        Command::GetRange { key, start, end } => {
            Response::Value(database.get_range(&key, start, end))
        }

        Command::SetRange { key, offset, value } => {
            Response::Integer(database.set_range(key, offset, value) as i64)
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
