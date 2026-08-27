use super::indexing::normalize_index;
use super::stored_value::StoredValue;
use crate::storage::clock::{Clock, SystemClock};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, hash_map::Entry as HashMapEntry};
use std::fmt;
use std::str;
use std::time::{Duration, Instant};

pub(crate) struct InMemoryStore {
    pub(super) storage: HashMap<Vec<u8>, StoredValue>,
    pub(super) expirations: BinaryHeap<Reverse<(Instant, Vec<u8>)>>,
    max_keys: Option<usize>,
    reclamation_metrics: ReclamationMetrics,
    pending_evictions: Vec<Vec<u8>>,
    pub(super) clock: Box<dyn Clock>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReclamationMetrics {
    pub(crate) deletions: u64,
    pub(crate) expirations: u64,
    pub(crate) evictions: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StoreError {
    ValueIsNotInteger,
    IntegerOverflow,
    ValueIsNotFloat,
    FloatIsNotFinite,
    WrongType,
    ExpirationOutOfRange,
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
            Self::ExpirationOutOfRange => write!(formatter, "expiration is out of range"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetCondition {
    IfAbsent,
    IfPresent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetExpiration {
    Duration(Duration),
    KeepTtl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExpirationUpdate {
    Set(Duration),
    Persist,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SetResult {
    pub(crate) applied: bool,
    pub(crate) old_value: Option<Vec<u8>>,
}

impl InMemoryStore {
    pub(crate) fn new() -> Self {
        Self::with_max_keys(None)
    }

    pub(crate) fn with_max_keys(max_keys: Option<usize>) -> Self {
        Self::with_clock_and_max_keys(Box::new(SystemClock), max_keys)
    }

    #[cfg(test)]
    pub(crate) fn with_clock(clock: Box<dyn Clock>) -> Self {
        Self::with_clock_and_max_keys(clock, None)
    }

    pub(crate) fn with_clock_and_max_keys(clock: Box<dyn Clock>, max_keys: Option<usize>) -> Self {
        Self {
            storage: HashMap::new(),
            expirations: BinaryHeap::new(),
            max_keys,
            reclamation_metrics: ReclamationMetrics::default(),
            pending_evictions: Vec::new(),
            clock,
        }
    }

    pub(crate) fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.remove_if_expired(&key);
        self.ensure_capacity_for(&key);
        self.storage.insert(key, StoredValue::new(value));
    }

    pub(crate) fn set_advanced(
        &mut self,
        key: Vec<u8>,
        value: Vec<u8>,
        condition: Option<SetCondition>,
        return_old: bool,
        expiration: Option<SetExpiration>,
    ) -> Result<SetResult, StoreError> {
        self.remove_if_expired(&key);

        let old_value = if return_old {
            self.storage
                .get(&key)
                .map(|entry| entry.value().map(<[u8]>::to_vec))
                .transpose()?
        } else {
            None
        };
        let exists = self.storage.contains_key(&key);
        let applies = match condition {
            Some(SetCondition::IfAbsent) => !exists,
            Some(SetCondition::IfPresent) => exists,
            None => true,
        };
        if !applies {
            return Ok(SetResult {
                applied: false,
                old_value,
            });
        }

        let expires_at = match expiration {
            Some(SetExpiration::Duration(duration)) => Some(
                self.clock
                    .now()
                    .checked_add(duration)
                    .ok_or(StoreError::ExpirationOutOfRange)?,
            ),
            Some(SetExpiration::KeepTtl) => {
                self.storage.get(&key).and_then(StoredValue::expires_at)
            }
            None => None,
        };

        self.ensure_capacity_for(&key);
        let mut entry = StoredValue::new(value);
        if let Some(expires_at) = expires_at {
            entry.set_expires_at(expires_at);
            self.expirations.push(Reverse((expires_at, key.clone())));
        }
        self.storage.insert(key, entry);
        Ok(SetResult {
            applied: true,
            old_value,
        })
    }

    pub(crate) fn get(&mut self, key: impl AsRef<[u8]>) -> Result<Option<&[u8]>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);

        self.storage.get(key).map(StoredValue::value).transpose()
    }

    pub(crate) fn get_with_expiration(
        &mut self,
        key: impl AsRef<[u8]>,
        update: Option<ExpirationUpdate>,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let key = key.as_ref();
        self.remove_if_expired(key);
        let Some(entry) = self.storage.get(key) else {
            return Ok(None);
        };
        let value = entry.value()?.to_vec();
        let expires_at = match update {
            Some(ExpirationUpdate::Set(duration)) => Some(
                self.clock
                    .now()
                    .checked_add(duration)
                    .ok_or(StoreError::ExpirationOutOfRange)?,
            ),
            _ => None,
        };
        if let Some(update) = update {
            let Some(entry) = self.storage.get_mut(key) else {
                return Ok(None);
            };
            match update {
                ExpirationUpdate::Set(_) => {
                    let expires_at = expires_at.ok_or(StoreError::ExpirationOutOfRange)?;
                    entry.set_expires_at(expires_at);
                    self.expirations.push(Reverse((expires_at, key.to_vec())));
                }
                ExpirationUpdate::Persist => entry.clear_expiration(),
            }
        }
        Ok(Some(value))
    }

    pub(crate) fn mset_if_absent(&mut self, entries: Vec<(Vec<u8>, Vec<u8>)>) -> bool {
        for (key, _) in &entries {
            self.remove_if_expired(key);
        }
        if entries
            .iter()
            .any(|(key, _)| self.storage.contains_key(key))
        {
            return false;
        }
        for (key, value) in entries {
            self.set(key, value);
        }
        true
    }

    pub(crate) fn exists(&mut self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        self.storage.contains_key(key)
    }

    pub(crate) fn delete(&mut self, key: impl AsRef<[u8]>) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        let removed = self.storage.remove(key).is_some();
        if removed {
            self.reclamation_metrics.deletions =
                self.reclamation_metrics.deletions.saturating_add(1);
        }
        removed
    }

    pub(crate) fn len(&mut self) -> usize {
        self.remove_expired();

        self.storage.len()
    }

    pub(crate) fn clear(&mut self) {
        self.remove_expired();
        let removed = u64::try_from(self.storage.len()).unwrap_or(u64::MAX);
        self.storage.clear();
        self.reclamation_metrics.deletions =
            self.reclamation_metrics.deletions.saturating_add(removed);
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
                let expires_at = entry.expires_at();
                if let Some(expires_at) = expires_at {
                    self.expirations
                        .push(Reverse((expires_at, new_key.to_vec())));
                }
                if self.storage.insert(new_key, entry).is_some() {
                    self.reclamation_metrics.deletions =
                        self.reclamation_metrics.deletions.saturating_add(1);
                }
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
        self.ensure_capacity_for(key);

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
                self.ensure_capacity_for(&key);
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
                self.ensure_capacity_for(&key);
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
                self.ensure_capacity_for(&key);
                self.storage
                    .insert(key, StoredValue::new(result.to_string().into_bytes()));
            }
        }

        Ok(result)
    }

    pub(crate) fn set_if_absent(&mut self, key: Vec<u8>, value: Vec<u8>) -> bool {
        self.remove_if_expired(&key);

        if self.storage.contains_key(&key) {
            return false;
        }
        self.ensure_capacity_for(&key);

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

        self.ensure_capacity_for(&key);

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

        let removed = self
            .storage
            .remove(&key)
            .map(StoredValue::into_value)
            .transpose()?;
        if removed.is_some() {
            self.reclamation_metrics.deletions =
                self.reclamation_metrics.deletions.saturating_add(1);
        }
        Ok(removed)
    }

    pub(crate) fn expire_at(&mut self, key: impl AsRef<[u8]>, expires_at: Instant) -> bool {
        let key = key.as_ref();
        self.remove_if_expired(key);

        match self.storage.get_mut(key) {
            Some(entry) => {
                entry.set_expires_at(expires_at);
                self.expirations.push(Reverse((expires_at, key.to_vec())));
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
            self.reclamation_metrics.expirations =
                self.reclamation_metrics.expirations.saturating_add(1);
        }

        expired
    }

    pub(super) fn remove_expired(&mut self) {
        let now = self.clock.now();
        let before = self.storage.len();
        self.storage.retain(|_, entry| !entry.is_expired(now));
        let removed = u64::try_from(before - self.storage.len()).unwrap_or(u64::MAX);
        self.reclamation_metrics.expirations =
            self.reclamation_metrics.expirations.saturating_add(removed);
    }

    pub(crate) fn active_expire(&mut self, limit: usize) -> usize {
        let now = self.clock.now();
        let mut inspected = 0;
        let mut removed = 0;

        while inspected < limit {
            let Some(Reverse((expires_at, _))) = self.expirations.peek() else {
                break;
            };
            if *expires_at > now {
                break;
            }

            let Some(Reverse((expires_at, key))) = self.expirations.pop() else {
                break;
            };
            inspected += 1;

            let is_current = self
                .storage
                .get(&key)
                .is_some_and(|entry| entry.expires_at() == Some(expires_at));
            if is_current {
                self.storage.remove(&key);
                removed += 1;
                self.reclamation_metrics.expirations =
                    self.reclamation_metrics.expirations.saturating_add(1);
            }
        }

        removed
    }

    pub(crate) fn enforce_key_limit(&mut self) {
        let Some(max_keys) = self.max_keys else {
            return;
        };

        while self.storage.len() > max_keys {
            self.evict_one();
        }
    }

    fn ensure_capacity_for(&mut self, key: &[u8]) {
        let Some(max_keys) = self.max_keys else {
            return;
        };
        if !self.storage.contains_key(key) && self.storage.len() >= max_keys {
            self.evict_one();
        }
    }

    fn evict_one(&mut self) {
        let now = self.clock.now();
        let expired = self
            .storage
            .iter()
            .filter(|(_, entry)| entry.is_expired(now))
            .map(|(key, _)| key)
            .min()
            .cloned();
        if let Some(key) = expired {
            self.storage.remove(&key);
            self.reclamation_metrics.expirations =
                self.reclamation_metrics.expirations.saturating_add(1);
            return;
        }

        if let Some(key) = self.storage.keys().min().cloned() {
            self.storage.remove(&key);
            self.reclamation_metrics.evictions =
                self.reclamation_metrics.evictions.saturating_add(1);
            self.pending_evictions.push(key);
        }
    }

    pub(crate) fn take_evicted_keys(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.pending_evictions)
    }

    pub(crate) fn contains_stored_key(&self, key: &[u8]) -> bool {
        self.storage.contains_key(key)
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
        self.ensure_capacity_for(&key);

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
        self.ensure_capacity_for(key);

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
        self.ensure_capacity_for(key);

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
            self.reclamation_metrics.deletions =
                self.reclamation_metrics.deletions.saturating_add(1);
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
            self.reclamation_metrics.deletions =
                self.reclamation_metrics.deletions.saturating_add(1);
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
        self.ensure_capacity_for(key);

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
            self.reclamation_metrics.deletions =
                self.reclamation_metrics.deletions.saturating_add(1);
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

    #[cfg(test)]
    pub(crate) fn evicted_keys(&self) -> u64 {
        self.reclamation_metrics.evictions
    }

    pub(crate) fn reclamation_metrics(&self) -> ReclamationMetrics {
        self.reclamation_metrics
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
