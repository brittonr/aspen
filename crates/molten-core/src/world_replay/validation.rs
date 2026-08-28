mod capsule;
mod closure;
mod request;
mod trace;

pub use capsule::*;
pub use closure::*;
pub use request::*;
pub use trace::*;

use super::*;

const BLAKE3_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LENGTH: usize = 64;

pub(crate) fn validate_world_replay_bounds(bounds: &WorldReplayBounds) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    if bounds.max_steps == 0 || bounds.max_steps > MAX_WORLD_REPLAY_STEPS {
        issues.push(WorldReplayIssue::InvalidBounds("max-steps"));
    }
    if bounds.max_members == 0 || bounds.max_members > MAX_WORLD_REPLAY_MEMBERS {
        issues.push(WorldReplayIssue::InvalidBounds("max-members"));
    }
    if bounds.max_member_bytes == 0 || bounds.max_member_bytes > MAX_WORLD_REPLAY_MEMBER_BYTES {
        issues.push(WorldReplayIssue::InvalidBounds("max-member-bytes"));
    }
    if bounds.max_total_bytes < bounds.max_member_bytes || bounds.max_total_bytes > MAX_WORLD_REPLAY_TOTAL_BYTES {
        issues.push(WorldReplayIssue::InvalidBounds("max-total-bytes"));
    }
    if bounds.max_field_path_segments == 0 || bounds.max_field_path_segments > MAX_WORLD_REPLAY_FIELD_PATH_SEGMENTS {
        issues.push(WorldReplayIssue::InvalidBounds("max-field-path-segments"));
    }
    if bounds.max_field_segment_bytes == 0 || bounds.max_field_segment_bytes > MAX_WORLD_REPLAY_FIELD_SEGMENT_BYTES {
        issues.push(WorldReplayIssue::InvalidBounds("max-field-segment-bytes"));
    }
    if bounds.max_diagnostics == 0 || bounds.max_diagnostics > MAX_WORLD_REPLAY_DIAGNOSTICS {
        issues.push(WorldReplayIssue::InvalidBounds("max-diagnostics"));
    }
    issues
}

pub(crate) fn validate_world_replay_profile(profile: &WorldReplayProfile) -> Vec<WorldReplayIssue> {
    let mut issues = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    match profile.kind {
        WorldReplayProfileKind::Logical => {
            if profile.cohort_ref.is_some() {
                issues.push(WorldReplayIssue::LogicalCohortUnexpected);
            }
            if profile.snapshot_descriptor_ref.is_some() {
                issues.push(WorldReplayIssue::LogicalSnapshotDescriptorUnexpected);
            }
        }
        WorldReplayProfileKind::Opaque => {
            if profile.cohort_ref.is_none() {
                issues.push(WorldReplayIssue::OpaqueCohortMissing);
            }
            if profile.snapshot_descriptor_ref.is_none() {
                issues.push(WorldReplayIssue::OpaqueSnapshotDescriptorMissing);
            }
        }
    }
    if profile.snapshot_descriptor_ref.as_ref().is_some_and(|reference| !valid_reference(reference)) {
        issues.push(WorldReplayIssue::InvalidReference("snapshot-descriptor-ref"));
    }
    issues
}

pub(crate) fn valid_reference(value: &str) -> bool {
    value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

pub(crate) fn valid_field_segment(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/' | b'[' | b']'))
}

pub(crate) fn sorted_issues(mut issues: Vec<WorldReplayIssue>) -> Vec<WorldReplayIssue> {
    issues.sort();
    issues.dedup();
    issues
}

pub(crate) fn bounded_sorted_issues(issues: Vec<WorldReplayIssue>, maximum: usize) -> Vec<WorldReplayIssue> {
    let mut issues = sorted_issues(issues);
    if issues.len() > maximum {
        let retained = maximum.saturating_sub(1);
        issues.truncate(retained);
        issues.push(WorldReplayIssue::DiagnosticLimitExceeded);
        issues.sort();
    }
    issues
}
