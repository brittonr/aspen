use molten_core::world_replay::*;

use super::super::*;
use super::fixture::*;
use super::ports::*;

#[test]
fn import_publishes_availability_only_after_every_member_verifies() {
    // r[verify molten.world_replay.import]
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let payloads = payloads(&fixture.request.capsule);
    let mut validation = ImportValidation::default();
    let mut publication = ImportPublication::default();
    let mut receipts = Receipts::default();
    let outcome = import_world_replay_capsule(&fixture.request, &payloads, WorldReplayImportPorts {
        validation: &mut validation,
        publication: &mut publication,
        receipts: &mut receipts,
    })
    .expect("complete import");

    assert_eq!(outcome.receipt.decision, WorldReplayImportDecision::Available);
    assert_eq!(publication.staged, fixture.request.capsule.members.len());
    assert_eq!(publication.available, 1);
    assert!(!outcome.receipt.branch_moved);
    assert!(!outcome.receipt.runtime_activated);
    assert!(!outcome.receipt.authority_granted);
}

#[test]
fn import_denies_plaintext_bearer_and_unavailable_ciphertext_without_staging() {
    // r[verify molten.world_replay.verification]
    let mut fixture = fixture(WorldReplayProfileKind::Logical);
    let target_ref = fixture.request.capsule.members[0].object_ref.clone();
    fixture.request.capsule.members[0].protection = WorldReplayMemberProtection::Ciphertext {
        descriptor_ref: digest("protection-descriptor"),
    };
    fixture.request.capsule.capsule_ref =
        identify_world_replay_capsule(&fixture.request.capsule).expect("capsule identity");
    let payloads = payloads(&fixture.request.capsule);
    let mut validation = ImportValidation {
        denied_ref: Some(target_ref),
        sensitive: true,
        bearer: true,
        decryption_available: false,
        ..ImportValidation::default()
    };
    let mut publication = ImportPublication::default();
    let mut receipts = Receipts::default();
    let outcome = import_world_replay_capsule(&fixture.request, &payloads, WorldReplayImportPorts {
        validation: &mut validation,
        publication: &mut publication,
        receipts: &mut receipts,
    })
    .expect("bounded import denial");

    assert_eq!(outcome.receipt.decision, WorldReplayImportDecision::Denied);
    assert_eq!(publication.staged, 0);
    assert_eq!(publication.available, 0);
    assert!(outcome.receipt.diagnostics.iter().any(|item| item.starts_with("plaintext-sensitive")));
    assert!(outcome.receipt.diagnostics.iter().any(|item| item.starts_with("bearer-material")));
    assert!(outcome.receipt.diagnostics.iter().any(|item| item.starts_with("decryption-unavailable")));
}

#[test]
fn import_denies_noncanonical_tampered_missing_and_extra_members() {
    // r[verify molten.world_replay.verification]
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let target_ref = fixture.request.capsule.members[0].object_ref.clone();
    let mut tampered_payloads = payloads(&fixture.request.capsule);
    let mut validation = ImportValidation {
        denied_ref: Some(target_ref),
        fail_canonical: true,
        fail_identity: true,
        ..ImportValidation::default()
    };
    let mut publication = ImportPublication::default();
    let mut receipts = Receipts::default();
    let tampered = import_world_replay_capsule(&fixture.request, &tampered_payloads, WorldReplayImportPorts {
        validation: &mut validation,
        publication: &mut publication,
        receipts: &mut receipts,
    })
    .expect("tampered import denial");
    assert_eq!(tampered.receipt.decision, WorldReplayImportDecision::Denied);
    assert!(tampered.receipt.diagnostics.iter().any(|item| item.starts_with("member-verification-failed")));
    assert_eq!(publication.staged, 0);

    tampered_payloads.remove(0);
    tampered_payloads.push(WorldReplayMemberPayload {
        object_ref: digest("undeclared-payload"),
        bytes: vec![0; usize::try_from(MEMBER_BYTES).expect("fixture length")],
    });
    let mut validation = ImportValidation::default();
    let mut publication = ImportPublication::default();
    let mut receipts = Receipts::default();
    let incomplete = import_world_replay_capsule(&fixture.request, &tampered_payloads, WorldReplayImportPorts {
        validation: &mut validation,
        publication: &mut publication,
        receipts: &mut receipts,
    })
    .expect("missing and extra import denial");
    assert_eq!(incomplete.receipt.decision, WorldReplayImportDecision::Denied);
    assert!(incomplete.receipt.diagnostics.iter().any(|item| item.starts_with("missing-member")));
    assert!(incomplete.receipt.diagnostics.iter().any(|item| item.starts_with("undeclared-member")));
    assert_eq!(publication.staged, 0);
}

#[test]
fn canonical_records_are_stable_and_import_receipts_reject_authority_claims() {
    let fixture = fixture(WorldReplayProfileKind::Logical);
    let first = canonical_world_transition_trace(&fixture.request.trace).expect("trace record");
    let second = canonical_world_transition_trace(&fixture.request.trace).expect("stable trace record");
    assert_eq!(first.record_ref, second.record_ref);
    assert_eq!(first.bytes, second.bytes);
    crate::preserves_rail::strict_canonical_decode(&first.bytes).expect("strict canonical trace");

    let error = canonicalize_world_replay_import_receipt(WorldReplayImportReceipt {
        schema: WORLD_REPLAY_IMPORT_RECEIPT_SCHEMA.to_string(),
        receipt_ref: placeholder_ref(),
        decision: WorldReplayImportDecision::Available,
        capsule_ref: fixture.request.capsule.capsule_ref.clone(),
        verified_members: fixture.request.capsule.members.len(),
        availability_ref: Some(digest("availability")),
        diagnostics: Vec::new(),
        branch_moved: false,
        runtime_activated: false,
        authority_granted: true,
        non_claims: world_replay_non_claims(),
    })
    .expect_err("import-as-authority denied");
    assert!(error.to_string().contains("forbidden mutation or authority"));
}
