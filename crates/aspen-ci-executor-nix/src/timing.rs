//! Build phase timing for nix build jobs.
//!
//! Tracks the duration of each build phase: importing requisites,
//! building the derivation, and uploading outputs.

use std::collections::HashMap;
use std::time::Duration;

/// Timing breakdown for nix build phases.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildPhaseTimings {
    /// Time spent importing build inputs/requisites.
    pub import_ms: u64,
    /// Time spent building the derivation.
    pub build_ms: u64,
    /// Time spent uploading build outputs to cache.
    pub upload_ms: u64,
}

impl BuildPhaseTimings {
    /// Record import phase duration.
    pub fn record_import(&mut self, duration: Duration) {
        self.import_ms = duration.as_millis().min(u64::MAX as u128) as u64;
    }

    /// Record build phase duration.
    pub fn record_build(&mut self, duration: Duration) {
        self.build_ms = duration.as_millis().min(u64::MAX as u128) as u64;
    }

    /// Record upload phase duration.
    pub fn record_upload(&mut self, duration: Duration) {
        self.upload_ms = duration.as_millis().min(u64::MAX as u128) as u64;
    }

    /// Convert to metadata key-value pairs for `JobOutput.metadata`.
    pub fn to_metadata(self) -> HashMap<String, String> {
        let mut map = HashMap::with_capacity(3);
        map.insert("nix_import_time_ms".to_string(), self.import_ms.to_string());
        map.insert("nix_build_time_ms".to_string(), self.build_ms.to_string());
        map.insert("nix_upload_time_ms".to_string(), self.upload_ms.to_string());
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_timings() {
        let timings = BuildPhaseTimings::default();
        assert_eq!(timings.import_ms, 0);
        assert_eq!(timings.build_ms, 0);
        assert_eq!(timings.upload_ms, 0);
    }

    #[test]
    fn test_record_durations() {
        let mut timings = BuildPhaseTimings::default();

        timings.record_import(Duration::from_millis(150));
        timings.record_build(Duration::from_millis(3500));
        timings.record_upload(Duration::from_millis(750));

        assert_eq!(timings.import_ms, 150);
        assert_eq!(timings.build_ms, 3500);
        assert_eq!(timings.upload_ms, 750);
    }

    #[test]
    fn test_record_durations_truncation() {
        let mut timings = BuildPhaseTimings::default();

        // Test with duration that would overflow u64 when converted to millis
        let very_large_duration = Duration::from_secs(u64::MAX / 500); // Much larger than u64::MAX ms
        timings.record_import(very_large_duration);

        // Should be clamped to u64::MAX
        assert_eq!(timings.import_ms, u64::MAX);
    }

    #[test]
    fn test_to_metadata() {
        let mut timings = BuildPhaseTimings::default();
        timings.record_import(Duration::from_millis(120));
        timings.record_build(Duration::from_millis(2500));
        timings.record_upload(Duration::from_millis(80));

        let metadata = timings.to_metadata();

        assert_eq!(metadata.len(), 3);
        assert_eq!(metadata["nix_import_time_ms"], "120");
        assert_eq!(metadata["nix_build_time_ms"], "2500");
        assert_eq!(metadata["nix_upload_time_ms"], "80");
    }

    #[test]
    fn test_to_metadata_zero_timings() {
        let timings = BuildPhaseTimings::default();
        let metadata = timings.to_metadata();

        assert_eq!(metadata.len(), 3);
        assert_eq!(metadata["nix_import_time_ms"], "0");
        assert_eq!(metadata["nix_build_time_ms"], "0");
        assert_eq!(metadata["nix_upload_time_ms"], "0");
    }
}
