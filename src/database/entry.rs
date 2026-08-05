use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    value: String,
    expires_at: Option<Instant>,
}

impl Entry {
    pub(crate) fn new(value: String) -> Self {
        Self {
            value,
            expires_at: None,
        }
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut String {
        &mut self.value
    }

    pub(crate) fn into_value(self) -> String {
        self.value
    }
    pub(crate) fn expires_at(&self) -> Option<Instant> {
        self.expires_at
    }

    pub(crate) fn set_expires_at(&mut self, expires_at: Instant) {
        self.expires_at = Some(expires_at);
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
