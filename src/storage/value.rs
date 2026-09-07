use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    String(Vec<u8>),
    List(VecDeque<Vec<u8>>),
    Set(HashSet<Vec<u8>>),
    Hash(HashMap<Vec<u8>, Vec<u8>>),
    SortedSet(HashMap<Vec<u8>, Score>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ScoreBound {
    NegativeInfinity,
    PositiveInfinity,
    Inclusive(f64),
    Exclusive(f64),
}

impl ScoreBound {
    pub(crate) fn allows_lower(self, score: f64) -> bool {
        match self {
            Self::NegativeInfinity => true,
            Self::PositiveInfinity => false,
            Self::Inclusive(value) => score >= value,
            Self::Exclusive(value) => score > value,
        }
    }

    pub(crate) fn allows_upper(self, score: f64) -> bool {
        match self {
            Self::NegativeInfinity => false,
            Self::PositiveInfinity => true,
            Self::Inclusive(value) => score <= value,
            Self::Exclusive(value) => score < value,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Score(f64);

impl Score {
    pub(crate) fn new(value: f64) -> Option<Self> {
        value
            .is_finite()
            .then_some(Self(if value == 0.0 { 0.0 } else { value }))
    }

    pub(crate) fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for Score {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Score {}
