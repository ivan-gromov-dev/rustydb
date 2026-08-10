use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    String(String),
    List(VecDeque<String>),
}
