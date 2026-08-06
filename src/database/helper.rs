pub(super) fn normalize_index(index: i64, len: i64) -> i64 {
    if index < 0 { len + index } else { index }
}
