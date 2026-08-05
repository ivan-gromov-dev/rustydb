use super::entry::Entry;
use std::collections::{HashMap, hash_map::Entry as HashMapEntry};
use std::fmt;
use std::time::{Duration, Instant};

pub(crate) struct Database {
    storage: HashMap<String, Entry>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DatabaseError {
    ValueIsNotInteger,
    IntegerOverflow,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueIsNotInteger => {
                write!(formatter, "value is not integer")
            }

            Self::IntegerOverflow => {
                write!(formatter, "integer overflow")
            }
        }
    }
}

impl Database {
    pub(crate) fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    pub(crate) fn set(&mut self, key: String, value: String) {
        self.storage.insert(key, Entry::new(value));
    }

    pub(crate) fn get(&mut self, key: &str) -> Option<&str> {
        self.remove_if_expired(key);

        self.storage.get(key).map(Entry::value)
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

    pub(crate) fn rename_key(&mut self, old_key: &str, new_key: String) -> bool {
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
            .or_insert_with(|| Entry::new(String::new()))
            .value_mut();

        stored_value.push_str(&append_value);
        stored_value.len()
    }

    pub(crate) fn increment(&mut self, key: String) -> Result<i64, DatabaseError> {
        self.increment_by(key, 1)
    }

    pub(crate) fn increment_by(&mut self, key: String, amount: i64) -> Result<i64, DatabaseError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => entry
                .value()
                .parse::<i64>()
                .map_err(|_| DatabaseError::ValueIsNotInteger)?,

            None => 0,
        };

        let incremented = number
            .checked_add(amount)
            .ok_or(DatabaseError::IntegerOverflow)?;

        self.storage
            .insert(key, Entry::new(incremented.to_string()));

        Ok(incremented)
    }

    pub(crate) fn decrement(&mut self, key: String) -> Result<i64, DatabaseError> {
        self.decrement_by(key, 1)
    }

    pub(crate) fn decrement_by(&mut self, key: String, amount: i64) -> Result<i64, DatabaseError> {
        self.remove_if_expired(&key);

        let number = match self.storage.get(&key) {
            Some(entry) => entry
                .value()
                .parse::<i64>()
                .map_err(|_| DatabaseError::ValueIsNotInteger)?,

            None => 0,
        };

        let decremented = number
            .checked_sub(amount)
            .ok_or(DatabaseError::IntegerOverflow)?;

        self.storage
            .insert(key, Entry::new(decremented.to_string()));

        Ok(decremented)
    }

    pub(crate) fn set_if_absent(&mut self, key: String, value: String) -> bool {
        self.remove_if_expired(&key);

        match self.storage.entry(key) {
            HashMapEntry::Vacant(entry) => {
                entry.insert(Entry::new(value));
                true
            }

            HashMapEntry::Occupied(_) => false,
        }
    }

    pub(crate) fn get_and_set(&mut self, key: String, value: String) -> Option<String> {
        self.remove_if_expired(&key);

        self.storage
            .insert(key, Entry::new(value))
            .map(Entry::into_value)
    }

    pub(crate) fn get_and_delete(&mut self, key: String) -> Option<String> {
        self.remove_if_expired(&key);

        self.storage.remove(&key).map(Entry::into_value)
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
        let expires_at = Instant::now() + Duration::from_secs(seconds);

        self.expire_at(key, expires_at)
    }

    #[cfg(test)]
    pub(crate) fn expiration(&self, key: &str) -> Option<Instant> {
        self.storage.get(key).and_then(Entry::expires_at)
    }
}
