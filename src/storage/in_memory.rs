use super::indexing::normalize_index;
use super::stored_value::StoredValue;
use std::collections::{HashMap, hash_map::Entry as HashMapEntry};
use std::fmt;
use std::time::{Duration, Instant};

pub(crate) struct InMemoryStore {
    storage: HashMap<String, StoredValue>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StoreError {
    ValueIsNotInteger,
    IntegerOverflow,
    ValueIsNotFloat,
    FloatIsNotFinite,
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
        }
    }
}

impl InMemoryStore {
    pub(crate) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(crate) fn set(&mut self, key: String, value: String) {
        self.storage.insert(key, StoredValue::new(value));
    }

    pub(crate) fn get(&mut self, key: &str) -> Option<&str> {
        self.remove_if_expired(key);

        self.storage.get(key).map(StoredValue::value)
    }

    pub(crate) fn exists(&mut self, key: &str) -> bool {
        self.remove_if_expired(key);

        self.storage.contains_key(key)
    }

    pub(crate) fn delete(&mut self, key: &str) -> bool {
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

    pub(crate) fn keys(&mut self) -> Vec<String> {
        self.remove_expired();

        let mut keys: Vec<_> = self.storage.keys().cloned().collect();

        keys.sort();
        keys
    }

    pub(crate) fn rename(&mut self, old_key: &str, new_key: String) -> bool {
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

    pub(crate) fn append(&mut self, key: &str, append_value: String) -> usize {
        self.remove_if_expired(key);

        let stored_value = self
            .storage
            .entry(key.to_owned())
            .or_insert_with(|| StoredValue::new(String::new()))
            .value_mut();

        stored_value.push_str(&append_value);
        stored_value.len()
    }

    pub(crate) fn increment(&mut self, key: String) -> Result<i64, StoreError> {
        self.increment_by(key, 1)
    }

    pub(crate) fn increment_by(&mut self, key: String, amount: i64) -> Result<i64, StoreError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => entry
                .value()
                .parse::<i64>()
                .map_err(|_| StoreError::ValueIsNotInteger)?,

            None => 0,
        };

        let incremented = number
            .checked_add(amount)
            .ok_or(StoreError::IntegerOverflow)?;

        match self.storage.get_mut(&key) {
            Some(entry) => {
                entry.set_value(incremented.to_string());
            }

            None => {
                self.storage
                    .insert(key, StoredValue::new(incremented.to_string()));
            }
        }

        Ok(incremented)
    }

    pub(crate) fn decrement(&mut self, key: String) -> Result<i64, StoreError> {
        self.decrement_by(key, 1)
    }

    pub(crate) fn decrement_by(&mut self, key: String, amount: i64) -> Result<i64, StoreError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => entry
                .value()
                .parse::<i64>()
                .map_err(|_| StoreError::ValueIsNotInteger)?,

