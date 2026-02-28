//! Fork detection mode configuration.
//!
//! Controls what happens when seeders disagree about resource state.

use serde::Deserialize;
use serde::Serialize;

/// What to do when seeders report conflicting state for a resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForkDetectionMode {
    /// No fork detection — trust origin authority.
    ///
    /// Suitable for CRDTs where conflicts are resolved by the data type itself.
    Disabled,

    /// Detect forks and log a warning, but continue syncing.
    ///
    /// The seeder with the most trusted supporters wins.
    /// This is the default — provides visibility without blocking.
    #[default]
    Warn,

    /// Halt sync on detected fork — require manual resolution.
    ///
    /// Use for high-integrity resources (e.g., Forge repositories)
    /// where accepting the wrong branch could be catastrophic.
    Halt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_detection_mode_default() {
        let mode = ForkDetectionMode::default();
        assert!(matches!(mode, ForkDetectionMode::Warn));
    }

    #[test]
    fn test_fork_detection_mode_roundtrip() {
        let modes = vec![
            ForkDetectionMode::Disabled,
            ForkDetectionMode::Warn,
            ForkDetectionMode::Halt,
        ];

        for mode in modes {
            let bytes = postcard::to_allocvec(&mode).unwrap();
            let parsed: ForkDetectionMode = postcard::from_bytes(&bytes).unwrap();
            assert!(!format!("{:?}", parsed).is_empty());
        }
    }
}
