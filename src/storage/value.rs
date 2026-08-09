#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    String(String),
    // List commands arrive in 0.3; the variant exists in 0.2 to enforce typed access.
    #[allow(dead_code)]
    List(Vec<String>),
}
