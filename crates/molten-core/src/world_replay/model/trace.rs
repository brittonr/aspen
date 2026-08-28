use super::super::*;
use crate::world_commit::SnapshotCohortRef;
use crate::world_commit::SnapshotProfileRef;
use crate::world_commit::WorldCommitRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldReplayProfileKind {
    Logical,
    Opaque,
}

impl WorldReplayProfileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Logical => "logical",
            Self::Opaque => "opaque",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayProfile {
    pub kind: WorldReplayProfileKind,
    pub profile_ref: SnapshotProfileRef,
    pub cohort_ref: Option<SnapshotCohortRef>,
    pub snapshot_descriptor_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorldTransitionInputKind {
    Command,
    Event,
    RecordedEffect,
}

impl WorldTransitionInputKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Event => "event",
            Self::RecordedEffect => "recorded-effect",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTransitionInput {
    pub kind: WorldTransitionInputKind,
    pub input_ref: String,
    pub schema_ref: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTransitionStep {
    pub position: u64,
    pub expected_parent: WorldCommitRef,
    pub input: WorldTransitionInput,
    pub profile_ref: SnapshotProfileRef,
    pub expected_successor: WorldCommitRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTransitionTrace {
    pub schema: String,
    pub trace_ref: String,
    pub initial_commit: WorldCommitRef,
    pub profile: WorldReplayProfile,
    pub steps: Vec<WorldTransitionStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReplayBounds {
    pub max_steps: usize,
    pub max_members: usize,
    pub max_member_bytes: u64,
    pub max_total_bytes: u64,
    pub max_field_path_segments: usize,
    pub max_field_segment_bytes: usize,
    pub max_diagnostics: usize,
}

impl Default for WorldReplayBounds {
    fn default() -> Self {
        Self {
            max_steps: MAX_WORLD_REPLAY_STEPS,
            max_members: MAX_WORLD_REPLAY_MEMBERS,
            max_member_bytes: MAX_WORLD_REPLAY_MEMBER_BYTES,
            max_total_bytes: MAX_WORLD_REPLAY_TOTAL_BYTES,
            max_field_path_segments: MAX_WORLD_REPLAY_FIELD_PATH_SEGMENTS,
            max_field_segment_bytes: MAX_WORLD_REPLAY_FIELD_SEGMENT_BYTES,
            max_diagnostics: MAX_WORLD_REPLAY_DIAGNOSTICS,
        }
    }
}

pub fn world_replay_non_claims() -> Vec<String> {
    WORLD_REPLAY_NON_CLAIMS.iter().map(ToString::to_string).collect()
}