            None => 0,
        };

        let decremented = number
            .checked_sub(amount)
            .ok_or(StoreError::IntegerOverflow)?;

        match self.storage.get_mut(&key) {
            Some(entry) => {
                entry.set_value(decremented.to_string());
            }

            None => {
                self.storage
                    .insert(key, StoredValue::new(decremented.to_string()));
            }
        }

        Ok(decremented)
    }

    pub(crate) fn increment_by_float(
        &mut self,
        key: String,
        amount: f64,
    ) -> Result<f64, StoreError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => entry
                .value()
                .parse::<f64>()
                .map_err(|_| StoreError::ValueIsNotFloat)?,

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
                entry.set_value(result.to_string());
            }

            None => {
                self.storage
                    .insert(key, StoredValue::new(result.to_string()));
            }
        }

        Ok(result)
    }

    pub(crate) fn set_if_absent(&mut self, key: String, value: String) -> bool {
        self.remove_if_expired(&key);

        match self.storage.entry(key) {
            HashMapEntry::Vacant(entry) => {
                entry.insert(StoredValue::new(value));
                true
            }

            HashMapEntry::Occupied(_) => false,
        }
    }

    pub(crate) fn get_and_set(&mut self, key: String, value: String) -> Option<String> {
        self.remove_if_expired(&key);

        self.storage
            .insert(key, StoredValue::new(value))
            .map(StoredValue::into_value)
    }

    pub(crate) fn get_and_delete(&mut self, key: String) -> Option<String> {
        self.remove_if_expired(&key);

        self.storage.remove(&key).map(StoredValue::into_value)
    }

    pub(crate) fn expire_at(&mut self, key: &str, expires_at: Instant) -> bool {
        self.remove_if_expired(key);

        match self.storage.get_mut(key) {
            Some(entry) => {
                entry.set_expires_at(expires_at);
                true
            }

            None => false,
        }
    }

    fn remove_if_expired(&mut self, key: &str) -> bool {
        let now = Instant::now();

        let expired = self
            .storage
            .get(key)
            .is_some_and(|entry| entry.is_expired(now));

        if expired {
            self.storage.remove(key);
        }

        expired
    }

    fn remove_expired(&mut self) {
        let now = Instant::now();

        self.storage.retain(|_, entry| !entry.is_expired(now));
    }

    pub(crate) fn expire(&mut self, key: &str, seconds: u64) -> bool {
        let Some(expires_at) = Instant::now().checked_add(Duration::from_secs(seconds)) else {
            return false;
        };

        self.expire_at(key, expires_at)
    }

    pub(crate) fn ttl(&mut self, key: &str) -> i64 {
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get(key) else {
            return -2;
        };

        let Some(expires_at) = entry.expires_at() else {
            return -1;
        };

        let now = Instant::now();

        expires_at.saturating_duration_since(now).as_secs() as i64
    }

    pub(crate) fn persist(&mut self, key: &str) -> bool {
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

    pub(crate) fn pexpire(&mut self, key: &str, milliseconds: u64) -> bool {
        let Some(expires_at) = Instant::now().checked_add(Duration::from_millis(milliseconds))
        else {
            return false;
        };

        self.expire_at(key, expires_at)
    }

    pub(crate) fn pttl(&mut self, key: &str) -> i64 {
        self.remove_if_expired(key);

        let Some(entry) = self.storage.get(key) else {
            return -2;
        };

        let Some(expires_at) = entry.expires_at() else {
            return -1;
        };

        let milliseconds = expires_at
            .saturating_duration_since(Instant::now())
            .as_millis();

        i64::try_from(milliseconds).unwrap_or(i64::MAX)
    }

    pub(crate) fn string_length(&mut self, key: &str) -> usize {
        self.get(key).map_or(0, |value| value.chars().count())
    }

    pub(crate) fn get_range(&mut self, key: &str, start: i64, end: i64) -> String {
        let Some(value) = self.get(key) else {
            return String::new();
        };

        let characters: Vec<char> = value.chars().collect();
        let length = characters.len() as i64;

        if length == 0 {
            return String::new();
        }

        let mut start = normalize_index(start, length);
        let mut end = normalize_index(end, length);

        start = start.max(0);
        end = end.min(length - 1);

        if start >= length || end < 0 || start > end {
            return String::new();
        }

        characters[start as usize..=end as usize].iter().collect()
    }

    pub(crate) fn set_range(&mut self, key: String, offset: usize, value: String) -> usize {
        self.remove_if_expired(&key);

        let entry = self
            .storage
            .entry(key)
            .or_insert_with(|| StoredValue::new(String::new()));

        let mut characters: Vec<char> = entry.value().chars().collect();

        if characters.len() < offset {
            characters.resize(offset, '\0');
        }

        for (index, character) in value.chars().enumerate() {
            let position = offset + index;

            if position < characters.len() {
                characters[position] = character;
            } else {
                characters.push(character);
            }
        }

        let length = characters.len();

        entry.set_value(characters.into_iter().collect());

        length
    }

    #[cfg(test)]
    pub(crate) fn expiration(&self, key: &str) -> Option<Instant> {
        self.storage.get(key).and_then(StoredValue::expires_at)
    }
}
