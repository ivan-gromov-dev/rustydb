use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    String(String),
    List(VecDeque<String>),
    Set(HashSet<String>),
}
