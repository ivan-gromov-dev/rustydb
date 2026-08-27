use crate::command::Command;
use crate::command::{COMMANDS, command_metadata};
use crate::output::CommandOutput;
use crate::snapshot;
use crate::storage::InMemoryStore;
use std::path::Path;

#[cfg(test)]
pub(crate) fn execute(command: Command, store: &mut InMemoryStore) -> CommandOutput {
    execute_with_snapshot(command, store, None)
}

pub(crate) fn execute_with_snapshot(
    command: Command,
    store: &mut InMemoryStore,
    snapshot_path: Option<&Path>,
) -> CommandOutput {
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
            Ok(Some(value)) => CommandOutput::Value(value.to_vec()),
            Ok(None) => CommandOutput::Nil,
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::MGet { keys } => {
            let values: Result<Vec<Option<Vec<u8>>>, _> = keys
                .into_iter()
                .map(|key| store.get(&key).map(|value| value.map(<[u8]>::to_vec)))
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

        Command::SAdd { key, member } => match store.set_add(&key, member) {
            Ok(added) => CommandOutput::Integer(i64::from(added)),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::SRem { key, member } => match store.set_remove(&key, &member) {
            Ok(removed) => CommandOutput::Integer(i64::from(removed)),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::SIsMember { key, member } => match store.set_contains(&key, &member) {
            Ok(found) => CommandOutput::Integer(i64::from(found)),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::SMembers { key } => match store.set_members(&key) {
            Ok(members) => CommandOutput::KeyList(members),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::SCard { key } => match store.set_cardinality(&key) {
            Ok(cardinality) => CommandOutput::Integer(cardinality as i64),
            Err(error) => CommandOutput::Error(error.to_string()),
        },

        Command::Ping { message: None } => CommandOutput::Pong,

        Command::Ping {
            message: Some(message),
        }
        | Command::Echo { message } => CommandOutput::Value(message),

        Command::Hello { protocol } => CommandOutput::Hello {
            protocol,
            connection_id: None,
        },

        Command::ClientId
        | Command::ClientSetName { .. }
        | Command::ClientGetName
        | Command::ClientSetInfo { .. } => {
            CommandOutput::Error("CLIENT requires a server connection".to_owned())
        }

        Command::MetadataList => {
            CommandOutput::CommandMetadata(COMMANDS.iter().copied().map(Some).collect())
        }

        Command::MetadataInfo { names } => CommandOutput::CommandMetadata(if names.is_empty() {
            COMMANDS.iter().copied().map(Some).collect()
        } else {
            names.iter().map(|name| command_metadata(name)).collect()
        }),

        Command::MetadataCount => CommandOutput::Integer(COMMANDS.len() as i64),

        Command::Select => CommandOutput::Ok,

        Command::DbSize => CommandOutput::Integer(store.len() as i64),

        Command::FlushDb | Command::FlushAll => {
            store.clear();
            CommandOutput::Ok
        }

        Command::Len => CommandOutput::Integer(store.len() as i64),

        Command::Clear => {
            store.clear();
            CommandOutput::Ok
        }

        Command::Save => match snapshot_path {
            Some(path) => match snapshot::save(path, store) {
                Ok(()) => CommandOutput::Ok,
                Err(error) => CommandOutput::Error(format!("snapshot save failed: {error}")),
            },
            None => CommandOutput::Error("snapshot path is not configured".to_owned()),
        },

        Command::AofRewrite => CommandOutput::Error("AOF is not configured".to_owned()),

        Command::Info => CommandOutput::Error("INFO requires database metrics".to_owned()),

        Command::Help => CommandOutput::Help,

        Command::Exit => CommandOutput::Exit,
    }
}
