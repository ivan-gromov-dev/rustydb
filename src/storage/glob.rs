use std::collections::HashMap;

pub(super) fn matches(pattern: &[u8], value: &[u8]) -> bool {
    matches_from(pattern, value, 0, 0, &mut HashMap::new())
}

fn matches_from(
    pattern: &[u8],
    value: &[u8],
    mut p: usize,
    mut v: usize,
    memo: &mut HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(result) = memo.get(&(p, v)) {
        return *result;
    }
    let start = (p, v);
    let result = matches_loop(pattern, value, &mut p, &mut v, memo);
    memo.insert(start, result);
    result
}

fn matches_loop(
    pattern: &[u8],
    value: &[u8],
    p: &mut usize,
    v: &mut usize,
    memo: &mut HashMap<(usize, usize), bool>,
) -> bool {
    while *p < pattern.len() {
        match pattern[*p] {
            b'*' => {
                while pattern.get(*p + 1) == Some(&b'*') {
                    *p += 1;
                }
                if *p + 1 == pattern.len() {
                    return true;
                }
                return (*v..=value.len())
                    .any(|next| matches_from(pattern, value, *p + 1, next, memo));
            }
            b'?' => {
                if *v == value.len() {
                    return false;
                }
                *p += 1;
                *v += 1;
            }
            b'[' => {
                if *v == value.len() {
                    return false;
                }
                let Some((matched, next)) = class_matches(pattern, *p + 1, value[*v]) else {
                    return false;
                };
                if !matched {
                    return false;
                }
                *p = next;
                *v += 1;
            }
            b'\\' if *p + 1 < pattern.len() => {
                *p += 1;
                if value.get(*v) != pattern.get(*p) {
                    return false;
                }
                *p += 1;
                *v += 1;
            }
            byte => {
                if value.get(*v) != Some(&byte) {
                    return false;
                }
                *p += 1;
                *v += 1;
            }
        }
    }
    *v == value.len()
}

fn class_matches(pattern: &[u8], mut index: usize, value: u8) -> Option<(bool, usize)> {
    let negated = matches!(pattern.get(index), Some(b'^'));
    index += usize::from(negated);
    let mut matched = false;
    let mut has_item = false;
    while index < pattern.len() && pattern[index] != b']' {
        has_item = true;
        let start = class_byte(pattern, &mut index)?;
        if pattern.get(index) == Some(&b'-') && pattern.get(index + 1) != Some(&b']') {
            index += 1;
            let end = class_byte(pattern, &mut index)?;
            let (lower, upper) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            matched |= lower <= value && value <= upper;
        } else {
            matched |= start == value;
        }
    }
    if !has_item || pattern.get(index) != Some(&b']') {
        return None;
    }
    Some((matched != negated, index + 1))
}

fn class_byte(pattern: &[u8], index: &mut usize) -> Option<u8> {
    if pattern.get(*index) == Some(&b'\\') && *index + 1 < pattern.len() {
        *index += 1;
    }
    let byte = *pattern.get(*index)?;
    *index += 1;
    Some(byte)
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn supports_redis_style_binary_globs() {
        assert!(matches(b"user:*", b"user:42"));
        assert!(matches(b"file?.[ch]", b"file1.c"));
        assert!(matches(b"[a-c][^0-9]", b"bz"));
        assert!(matches(b"literal\\*", b"literal*"));
        assert!(!matches(b"[a-c]", b"z"));
        assert!(!matches(b"unterminated[", b"unterminated["));
    }
}
