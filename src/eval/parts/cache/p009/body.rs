
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListFilter {
    pub operation: Option<String>,
    pub tier: Option<String>,
    pub status: Option<String>,
    pub dependency_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub capability_ref: Option<String>,
    pub revocation_ref: Option<String>,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySummary {
    pub key_ref: String,
    pub operation: String,
    pub tier: String,
    pub status: String,
    pub value_ref: String,
    pub tombstoned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InvalidateInput {
    pub key_ref: Option<String>,
    pub dependency_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub capability_ref: Option<String>,
    pub revocation_ref: Option<String>,
    pub operation: Option<String>,
    pub reason: String,
    pub retention_evidence: crate::retention::DestructiveEvidence,
    pub apply_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalidation {
    pub decision: String,
    pub invalidated_key_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct SchemaCompatibilityKeyInput<'a> {
    pub expected_identity_ref: &'a str,
    pub actual_identity_ref: &'a str,
    pub alias_ref: Option<&'a str>,
    pub migration_ref: Option<&'a str>,
    pub tool_ref: &'a str,
    pub tool_version: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactClosureKeyInput<'a> {
    pub root_refs: &'a [String],
    pub closure_hash: &'a str,
    pub dependency_refs: &'a [String],
    pub tool_ref: &'a str,
    pub tool_version: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct TranscriptRunKeyInput<'a> {
    pub transcript_ref: &'a str,
    pub closure_hash: &'a str,
    pub dependency_refs: &'a [String],
    pub handler_profile_ref: &'a str,
    pub harness_ref: &'a str,
    pub harness_version: &'a str,
}
