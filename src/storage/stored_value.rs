use crate::storage::value::Value;
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

    pub(crate) fn value(&self) -> &str {
        match &self.value {
            Value::String(value) => value,
        }
    }

    pub(crate) fn value_mut(&mut self) -> &mut String {
        match &mut self.value {
            Value::String(value) => value,
        }
    }

    pub(crate) fn set_value(&mut self, value: String) {
        self.value = Value::String(value);
    }

    pub(crate) fn into_value(self) -> String {
        match self.value {
            Value::String(value) => value,
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
}
