use std::collections::BTreeMap;

use molten_core::world_replay::*;

use super::super::*;
use super::model::WorldReplayExportOutcome;
use super::support::*;
use crate::error::MoltenError;
use crate::error::Result;

// r[impl molten.world_replay.capsule]
pub fn export_world_replay_capsule(
    request: &WorldReplayPlanRequest,
    payloads: &[WorldReplayMemberPayload],
    exchange: &mut dyn WorldReplayExchangePort,
) -> Result<WorldReplayExportOutcome> {
    plan_world_replay(request).map_err(core_issues)?;
    let payloads = payload_map(payloads)?;
    if payloads.len() != request.capsule.members.len() {
        return Err(MoltenError::invalid_harness(
            "world replay export payload set does not match the capsule member set",
        ));
    }
    let capsule_record = canonical_world_replay_capsule(&request.capsule)?;
    let mut observations = Vec::with_capacity(request.capsule.members.len());
    for member in &request.capsule.members {
        let payload = payloads
            .get(member.object_ref.as_str())
            .ok_or_else(|| MoltenError::invalid_harness("world replay export payload is missing"))?;
        validate_payload_shape(member, payload, "export")?;
        let observation = exchange.export_member(member, payload)?;
        if observation.object_ref != member.object_ref {
            return Err(MoltenError::invalid_harness("world replay exchange substituted the exported member identity"));
        }
        validate_ref(&observation.observation_ref, "world replay export observation")?;
        observations.push(observation);
    }
    Ok(WorldReplayExportOutcome {
        capsule_record,
        observations,
    })
}

pub fn fetch_world_replay_capsule(
    request: &WorldReplayPlanRequest,
    locator_hints: &BTreeMap<String, String>,
    exchange: &mut dyn WorldReplayExchangePort,
) -> Result<Vec<WorldReplayMemberPayload>> {
    plan_world_replay(request).map_err(core_issues)?;
    if locator_hints.len() != request.capsule.members.len() {
        return Err(MoltenError::invalid_harness("world replay locator set does not match the capsule member set"));
    }
    let mut payloads = Vec::with_capacity(request.capsule.members.len());
    for member in &request.capsule.members {
        let locator_hint = locator_hints
            .get(&member.object_ref)
            .ok_or_else(|| MoltenError::invalid_harness("world replay member locator is missing"))?;
        if locator_hint.is_empty() || locator_hint.len() > MAX_WORLD_REPLAY_TEXT_BYTES {
            return Err(MoltenError::invalid_harness("world replay member locator is empty or overbound"));
        }
        let payload = exchange.import_member(member, locator_hint)?;
        validate_payload_shape(member, &payload, "fetch")?;
        payloads.push(payload);
    }
    for object_ref in locator_hints.keys() {
        if !request.capsule.members.iter().any(|member| member.object_ref == *object_ref) {
            return Err(MoltenError::invalid_harness("world replay locator set contains an undeclared member"));
        }
    }
    Ok(payloads)
}

fn validate_payload_shape(
    member: &WorldReplayCapsuleMember,
    payload: &WorldReplayMemberPayload,
    operation: &str,
) -> Result<()> {
    if payload.object_ref != member.object_ref || u64::try_from(payload.bytes.len()).ok() != Some(member.byte_length) {
        return Err(MoltenError::invalid_harness(format!(
            "world replay {operation} returned a substituted or overbound member"
        )));
    }
    Ok(())
}
