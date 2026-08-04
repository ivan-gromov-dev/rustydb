use std::collections::HashMap;
use std::fmt;

pub(crate) struct Database {
    storage: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DatabaseError {
    ValueIsNotInteger,
    IntegerOverflow,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValueIsNotInteger => write!(formatter, "value is not integer"),
            Self::IntegerOverflow => write!(formatter, "integer overflow"),
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
        self.storage.insert(key, value);
    }

    pub(crate) fn get(&self, key: &str) -> Option<&str> {
        self.storage.get(key).map(String::as_str)
    }

    pub(crate) fn exists(&self, key: &str) -> bool {
        self.storage.contains_key(key)
    }

    pub(crate) fn delete(&mut self, key: &str) -> bool {
        self.storage.remove(key).is_some()
    }

    pub(crate) fn len(&self) -> usize {
        self.storage.len()
    }

    pub(crate) fn clear(&mut self) {
        self.storage.clear();
    }

    pub(crate) fn keys(&self) -> Vec<&str> {
        let mut values: Vec<_> = self.storage.keys().map(String::as_str).collect();
        values.sort();
        values
    }

    pub(crate) fn rename_key(&mut self, old_key: &str, new_key: String) -> bool {
        match self.storage.remove(old_key) {
            Some(value) => {
                self.storage.insert(new_key, value);
                true
            }
            None => false,
        }
    }

    pub(crate) fn append(&mut self, key: &str, append_value: String) -> usize {
        let stored_value = self.storage.entry(key.to_owned()).or_default();
        stored_value.push_str(&append_value);
        stored_value.len()
    }

    pub(crate) fn increment(&mut self, key: String) -> Result<i64, DatabaseError> {
        self.increment_by(key, 1)
    }

    pub(crate) fn increment_by(
        &mut self,
        key: String,
        incr_amount: i64,
    ) -> Result<i64, DatabaseError> {
        let number = match self.storage.get(&key) {
            Some(value) => value
                .parse::<i64>()
                .map_err(|_| DatabaseError::ValueIsNotInteger)?,
            None => 0,
        };

        let incremented = number
            .checked_add(incr_amount)
            .ok_or(DatabaseError::IntegerOverflow)?;

        self.storage.insert(key, incremented.to_string());

        Ok(incremented)
    }
}
