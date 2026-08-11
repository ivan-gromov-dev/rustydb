use super::indexing::normalize_index;
use super::stored_value::StoredValue;
use crate::storage::clock::{Clock, SystemClock};
use std::collections::{HashMap, hash_map::Entry as HashMapEntry};
use std::fmt;
use std::str;
use std::time::{Duration, Instant};

pub(crate) struct InMemoryStore {
    storage: HashMap<Vec<u8>, StoredValue>,
    clock: Box<dyn Clock>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StoreError {
    ValueIsNotInteger,
    IntegerOverflow,
    ValueIsNotFloat,
    FloatIsNotFinite,
    WrongType,
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueIsNotInteger => {
                write!(formatter, "value is not integer")
            }

            Self::IntegerOverflow => {
                write!(formatter, "integer overflow")
            }

            Self::ValueIsNotFloat => {
                write!(formatter, "value is not float")
            }

            Self::FloatIsNotFinite => {
                write!(formatter, "float is not finite")
            }

            Self::WrongType => {
                write!(
                    formatter,
                    "operation against a key holding the wrong kind of value"
                )
            }
        }
    }
}

impl InMemoryStore {
    pub(crate) fn new() -> Self {
        Self::with_clock(Box::new(SystemClock))
    }

    pub(crate) fn with_clock(clock: Box<dyn Clock>) -> Self {
        Self {
            storage: HashMap::new(),
            clock,
        }
    }

