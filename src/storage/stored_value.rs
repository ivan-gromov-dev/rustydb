use crate::storage::{in_memory::StoreError, value::Value};
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredValue {
    value: Value,
    expires_at: Option<Instant>,
}

impl StoredValue {
    pub(crate) fn new(value: String) -> Self {
        Self {
            value: Value::String(value),
            expires_at: None,
        }
    }

    pub(crate) fn value(&self) -> Result<&str, StoreError> {
        match &self.value {
            Value::String(value) => Ok(value),
            Value::List(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn value_mut(&mut self) -> Result<&mut String, StoreError> {
        match &mut self.value {
            Value::String(value) => Ok(value),
            Value::List(_) => Err(StoreError::WrongType),
        }
    }

    pub(crate) fn set_value(&mut self, value: String) {
        self.value = Value::String(value);
    }

    pub(crate) fn into_value(self) -> Result<String, StoreError> {
        match self.value {
            Value::String(value) => Ok(value),
            Value::List(_) => Err(StoreError::WrongType),
        }
    }
    pub(crate) fn expires_at(&self) -> Option<Instant> {
        self.expires_at
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

    #[cfg(test)]
    pub(crate) fn new_list(values: Vec<String>) -> Self {
        Self {
            value: Value::List(values),
            expires_at: None,
        }
    }
}
