use crate::command::Command;
use crate::output::CommandOutput;
use crate::storage::InMemoryStore;

pub(crate) fn execute(command: Command, store: &mut InMemoryStore) -> CommandOutput {
    match command {
        Command::Set { key, value } => {
            store.set(key, value);
            CommandOutput::Ok
        }

        Command::MSet { entries } => {
            for (key, value) in entries {
                store.set(key, value);
            }

            CommandOutput::Ok
        }

        Command::SetNx { key, value } => {
            CommandOutput::Integer(if store.set_if_absent(key, value) {
                1
            } else {
                0
            })
        }

        Command::Get { key } => match store.get(&key) {
            Ok(Some(value)) => CommandOutput::Value(value.to_owned()),
            Ok(None) => CommandOutput::Nil,
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::MGet { keys } => {
            let values: Result<Vec<Option<String>>, _> = keys
                .into_iter()
                .map(|key| store.get(&key).map(|value| value.map(str::to_owned)))
                .collect();

            match values {
                Ok(values) => CommandOutput::OptionalValues(values),
                Err(error) => CommandOutput::Error(error.to_string()),
            }
        }

        Command::GetSet { key, value } => match store.get_and_set(key, value) {
            Ok(Some(old_value)) => CommandOutput::Value(old_value),
            Ok(None) => CommandOutput::Nil,
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::GetDel { key } => match store.get_and_delete(key) {
            Ok(Some(old_value)) => CommandOutput::Value(old_value),
            Ok(None) => CommandOutput::Nil,
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::Append { key, append_value } => match store.append(&key, append_value) {
            Ok(length) => CommandOutput::Integer(length as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::Increment { key } => match store.increment(key) {
            Ok(value) => CommandOutput::Integer(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::IncrementBy {
            key,
            amount: inc_value,
        } => match store.increment_by(key, inc_value) {
            Ok(value) => CommandOutput::Integer(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::Decrement { key } => match store.decrement(key) {
            Ok(value) => CommandOutput::Integer(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::DecrementBy {
            key,
            amount: decr_value,
        } => match store.decrement_by(key, decr_value) {
            Ok(value) => CommandOutput::Integer(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::IncrementByFloat { key, amount } => match store.increment_by_float(key, amount) {
            Ok(value) => CommandOutput::Float(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::Delete { keys } => CommandOutput::Integer(store.delete_many(&keys) as i64),

        Command::Exists { keys } => CommandOutput::Integer(store.exists_many(&keys) as i64),

        Command::Keys => CommandOutput::KeyList(store.keys()),

        Command::Rename { old_key, new_key } => {
            CommandOutput::Integer(if store.rename(&old_key, new_key) {
                1
            } else {
                0
            })
        }

        Command::Expire { key, seconds } => {
            let result = store.expire(&key, seconds);

            CommandOutput::Integer(if result { 1 } else { 0 })
        }

        Command::PExpire { key, milliseconds } => {
            let result = store.pexpire(&key, milliseconds);

            CommandOutput::Integer(if result { 1 } else { 0 })
        }

        Command::Ttl { key } => CommandOutput::Integer(store.ttl(&key)),

        Command::PTtl { key } => CommandOutput::Integer(store.pttl(&key)),

        Command::Persist { key } => CommandOutput::Integer(if store.persist(&key) { 1 } else { 0 }),

        Command::StrLen { key } => match store.string_length(&key) {
            Ok(length) => CommandOutput::Integer(length as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::GetRange { key, start, end } => match store.get_range(&key, start, end) {
            Ok(value) => CommandOutput::Value(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::SetRange { key, offset, value } => match store.set_range(key, offset, value) {
            Ok(length) => CommandOutput::Integer(length as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::LPush { key, value } => match store.push_left(&key, value) {
            Ok(length) => CommandOutput::Integer(length as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::RPush { key, value } => match store.push_right(&key, value) {
            Ok(length) => CommandOutput::Integer(length as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::LLen { key } => match store.list_length(&key) {
            Ok(length) => CommandOutput::Integer(length as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::LPop { key } => match store.pop_left(&key) {
            Ok(Some(value)) => CommandOutput::Value(value),
            Ok(None) => CommandOutput::Nil,
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::RPop { key } => match store.pop_right(&key) {
            Ok(Some(value)) => CommandOutput::Value(value),
            Ok(None) => CommandOutput::Nil,
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::LRange { key, start, end } => match store.list_range(&key, start, end) {
            Ok(value) => CommandOutput::KeyList(value),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::Len => CommandOutput::Integer(store.len() as i64),

        Command::Clear => {
            store.clear();
            CommandOutput::Ok
        }

        Command::Help => CommandOutput::Help,

        Command::Exit => CommandOutput::Exit,
    }
}
