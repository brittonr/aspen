use molten_core::world_replay::*;
use preserves::IOValue;

use super::CanonicalWorldReplayRecord;
use crate::error::MoltenError;
use crate::error::Result;

const WORLD_REPLAY_RECORD_CONTEXT: &str = "onixresearch.molten.world-replay.record.v1";

pub(super) struct ReceiptValidationInput<'a> {
    pub trace_ref: &'a str,
    pub capsule_ref: &'a str,
    pub profile_ref: &'a str,
    pub dependencies: &'a [String],
    pub diagnostics: &'a [String],
    pub non_claims: &'a [String],
}

pub(super) fn validate_receipt_common(input: &ReceiptValidationInput<'_>) -> Result<()> {
    validate_ref(input.trace_ref, "replay trace")?;
    validate_ref(input.capsule_ref, "replay capsule")?;
    validate_ref(input.profile_ref, "replay profile")?;
    if input.dependencies.len() > MAX_WORLD_REPLAY_DEPENDENCY_REFS {
        return Err(MoltenError::invalid_harness("world replay dependency refs exceed the bound"));
    }
    for reference in input.dependencies {
        validate_ref(reference, "replay dependency")?;
    }
    validate_diagnostics(input.diagnostics)?;
    require_non_claims(input.non_claims)
}

pub(super) fn validate_diagnostics(diagnostics: &[String]) -> Result<()> {
    if diagnostics.len() > MAX_WORLD_REPLAY_DIAGNOSTICS
        || diagnostics
            .iter()
            .any(|diagnostic| diagnostic.is_empty() || diagnostic.len() > MAX_WORLD_REPLAY_TEXT_BYTES)
    {
        return Err(MoltenError::invalid_harness("world replay diagnostics are empty or overbound"));
    }
    Ok(())
}

pub(super) fn require_non_claims(non_claims: &[String]) -> Result<()> {
    if non_claims != world_replay_non_claims() {
        return Err(MoltenError::invalid_harness("world replay non-claims are incomplete"));
    }
    Ok(())
}

pub(super) fn core_issues(issues: Vec<WorldReplayIssue>) -> Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("world replay record denied: {issues:?}")))
    }
}

pub(super) fn core_issue(issue: WorldReplayIssue) -> MoltenError {
    MoltenError::invalid_harness(format!("world replay identity denied: {issue:?}"))
}

pub(super) fn validate_ref(reference: &str, field_name: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|_| MoltenError::invalid_harness(format!("{field_name} is not a canonical content reference")))
}

pub(super) fn canonical(
    identity_kind: &str,
    record_kind: &'static str,
    value: IOValue,
) -> Result<CanonicalWorldReplayRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    if bytes.len() > MAX_WORLD_REPLAY_CANONICAL_BYTES {
        return Err(MoltenError::invalid_harness("world replay canonical record exceeds the byte bound"));
    }
    let record_ref = domain_identity_with_context(WORLD_REPLAY_RECORD_CONTEXT, identity_kind, &bytes)?;
    Ok(CanonicalWorldReplayRecord {
        kind: record_kind,
        record_ref,
        value,
        bytes,
    })
}

pub(super) fn domain_identity(context: &'static str, value: &IOValue) -> Result<String> {
    let bytes = crate::preserves_rail::canonical_bytes(value)?;
    domain_identity_with_context(context, "identity", &bytes)
}

fn domain_identity_with_context(context: &'static str, kind: &str, bytes: &[u8]) -> Result<String> {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    update(&mut hasher, kind)?;
    let length = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness("world replay canonical length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(bytes);
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<()> {
    let length = u64::try_from(value.len())
        .map_err(|_| MoltenError::invalid_harness("world replay identity field length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}

pub(super) fn non_claims_value(non_claims: &[String]) -> IOValue {
    field("non-claims", sequence(non_claims.iter().map(string).collect()))
}

pub(super) fn optional_ref(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record("none", Vec::new()), |reference| record("some", vec![string(reference)]))
}

pub(super) fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

pub(super) fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

pub(super) fn usize_value(value: usize) -> IOValue {
    u64::try_from(value).map_or_else(|_| number(u64::MAX), number)
}

pub(super) fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

pub(super) fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

pub(super) fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

pub(super) fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
