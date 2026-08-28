use std::collections::BTreeMap;

use molten_core::world_replay::*;

use super::super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub(super) fn payload_map(payloads: &[WorldReplayMemberPayload]) -> Result<BTreeMap<&str, &WorldReplayMemberPayload>> {
    let mut map = BTreeMap::new();
    for payload in payloads {
        if map.insert(payload.object_ref.as_str(), payload).is_some() {
            return Err(MoltenError::invalid_harness("world replay payload set contains a duplicate member"));
        }
    }
    Ok(map)
}

pub(super) fn validate_dependency_refs(dependency_refs: &[String]) -> Result<()> {
    if dependency_refs.len() > MAX_WORLD_REPLAY_DEPENDENCY_REFS {
        return Err(MoltenError::invalid_harness("world replay dependency refs exceed the bound"));
    }
    for reference in dependency_refs {
        validate_ref(reference, "world replay dependency")?;
    }
    Ok(())
}

pub(super) fn publish_exact(port: &mut dyn WorldReplayReceiptPort, record: &CanonicalWorldReplayRecord) -> Result<()> {
    let published_ref = port.publish(record)?;
    if published_ref != record.record_ref {
        return Err(MoltenError::invalid_harness(
            "world replay receipt publication substituted the canonical record identity",
        ));
    }
    Ok(())
}

pub(super) fn validate_ref(reference: &str, field_name: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|_| MoltenError::invalid_harness(format!("{field_name} is not a canonical content reference")))
}

pub(super) fn core_issues(issues: Vec<WorldReplayIssue>) -> MoltenError {
    MoltenError::invalid_harness(format!("world replay denied: {issues:?}"))
}

pub(super) fn placeholder_ref() -> String {
    const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    format!("blake3:{ZERO_DIGEST}")
}