    pub(crate) fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.storage.insert(key, StoredValue::new(value));
    }

    pub(crate) fn get(&mut self, key: impl AsRef<[u8]>) -> Result<Option<&[u8]>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        self.storage.get(key).map(StoredValue::value).transpose()
    }

    pub(crate) fn exists(&mut self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        self.storage.contains_key(key)
    }

    pub(crate) fn delete(&mut self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        self.storage.remove(key).is_some()
    }

    pub(crate) fn len(&mut self) -> usize {
        self.remove_expired();

        self.storage.len()
    }

    pub(crate) fn clear(&mut self) {
        self.storage.clear();
    }

    pub(crate) fn keys(&mut self) -> Vec<Vec<u8>> {
        self.remove_expired();

        let mut keys: Vec<_> = self.storage.keys().cloned().collect();

        keys.sort();
        keys
    }

    pub(crate) fn rename(&mut self, old_key: impl AsRef<[u8]>, new_key: Vec<u8>) -> bool {
        let old_key = old_key.as_ref();
        self.remove_if_expired(old_key);
        self.remove_if_expired(&new_key);

        match self.storage.remove(old_key) {
            Some(entry) => {
                self.storage.insert(new_key, entry);
                true
            }

            None => false,
        }
    }

    pub(crate) fn append(
        &mut self,
        key: impl AsRef<[u8]>,
        append_value: Vec<u8>,
    ) -> Result<usize, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let stored_value = self
            .storage
            .entry(key.to_vec())
            .or_insert_with(|| StoredValue::new(Vec::new()))
            .value_mut()?;

        stored_value.extend(append_value);
        Ok(stored_value.len())
    }

    pub(crate) fn increment(&mut self, key: Vec<u8>) -> Result<i64, StoreError> {
        self.increment_by(key, 1)
    }

    pub(crate) fn increment_by(&mut self, key: Vec<u8>, amount: i64) -> Result<i64, StoreError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => parse_integer(entry.value()?)?,

            None => 0,
        };

        let incremented = number
            .checked_add(amount)
            .ok_or(StoreError::IntegerOverflow)?;

        match self.storage.get_mut(&key) {
            Some(entry) => {
                entry.set_value(incremented.to_string().into_bytes());
            }

            None => {
                self.storage
                    .insert(key, StoredValue::new(incremented.to_string().into_bytes()));
            }
        }

        Ok(incremented)
    }

    pub(crate) fn decrement(&mut self, key: Vec<u8>) -> Result<i64, StoreError> {
        self.decrement_by(key, 1)
    }

    pub(crate) fn decrement_by(&mut self, key: Vec<u8>, amount: i64) -> Result<i64, StoreError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => parse_integer(entry.value()?)?,

            None => 0,
        };

        let decremented = number
            .checked_sub(amount)
            .ok_or(StoreError::IntegerOverflow)?;

        match self.storage.get_mut(&key) {
            Some(entry) => {
                entry.set_value(decremented.to_string().into_bytes());
            }

            None => {
                self.storage
                    .insert(key, StoredValue::new(decremented.to_string().into_bytes()));
            }
        }

        Ok(decremented)
    }

    pub(crate) fn increment_by_float(
        &mut self,
        key: Vec<u8>,
        amount: f64,
    ) -> Result<f64, StoreError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => parse_float(entry.value()?)?,

            None => 0.0,
        };

        if !number.is_finite() {
            return Err(StoreError::ValueIsNotFloat);
        }

        let result = number + amount;

        if !result.is_finite() {
            return Err(StoreError::FloatIsNotFinite);
        }

        match self.storage.get_mut(&key) {
            Some(entry) => {
                entry.set_value(result.to_string().into_bytes());
            }

            None => {
                self.storage
                    .insert(key, StoredValue::new(result.to_string().into_bytes()));
            }
        }

        Ok(result)
    }

    pub(crate) fn set_if_absent(&mut self, key: Vec<u8>, value: Vec<u8>) -> bool {
        self.remove_if_expired(&key);

        match self.storage.entry(key) {
            HashMapEntry::Vacant(entry) => {
                entry.insert(StoredValue::new(value));
                true
            }

            HashMapEntry::Occupied(_) => false,
        }
    }

    pub(crate) fn get_and_set(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.remove_if_expired(&key);

        if let Some(entry) = self.storage.get(&key) {
            entry.value()?;
        }

        self.storage
            .insert(key, StoredValue::new(value))
            .map(StoredValue::into_value)
            .transpose()
    }

    pub(crate) fn get_and_delete(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError> {
        self.remove_if_expired(&key);

        if let Some(entry) = self.storage.get(&key) {
            entry.value()?;
        }

        self.storage
            .remove(&key)
            .map(StoredValue::into_value)
            .transpose()
    }

    pub(crate) fn expire_at(&mut self, key: impl AsRef<[u8]>, expires_at: Instant) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        match self.storage.get_mut(key) {
            Some(entry) => {
                entry.set_expires_at(expires_at);
                true
            }

            None => false,
        }
    }

    fn remove_if_expired(&mut self, key: &[u8]) -> bool {
        let expired = self
            .storage
            .get(key)
            .is_some_and(|entry| entry.is_expired(self.clock.now()));

        if expired {
            self.storage.remove(key);
        }

        expired
    }

    fn remove_expired(&mut self) {
        let now = self.clock.now();
        self.storage.retain(|_, entry| !entry.is_expired(now));
    }

    pub(crate) fn expire(&mut self, key: impl AsRef<[u8]>, seconds: u64) -> bool {
        let Some(expires_at) = self.clock.now().checked_add(Duration::from_secs(seconds)) else {
            return false;
        };

        self.expire_at(key, expires_at)
    }

    pub(crate) fn ttl(&mut self, key: impl AsRef<[u8]>) -> i64 {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get(key) else {
            return -2;
        };

        let Some(expires_at) = entry.expires_at() else {
            return -1;
        };

        expires_at
            .saturating_duration_since(self.clock.now())
            .as_secs() as i64
    }

    pub(crate) fn persist(&mut self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get_mut(key) else {
            return false;
        };

        if !entry.has_expiration() {
            return false;
        };

        entry.clear_expiration();
        true
    }

    pub(crate) fn pexpire(&mut self, key: impl AsRef<[u8]>, milliseconds: u64) -> bool {
        let Some(expires_at) = self
            .clock
            .now()
            .checked_add(Duration::from_millis(milliseconds))
        else {
            return false;
        };

        self.expire_at(key, expires_at)
    }

    pub(crate) fn pttl(&mut self, key: impl AsRef<[u8]>) -> i64 {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get(key) else {
            return -2;
        };

        let Some(expires_at) = entry.expires_at() else {
            return -1;
        };

        let milliseconds = expires_at
            .saturating_duration_since(self.clock.now())
            .as_millis();

        i64::try_from(milliseconds).unwrap_or(i64::MAX)
    }

    pub(crate) fn string_length(&mut self, key: impl AsRef<[u8]>) -> Result<usize, StoreError> {
        Ok(self.get(key)?.map_or(0, <[u8]>::len))
    }

    pub(crate) fn get_range(
        &mut self,
        key: impl AsRef<[u8]>,
        start: i64,
        end: i64,
    ) -> Result<Vec<u8>, StoreError> {
        let Some(value) = self.get(key)? else {
            return Ok(Vec::new());
        };
        let length = value.len() as i64;

        if length == 0 {
            return Ok(Vec::new());
        }

        let mut start = normalize_index(start, length);
        let mut end = normalize_index(end, length);

        start = start.max(0);
        end = end.min(length - 1);

        if start >= length || end < 0 || start > end {
            return Ok(Vec::new());
        }

        Ok(value[start as usize..=end as usize].to_vec())
    }

    pub(crate) fn set_range(
        &mut self,
        key: Vec<u8>,
        offset: usize,
        value: Vec<u8>,
    ) -> Result<usize, StoreError> {
        self.remove_if_expired(&key);

        let entry = self
            .storage
            .entry(key)
            .or_insert_with(|| StoredValue::new(Vec::new()));

        let bytes = entry.value_mut()?;

        if bytes.len() < offset {
            bytes.resize(offset, 0);
        }

        for (index, byte) in value.into_iter().enumerate() {
            let position = offset + index;

            if position < bytes.len() {
                bytes[position] = byte;
            } else {
                bytes.push(byte);
            }
        }

        Ok(bytes.len())
    }

    pub(crate) fn delete_many(&mut self, keys: &[Vec<u8>]) -> usize {
        keys.iter().filter(|key| self.delete(key)).count()
    }

    pub(crate) fn exists_many(&mut self, keys: &[Vec<u8>]) -> usize {
        keys.iter().filter(|key| self.exists(key)).count()
    }

    pub(crate) fn push_left(
        &mut self,
        key: impl AsRef<[u8]>,
        value: Vec<u8>,
    ) -> Result<usize, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let list = self
            .storage
            .entry(key.to_vec())
            .or_insert_with(StoredValue::new_list)
            .list_mut()?;

        list.push_front(value);
        Ok(list.len())
    }

    pub(crate) fn push_right(
        &mut self,
        key: impl AsRef<[u8]>,
        value: Vec<u8>,
    ) -> Result<usize, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let list = self
            .storage
            .entry(key.to_vec())
            .or_insert_with(StoredValue::new_list)
            .list_mut()?;

        list.push_back(value);
        Ok(list.len())
    }

    pub(crate) fn list_length(&mut self, key: impl AsRef<[u8]>) -> Result<usize, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        match self.storage.get(key) {
            Some(entry) => Ok(entry.list()?.len()),
            None => Ok(0),
        }
    }

    pub(crate) fn pop_left(
        &mut self,
        key: impl AsRef<[u8]>,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let (value, became_empty) = {
            let Some(entry) = self.storage.get_mut(key) else {
                return Ok(None);
            };

            let list = entry.list_mut()?;
            let value = list.pop_front();
            (value, list.is_empty())
        };

        if became_empty {
            self.storage.remove(key);
        }

        Ok(value)
    }

    pub(crate) fn pop_right(
        &mut self,
        key: impl AsRef<[u8]>,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let (value, became_empty) = {
            let Some(entry) = self.storage.get_mut(key) else {
                return Ok(None);
            };

            let list = entry.list_mut()?;
            let value = list.pop_back();
            (value, list.is_empty())
        };

        if became_empty {
            self.storage.remove(key);
        }

        Ok(value)
    }

    pub(crate) fn list_range(
        &mut self,
        key: impl AsRef<[u8]>,
        start: i64,
        end: i64,
    ) -> Result<Vec<Vec<u8>>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get(key) else {
            return Ok(Vec::new());
        };

        let list = entry.list()?;
        let length = i64::try_from(list.len()).unwrap_or(i64::MAX);

        if length == 0 {
            return Ok(Vec::new());
        }

        let start = normalize_index(start, length).max(0);
        let end = normalize_index(end, length).min(length - 1);

        if start >= length || end < 0 || start > end {
            return Ok(Vec::new());
        }

        let count = (end - start + 1) as usize;

        Ok(list
            .iter()
            .skip(start as usize)
            .take(count)
            .cloned()
            .collect())
    }

    pub(crate) fn set_add(
        &mut self,
        key: impl AsRef<[u8]>,
        member: Vec<u8>,
    ) -> Result<bool, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        self.storage
            .entry(key.to_vec())
            .or_insert_with(StoredValue::new_set)
            .set_mut()
            .map(|set| set.insert(member))
    }

    pub(crate) fn set_remove(
        &mut self,
        key: impl AsRef<[u8]>,
        member: impl AsRef<[u8]>,
    ) -> Result<bool, StoreError> {
        let key = key.as_ref();
        let member = member.as_ref();
        self.remove_if_expired(key);

        let (removed, became_empty) = {
            let Some(entry) = self.storage.get_mut(key) else {
                return Ok(false);
            };

            let set = entry.set_mut()?;
            let removed = set.remove(member);
            (removed, set.is_empty())
        };

        if became_empty {
            self.storage.remove(key);
        }

        Ok(removed)
    }

    pub(crate) fn set_contains(
        &mut self,
        key: impl AsRef<[u8]>,
        member: impl AsRef<[u8]>,
    ) -> Result<bool, StoreError> {
        let key = key.as_ref();
        let member = member.as_ref();
        self.remove_if_expired(key);

        match self.storage.get(key) {
            Some(entry) => Ok(entry.set()?.contains(member)),
            None => Ok(false),
        }
    }

    pub(crate) fn set_members(
        &mut self,
        key: impl AsRef<[u8]>,
    ) -> Result<Vec<Vec<u8>>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get(key) else {
            return Ok(Vec::new());
        };

        let mut members: Vec<_> = entry.set()?.iter().cloned().collect();
        members.sort();
        Ok(members)
    }

    pub(crate) fn set_cardinality(&mut self, key: impl AsRef<[u8]>) -> Result<usize, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        match self.storage.get(key) {
            Some(entry) => Ok(entry.set()?.len()),
            None => Ok(0),
        }
    }

    #[cfg(test)]
    pub(crate) fn expiration(&self, key: impl AsRef<[u8]>) -> Option<Instant> {
        self.storage
            .get(key.as_ref())
            .and_then(StoredValue::expires_at)
    }

    #[cfg(test)]
    pub(crate) fn set_list(&mut self, key: Vec<u8>, values: Vec<Vec<u8>>) {
        let mut entry = StoredValue::new_list();
        entry
            .list_mut()
            .expect("a new list entry should expose its list")
            .extend(values);
        self.storage.insert(key, entry);
    }

    #[cfg(test)]
    pub(crate) fn list_values(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Result<Option<Vec<Vec<u8>>>, StoreError> {
        self.storage
            .get(key.as_ref())
            .map(|entry| entry.list().map(|values| values.iter().cloned().collect()))
            .transpose()
    }

    #[cfg(test)]
    pub(crate) fn set_set(&mut self, key: Vec<u8>, members: Vec<Vec<u8>>) {
        let mut entry = StoredValue::new_set();
        entry
            .set_mut()
            .expect("a new set entry should expose its set")
            .extend(members);
        self.storage.insert(key, entry);
    }
}

fn parse_integer(value: &[u8]) -> Result<i64, StoreError> {
    str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or(StoreError::ValueIsNotInteger)
}

fn parse_float(value: &[u8]) -> Result<f64, StoreError> {
    str::from_utf8(value)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or(StoreError::ValueIsNotFloat)
}
