use std::collections::BTreeMap;

use molten_core::world_replay::*;

use super::super::*;
use super::model::*;
use super::support::*;
use crate::error::MoltenError;
use crate::error::Result;

struct ImportReview {
    verifications: Vec<WorldReplayImportVerification>,
    diagnostics: Vec<String>,
}

// r[impl molten.world_replay.import]
pub fn import_world_replay_capsule(
    request: &WorldReplayPlanRequest,
    payloads: &[WorldReplayMemberPayload],
    ports: WorldReplayImportPorts<'_>,
) -> Result<WorldReplayImportOutcome> {
    plan_world_replay(request).map_err(core_issues)?;
    let payloads = payload_map(payloads)?;
    let review = verify_import_members(request, &payloads, ports.validation)?;
    if !review.diagnostics.is_empty() {
        return denied_import_outcome(request, review, ports.receipts);
    }
    publish_import_outcome(request, &payloads, review.verifications, ports.publication, ports.receipts)
}

fn verify_import_members(
    request: &WorldReplayPlanRequest,
    payloads: &BTreeMap<&str, &WorldReplayMemberPayload>,
    validation: &mut dyn WorldReplayImportValidationPort,
) -> Result<ImportReview> {
    let mut diagnostics = Vec::with_capacity(request.bounds.max_diagnostics);
    let mut verifications = Vec::with_capacity(request.capsule.members.len());
    for member in &request.capsule.members {
        let Some(payload) = payloads.get(member.object_ref.as_str()) else {
            diagnostics.push(format!("missing-member:{}", member.object_ref));
            continue;
        };
        if u64::try_from(payload.bytes.len()).ok() != Some(member.byte_length) {
            diagnostics.push(format!("member-length-mismatch:{}", member.object_ref));
            continue;
        }
        let verification = validation.verify_member(member, payload)?;
        diagnostics.extend(inspect_verification(member, &verification)?);
        verifications.push(verification);
    }
    diagnostics.extend(extra_payload_diagnostics(request, payloads));
    diagnostics.sort();
    diagnostics.dedup();
    if diagnostics.len() > request.bounds.max_diagnostics {
        let retained = request.bounds.max_diagnostics.saturating_sub(1);
        diagnostics.truncate(retained);
        diagnostics.push("diagnostic-bound-exhausted".to_string());
    }
    Ok(ImportReview {
        verifications,
        diagnostics,
    })
}

fn inspect_verification(
    member: &WorldReplayCapsuleMember,
    verification: &WorldReplayImportVerification,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(MAX_WORLD_REPLAY_DIAGNOSTICS);
    validate_ref(&verification.observation_ref, "world replay import verification")?;
    if verification.object_ref != member.object_ref
        || verification.byte_length != member.byte_length
        || !verification.canonical
        || !verification.identity_verified
    {
        diagnostics.push(format!("member-verification-failed:{}", member.object_ref));
    }
    if verification.sensitive_plaintext_found {
        diagnostics.push(format!("plaintext-sensitive-member:{}", member.object_ref));
    }
    if verification.bearer_material_found {
        diagnostics.push(format!("bearer-material-member:{}", member.object_ref));
    }
    if matches!(member.protection, WorldReplayMemberProtection::Ciphertext { .. }) && !verification.decryption_available
    {
        diagnostics.push(format!("decryption-unavailable:{}", member.object_ref));
    }
    Ok(diagnostics)
}

fn extra_payload_diagnostics(
    request: &WorldReplayPlanRequest,
    payloads: &BTreeMap<&str, &WorldReplayMemberPayload>,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(request.bounds.max_diagnostics);
    for payload_ref in payloads.keys() {
        if !request.capsule.members.iter().any(|member| member.object_ref == **payload_ref) {
            diagnostics.push(format!("undeclared-member:{payload_ref}"));
        }
    }
    diagnostics
}

fn denied_import_outcome(
    request: &WorldReplayPlanRequest,
    review: ImportReview,
    receipts: &mut dyn WorldReplayReceiptPort,
) -> Result<WorldReplayImportOutcome> {
    let receipt_input = import_receipt_input(
        request,
        WorldReplayImportDecision::Denied,
        review.verifications.len(),
        None,
        review.diagnostics,
    );
    let (receipt, receipt_record) = canonicalize_world_replay_import_receipt(receipt_input)?;
    publish_exact(receipts, &receipt_record)?;
    Ok(WorldReplayImportOutcome {
        verifications: review.verifications,
        staged_refs: Vec::new(),
        receipt,
        receipt_record,
    })
}

fn publish_import_outcome(
    request: &WorldReplayPlanRequest,
    payloads: &BTreeMap<&str, &WorldReplayMemberPayload>,
    verifications: Vec<WorldReplayImportVerification>,
    publication: &mut dyn WorldReplayImportPublicationPort,
    receipts: &mut dyn WorldReplayReceiptPort,
) -> Result<WorldReplayImportOutcome> {
    let verification_by_ref = verifications
        .iter()
        .map(|verification| (verification.object_ref.as_str(), verification))
        .collect::<BTreeMap<_, _>>();
    let staged_refs = stage_verified_members(request, payloads, &verification_by_ref, publication)?;
    let availability_ref = publication.publish_available(&request.capsule.capsule_ref, &staged_refs)?;
    validate_ref(&availability_ref, "world replay capsule availability")?;
    let receipt_input = import_receipt_input(
        request,
        WorldReplayImportDecision::Available,
        verifications.len(),
        Some(availability_ref),
        Vec::new(),
    );
    let (receipt, receipt_record) = canonicalize_world_replay_import_receipt(receipt_input)?;
    publish_exact(receipts, &receipt_record)?;
    Ok(WorldReplayImportOutcome {
        verifications,
        staged_refs,
        receipt,
        receipt_record,
    })
}

fn stage_verified_members(
    request: &WorldReplayPlanRequest,
    payloads: &BTreeMap<&str, &WorldReplayMemberPayload>,
    verifications: &BTreeMap<&str, &WorldReplayImportVerification>,
    publication: &mut dyn WorldReplayImportPublicationPort,
) -> Result<Vec<String>> {
    let mut staged_refs = Vec::with_capacity(request.capsule.members.len());
    for member in &request.capsule.members {
        let payload = payloads
            .get(member.object_ref.as_str())
            .ok_or_else(|| MoltenError::invalid_harness("verified replay payload disappeared"))?;
        let verification = verifications
            .get(member.object_ref.as_str())
            .ok_or_else(|| MoltenError::invalid_harness("replay member verification disappeared"))?;
        let staged_ref = publication.stage_member(member, payload, verification)?;
        validate_ref(&staged_ref, "world replay staged member")?;
        staged_refs.push(staged_ref);
    }
    Ok(staged_refs)
}

fn import_receipt_input(
    request: &WorldReplayPlanRequest,
    decision: WorldReplayImportDecision,
    verified_members: usize,
    availability_ref: Option<String>,
    diagnostics: Vec<String>,
) -> WorldReplayImportReceipt {
    WorldReplayImportReceipt {
        schema: WORLD_REPLAY_IMPORT_RECEIPT_SCHEMA.to_string(),
        receipt_ref: placeholder_ref(),
        decision,
        capsule_ref: request.capsule.capsule_ref.clone(),
        verified_members,
        availability_ref,
        diagnostics,
        branch_moved: false,
        runtime_activated: false,
        authority_granted: false,
        non_claims: world_replay_non_claims(),
    }
}
