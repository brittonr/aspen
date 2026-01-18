//! Blame/annotation support for Pijul repositories.
//!
//! This module provides attribution information for files, showing which
//! changes contributed to the current state of a file.
//!
//! ## Limitations
//!
//! Currently, this provides change-level attribution (which changes are in
//! the channel) rather than per-line attribution. Full per-line blame requires
//! traversing libpijul's internal graph structure, which is planned for a
//! future release.
//!
//! ## Usage
//!
//! ```ignore
//! use aspen_pijul::blame::BlameResult;
//!
//! // BlameResult is populated by PijulStore::blame_file()
//! let result = store.blame_file(&repo_id, "main", "src/main.rs").await?;
//! for attr in &result.attributions {
//!     println!("{}: {} - {}", attr.change_hash, attr.author, attr.message);
//! }
//! ```

use super::types::ChangeHash;

/// Attribution entry for a file.
///
/// Represents a change that contributed to the current state of a file.
#[derive(Debug, Clone)]
pub struct FileAttribution {
    /// Hash of the change.
    pub change_hash: ChangeHash,
    /// Author name.
    pub author: Option<String>,
    /// Author email.
    pub author_email: Option<String>,
    /// Change message (first line).
    pub message: String,
    /// Timestamp when the change was recorded.
    pub recorded_at_ms: u64,
    /// Path of the file within the change.
    pub path: String,
    /// Type of change: "add", "modify", "delete", "rename".
    pub change_type: String,
}

/// Blame result containing all attributions for a file.
#[derive(Debug, Clone)]
pub struct BlameResult {
    /// Path of the file being blamed.
    pub path: String,
    /// Channel the blame was performed on.
    pub channel: String,
    /// List of changes that contributed to this file, in reverse chronological order.
    pub attributions: Vec<FileAttribution>,
    /// Whether the file currently exists in the channel.
    pub file_exists: bool,
}

impl BlameResult {
    /// Create a new empty blame result.
    pub fn new(path: String, channel: String) -> Self {
        Self {
            path,
            channel,
            attributions: Vec::new(),
            file_exists: true,
        }
    }

    /// Add an attribution entry.
    pub fn add_attribution(&mut self, attribution: FileAttribution) {
        self.attributions.push(attribution);
    }
}

#[cfg(test)]
mod tests {
    // Tests would go here once we have a test harness for pristine operations
}
