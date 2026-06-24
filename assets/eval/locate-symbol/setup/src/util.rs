/// Join two path segments.
pub fn join_paths(a: &str, b: &str) -> String {
    format!("{}/{}", a.trim_end_matches('/'), b.trim_start_matches('/'))
}

/// Count words in a string.
pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}
