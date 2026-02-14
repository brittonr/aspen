//! COB store types for events and conflict reporting.

use aspen_core::hlc::SerializableTimestamp;

use super::super::change::CobType;
use crate::identity::RepoId;

/// Event emitted when a COB change is stored.
#[derive(Debug, Clone)]
pub struct CobUpdateEvent {
    /// Repository ID.
    pub repo_id: RepoId,
    /// COB type (Issue, Patch, etc.).
    pub cob_type: CobType,
    /// COB instance ID.
    pub cob_id: blake3::Hash,
    /// Hash of the new change.
    pub change_hash: blake3::Hash,
    /// Public key of the author.
    pub author: iroh::PublicKey,
    /// HLC timestamp for deterministic ordering.
    pub hlc_timestamp: SerializableTimestamp,
}

/// A conflicting value for a scalar field.
#[derive(Debug, Clone)]
pub struct ConflictingValue {
    /// Hash of the change that set this value.
    pub change_hash: blake3::Hash,
    /// The value (serialized as string for display).
    pub value: String,
    /// HLC timestamp when the value was set.
    pub hlc_timestamp: SerializableTimestamp,
    /// Author who set the value.
    pub author: iroh::PublicKey,
}

/// A field conflict requiring explicit resolution.
#[derive(Debug, Clone)]
pub struct FieldConflict {
    /// Name of the conflicting field ("title", "body", "state").
    pub field: String,
    /// Competing values from different heads.
    pub values: Vec<ConflictingValue>,
}

/// Report of conflicts in a COB.
#[derive(Debug, Clone)]
pub struct ConflictReport {
    /// COB type.
    pub cob_type: CobType,
    /// COB instance ID.
    pub cob_id: blake3::Hash,
    /// Current heads (divergent history).
    pub heads: Vec<blake3::Hash>,
    /// Fields with conflicting values requiring explicit resolution.
    pub field_conflicts: Vec<FieldConflict>,
}
