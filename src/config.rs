use std::num::NonZeroUsize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoryConfig {
    max_keys: Option<NonZeroUsize>,
}

impl MemoryConfig {
    pub fn with_max_keys(max_keys: NonZeroUsize) -> Self {
        Self {
            max_keys: Some(max_keys),
        }
    }

    pub(crate) fn max_keys(self) -> Option<usize> {
        self.max_keys.map(NonZeroUsize::get)
    }
}
