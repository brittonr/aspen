fn highest() -> molten_core::world_witness::HighestWitnessState {
    molten_core::world_witness::HighestWitnessState {
        subject: molten_core::world_witness::WitnessSubject {
            branch_id: molten_core::world_head::WorldBranchId::new("main").unwrap(),
            claim_ref: molten_core::world_head::WorldHeadClaimRef::new(format!("blake3:{}", "1".repeat(64))).unwrap(),
            generation: u64::MAX,
        },
        checkpoint: molten_core::world_witness::SignedCheckpoint {
            provider_id: "provider-a".to_owned(),
            checkpoint_id: "signed-checkpoint-17".to_owned(),
            revision: u64::MAX,
            signature_verified: true,
            observed_at: 123,
        },
        consistency_predecessor: Some("signed-checkpoint-16".to_owned()),
        quorum: std::collections::BTreeSet::from(["witness-a".to_owned(), "witness-b".to_owned()]),
    }
}

fn receipt() -> crate::world_witness::records::Receipt {
    let highest = highest();
    crate::world_witness::records::Receipt {
        operation_id: "operation-42".to_owned(),
        claim_ref: highest.subject.claim_ref.clone(),
        authentication_ref: molten_core::world_head::WorldHeadAuthenticationDecisionRef::new(format!(
            "blake3:{}",
            "2".repeat(64)
        ))
        .unwrap(),
        authority_ref: molten_core::world_head::WorldHeadAuthorityRef::new(format!("blake3:{}", "3".repeat(64)))
            .unwrap(),
        profile: molten_core::world_witness::AssuranceProfile::IndependentWitnessV1,
        authentication: crate::world_witness::records::RoleDecision::Admitted,
        branch_authorization: crate::world_witness::records::RoleDecision::Admitted,
        witness_currentness: crate::world_witness::records::RoleDecision::Reserved,
        local_persistence: crate::world_witness::records::PersistenceDecision::Committed,
        reconciliation: crate::world_witness::records::ReconciliationDecision::NotRequired,
        highest: Some(highest),
        witness_grants_authority: false,
        non_claims: crate::world_witness::records::required_non_claims(),
    }
}

// r[verify molten.world_witness.currentness]
#[test]
fn highest_preserves_signed_observations_and_full_width_counters() {
    let state = highest();
    assert_eq!(
        crate::world_witness::records::parse_highest(
            &crate::world_witness::records::canonical_highest(&state).unwrap()
        )
        .unwrap(),
        state
    );
    let mut unverified = state;
    unverified.checkpoint.signature_verified = false;
    unverified.consistency_predecessor = None;
    assert_eq!(
        crate::world_witness::records::parse_highest(
            &crate::world_witness::records::canonical_highest(&unverified).unwrap()
        )
        .unwrap(),
        unverified
    );
}

// r[verify molten.world_witness.receipts]
#[test]
fn reserved_local_commit_does_not_become_external_admission() {
    let reserved = receipt();
    let bytes = crate::world_witness::records::receipt::canonical(&reserved).unwrap();
    assert_eq!(crate::world_witness::records::receipt::parse(&bytes).unwrap(), reserved);
    let mut admitted = reserved.clone();
    admitted.witness_currentness = crate::world_witness::records::RoleDecision::Admitted;
    assert_ne!(
        crate::world_witness::records::receipt::identity(&admitted).unwrap(),
        crate::world_witness::records::receipt::identity(&reserved).unwrap()
    );
    assert_eq!(
        crate::world_witness::records::receipt::identity(&reserved).unwrap(),
        crate::preserves_rail::content_ref_from_bytes(&bytes)
    );
}

// r[verify molten.world_witness.receipts]
#[test]
fn currentness_cannot_replace_authorization_or_claim_identity() {
    let mut denied = receipt();
    denied.branch_authorization = crate::world_witness::records::RoleDecision::Denied;
    assert!(crate::world_witness::records::receipt::canonical(&denied).is_err());
    let mut overclaim = receipt();
    overclaim.witness_grants_authority = true;
    assert!(crate::world_witness::records::receipt::canonical(&overclaim).is_err());
    let mut substituted = receipt();
    substituted.claim_ref =
        molten_core::world_head::WorldHeadClaimRef::new(format!("blake3:{}", "4".repeat(64))).unwrap();
    assert!(crate::world_witness::records::receipt::canonical(&substituted).is_err());
    let mut collapsed = receipt();
    collapsed.authority_ref =
        molten_core::world_head::WorldHeadAuthorityRef::new(collapsed.authentication_ref.as_str()).unwrap();
    assert!(crate::world_witness::records::receipt::canonical(&collapsed).is_err());
}

// r[verify molten.world_witness.receipts]
#[test]
fn strong_receipt_cannot_forget_highest_or_nonclaims() {
    let mut missing = receipt();
    missing.highest = None;
    assert!(crate::world_witness::records::receipt::canonical(&missing).is_err());
    let mut local = receipt();
    local.profile = molten_core::world_witness::AssuranceProfile::LocalGenerationV1;
    assert!(crate::world_witness::records::receipt::canonical(&local).is_err());
    let mut omitted = receipt();
    omitted.non_claims.pop();
    assert!(crate::world_witness::records::receipt::canonical(&omitted).is_err());
}

#[test]
fn durable_codec_rejects_trailing_data() {
    let mut bytes = crate::world_witness::records::canonical_highest(&highest()).unwrap();
    bytes.extend(crate::world_witness::records::canonical_highest(&highest()).unwrap());
    assert!(crate::world_witness::records::parse_highest(&bytes).is_err());
}
