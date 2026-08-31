use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::time::{Duration, Instant, SystemTime};

use super::in_memory::InMemoryStore;
use super::stored_value::StoredValue;
use super::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotEntry {
    pub(crate) key: Vec<u8>,
    pub(crate) value: SnapshotValue,
    pub(crate) expires_at_unix_millis: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SnapshotValue {
    String(Vec<u8>),
    List(Vec<Vec<u8>>),
    Set(Vec<Vec<u8>>),
    Hash(Vec<(Vec<u8>, Vec<u8>)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotDataError {
    TimeOutOfRange,
    DuplicateKey,
    DuplicateSetMember,
    DuplicateHashField,
    EmptyCollection,
    AllocationFailed,
}

impl fmt::Display for SnapshotDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimeOutOfRange => write!(
                formatter,
                "snapshot expiration is outside the supported time range"
            ),
            Self::DuplicateKey => write!(formatter, "snapshot contains a duplicate key"),
            Self::DuplicateSetMember => {
                write!(formatter, "snapshot contains a duplicate set member")
            }
            Self::DuplicateHashField => {
                write!(formatter, "snapshot contains a duplicate hash field")
            }
            Self::EmptyCollection => write!(formatter, "snapshot contains an empty collection"),
            Self::AllocationFailed => write!(formatter, "snapshot is too large to fit in memory"),
        }
    }
}

impl InMemoryStore {
    pub(crate) fn snapshot_entries(
        &mut self,
        wall_now: SystemTime,
    ) -> Result<Vec<SnapshotEntry>, SnapshotDataError> {
        self.remove_expired();

        let monotonic_now = self.clock.now();
        let mut keys: Vec<_> = self.storage.keys().cloned().collect();
        keys.sort();

        let mut entries = Vec::new();
        entries
            .try_reserve_exact(keys.len())
            .map_err(|_| SnapshotDataError::AllocationFailed)?;

        for key in keys {
            let stored = &self.storage[&key];
            let expires_at_unix_millis = stored
                .expires_at()
                .map(|expires_at| expiration_millis(expires_at, monotonic_now, wall_now))
                .transpose()?;

            entries.push(SnapshotEntry {
                key,
                value: snapshot_value(stored.typed_value()),
                expires_at_unix_millis,
            });
        }

        Ok(entries)
    }

    pub(crate) fn restore_snapshot(
        &mut self,
        entries: Vec<SnapshotEntry>,
        wall_now: SystemTime,
    ) -> Result<(), SnapshotDataError> {
        {
            let mut keys = HashSet::new();
            keys.try_reserve(entries.len())
                .map_err(|_| SnapshotDataError::AllocationFailed)?;
            for entry in &entries {
                if !keys.insert(entry.key.as_slice()) {
                    return Err(SnapshotDataError::DuplicateKey);
                }
            }
        }

        let monotonic_now = self.clock.now();
        let mut restored = HashMap::new();
        restored
            .try_reserve(entries.len())
            .map_err(|_| SnapshotDataError::AllocationFailed)?;

        for entry in entries {
            let value = restored_value(entry.value)?;
            let expires_at = match entry.expires_at_unix_millis {
                Some(milliseconds) => {
                    let expires_at_wall = SystemTime::UNIX_EPOCH
                        .checked_add(Duration::from_millis(milliseconds))
                        .ok_or(SnapshotDataError::TimeOutOfRange)?;

                    let Ok(remaining) = expires_at_wall.duration_since(wall_now) else {
                        continue;
                    };
                    if remaining.is_zero() {
                        continue;
                    }

                    Some(
                        monotonic_now
                            .checked_add(remaining)
                            .ok_or(SnapshotDataError::TimeOutOfRange)?,
                    )
                }
                None => None,
            };

            restored.insert(entry.key, StoredValue::from_parts(value, expires_at));
        }

        self.expirations.clear();
        for (key, entry) in &restored {
            if let Some(expires_at) = entry.expires_at() {
                self.expirations
                    .push(std::cmp::Reverse((expires_at, key.clone())));
            }
        }
        self.storage = restored;
        Ok(())
    }
}

fn expiration_millis(
    expires_at: Instant,
    monotonic_now: Instant,
    wall_now: SystemTime,
) -> Result<u64, SnapshotDataError> {
    let remaining = expires_at.saturating_duration_since(monotonic_now);
    let expires_at_wall = wall_now
        .checked_add(remaining)
        .ok_or(SnapshotDataError::TimeOutOfRange)?;
    let since_epoch = expires_at_wall
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| SnapshotDataError::TimeOutOfRange)?;

    u64::try_from(since_epoch.as_millis()).map_err(|_| SnapshotDataError::TimeOutOfRange)
}

fn snapshot_value(value: &Value) -> SnapshotValue {
    match value {
        Value::String(value) => SnapshotValue::String(value.clone()),
        Value::List(values) => SnapshotValue::List(values.iter().cloned().collect()),
        Value::Set(values) => {
            let mut values: Vec<_> = values.iter().cloned().collect();
            values.sort();
            SnapshotValue::Set(values)
        }
        Value::Hash(values) => {
            let mut values: Vec<_> = values
                .iter()
                .map(|(field, value)| (field.clone(), value.clone()))
                .collect();
            values.sort_by(|left, right| left.0.cmp(&right.0));
            SnapshotValue::Hash(values)
        }
    }
}

fn restored_value(value: SnapshotValue) -> Result<Value, SnapshotDataError> {
    match value {
        SnapshotValue::String(value) => Ok(Value::String(value)),
        SnapshotValue::List(values) if values.is_empty() => Err(SnapshotDataError::EmptyCollection),
        SnapshotValue::List(values) => Ok(Value::List(VecDeque::from(values))),
        SnapshotValue::Set(values) if values.is_empty() => Err(SnapshotDataError::EmptyCollection),
        SnapshotValue::Set(values) => {
            let mut members = HashSet::new();
            members
                .try_reserve(values.len())
                .map_err(|_| SnapshotDataError::AllocationFailed)?;
            for member in values {
                if !members.insert(member) {
                    return Err(SnapshotDataError::DuplicateSetMember);
                }
            }
            Ok(Value::Set(members))
        }
        SnapshotValue::Hash(values) if values.is_empty() => Err(SnapshotDataError::EmptyCollection),
        SnapshotValue::Hash(values) => {
            let mut fields = HashMap::new();
            fields
                .try_reserve(values.len())
                .map_err(|_| SnapshotDataError::AllocationFailed)?;
            for (field, value) in values {
                if fields.insert(field, value).is_some() {
                    return Err(SnapshotDataError::DuplicateHashField);
                }
            }
            Ok(Value::Hash(fields))
        }
    }
}
