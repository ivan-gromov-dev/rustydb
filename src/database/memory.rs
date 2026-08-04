use std::collections::HashMap;

pub(crate) struct Database {
    storage: HashMap<String, String>,
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
}
