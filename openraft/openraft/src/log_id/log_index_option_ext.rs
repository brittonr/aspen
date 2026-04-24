/// This helper trait extracts information from an `Option<LogIndex>`.
///
/// In openraft, `LogIndex` is a `u64`.
pub trait LogIndexOptionExt {
    /// Return the next log index.
    ///
    /// If self is `None`, it returns 0.
    fn next_index(&self) -> u64;

    /// Return the previous log index.
    ///
    /// If self is `None`, it returns `None`.
    fn prev_index(&self) -> Self;

    // TODO: unused, remove it
    /// Performs an "add" operation.
    fn add(&self, v: u64) -> Self;
}

impl LogIndexOptionExt for Option<u64> {
    fn next_index(&self) -> u64 {
        match self {
            None => 0,
            Some(v) => v.saturating_add(1),
        }
    }

    fn prev_index(&self) -> Self {
        match self {
            None => None,
            Some(v) => v.checked_sub(1),
        }
    }

    fn add(&self, v: u64) -> Self {
        let next_index = self.next_index();
        let sum = next_index.saturating_add(v);
        Some(sum).prev_index()
    }
}
