use crate::storage::{in_memory::StoreError, value::Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredValue {
    value: Value,
    expires_at: Option<Instant>,
}

impl StoredValue {
    pub(crate) fn new(value: Vec<u8>) -> Self {
        Self {
            value: Value::String(value),
            expires_at: None,
        }
    }

    pub(crate) fn value(&self) -> Result<&[u8], StoreError> {
        match &self.value {
            Value::String(value) => Ok(value),
            Value::List(_) | Value::Set(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn value_mut(&mut self) -> Result<&mut Vec<u8>, StoreError> {
        match &mut self.value {
            Value::String(value) => Ok(value),
            Value::List(_) | Value::Set(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn set_value(&mut self, value: Vec<u8>) {
        self.value = Value::String(value);
    }

    pub(crate) fn into_value(self) -> Result<Vec<u8>, StoreError> {
        match self.value {
            Value::String(value) => Ok(value),
            Value::List(_) | Value::Set(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }
    pub(crate) fn expires_at(&self) -> Option<Instant> {
        self.expires_at
    }

    pub(crate) fn typed_value(&self) -> &Value {
        &self.value
    }

    pub(crate) fn type_name(&self) -> &'static str {
        match &self.value {
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Set(_) => "set",
            Value::Hash(_) => "hash",
        }
    }

    pub(crate) fn from_parts(value: Value, expires_at: Option<Instant>) -> Self {
        Self { value, expires_at }
    }

    pub(crate) fn set_expires_at(&mut self, expires_at: Instant) {
        self.expires_at = Some(expires_at);
    }

    pub(crate) fn has_expiration(&self) -> bool {
        self.expires_at.is_some()
    }

    pub(crate) fn clear_expiration(&mut self) {
        self.expires_at = None;
    }

    pub(crate) fn is_expired(&self, now: Instant) -> bool {
        match self.expires_at {
            Some(expires_at) => now >= expires_at,
            None => false,
        }
    }

    pub(crate) fn new_list() -> Self {
        Self {
            value: Value::List(VecDeque::new()),
            expires_at: None,
        }
    }

    pub(crate) fn list(&self) -> Result<&VecDeque<Vec<u8>>, StoreError> {
        match &self.value {
            Value::List(values) => Ok(values),
            Value::String(_) | Value::Set(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn list_mut(&mut self) -> Result<&mut VecDeque<Vec<u8>>, StoreError> {
        match &mut self.value {
            Value::List(values) => Ok(values),
            Value::String(_) | Value::Set(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn new_set() -> Self {
        Self {
            value: Value::Set(HashSet::new()),
            expires_at: None,
        }
    }

    pub(crate) fn set(&self) -> Result<&HashSet<Vec<u8>>, StoreError> {
        match &self.value {
            Value::Set(values) => Ok(values),
            Value::String(_) | Value::List(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn set_mut(&mut self) -> Result<&mut HashSet<Vec<u8>>, StoreError> {
        match &mut self.value {
            Value::Set(values) => Ok(values),
            Value::String(_) | Value::List(_) | Value::Hash(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn new_hash() -> Self {
        Self {
            value: Value::Hash(HashMap::new()),
            expires_at: None,
        }
    }

    pub(crate) fn hash(&self) -> Result<&HashMap<Vec<u8>, Vec<u8>>, StoreError> {
        match &self.value {
            Value::Hash(values) => Ok(values),
            Value::String(_) | Value::List(_) | Value::Set(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn hash_mut(&mut self) -> Result<&mut HashMap<Vec<u8>, Vec<u8>>, StoreError> {
        match &mut self.value {
            Value::Hash(values) => Ok(values),
            Value::String(_) | Value::List(_) | Value::Set(_) => Err(StoreError::WrongType),
        }
    }
}
