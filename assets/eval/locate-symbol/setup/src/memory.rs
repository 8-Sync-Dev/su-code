/// In-memory key/value store for the harness.
pub struct Store {
    pub entries: Vec<(String, String)>,
}

/// Seed the harness memory with default entries.
pub fn seed_harness_memory() -> Store {
    Store {
        entries: vec![("project".into(), "eval".into())],
    }
}
