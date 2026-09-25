
impl PromotionReservationPort for Runtime {
    fn admit_promotion_reservation(
        &mut self,
        plan: &WorldBranchAuthorityPlan,
    ) -> Result<WorldBranchPromotionReservationAdmission> {
        self.promotion_calls += 1;
        let (promotion, committed, selected_ref) = promotion_material(plan);
        let mut admission = bind_world_branch_promotion_reservation(plan, &promotion, &committed, &selected_ref)?;
        if self.promotion_dispatch_overclaim {
            admission.dispatch_authorized = true;
        }
        if self.promotion_uncommitted {
            admission.reservation_committed = false;
        }
        Ok(admission)
    }
}

impl BranchActivationPort for Runtime {
    fn activate(&mut self, _decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome> {
        self.activation_calls += 1;
        if self.activation_unknown {
            Ok(ActivationOutcome::Unknown)
        } else {
            Ok(ActivationOutcome::Activated)
        }
    }

    fn reconcile_activation(&mut self, _decision: &WorldBranchActivationDecision) -> Result<ActivationOutcome> {
        self.activation_reconcile_calls += 1;
        Ok(self.activation_reconcile_outcome)
    }
}

impl BranchAuthorityReceiptPort for Runtime {
    fn publish_receipt(&mut self, _receipt_ref: &str, canonical_json: &[u8]) -> Result<()> {
        self.receipt_bytes.push(canonical_json.to_vec());
        Ok(())
    }
}

// r[verify molten.world_branch_authority.verification]
#[test]
fn copyable_and_simulation_paths_use_only_their_admitted_ports() {
    let mut copy_runtime = Runtime::new();
    let copy = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut copy_runtime,
    )
    .expect("copyable execution");
    assert_eq!(copy.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(copy_runtime.receipt_bytes.len(), 3);
    assert_eq!(copy_runtime.grant_calls, 1);
    assert_eq!(copy_runtime.simulation_calls, 0);

    let mut simulation_runtime = Runtime::new();
    let simulation = execute_world_branch_authority(
        &facts(CapabilityKind::ExternalEffect, WorldBranchAction::Simulate),
        &mut simulation_runtime,
    )
    .expect("simulation execution");
    assert_eq!(simulation.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(simulation_runtime.simulation_calls, 1);
    assert_eq!(simulation_runtime.grant_calls, 0);

    let mut fallback_runtime = Runtime::new();
    fallback_runtime.simulation_deterministic = false;
    let fallback = execute_world_branch_authority(
        &facts(CapabilityKind::ExternalEffect, WorldBranchAction::Simulate),
        &mut fallback_runtime,
    )
    .expect("live fallback returns a denial decision");
    assert_eq!(
        fallback.activation.expect("fallback decision").diagnostic,
        WorldBranchAuthorityDiagnostic::SimulationLiveFallback
    );
    assert_eq!(fallback_runtime.activation_calls, 0);
}

#[test]
fn linear_unknown_outcome_reconciles_without_blind_retry() {
    let mut runtime = Runtime::new();
    runtime.transfer_unknown = true;
    let execution = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut runtime,
    )
    .expect("reconciled transfer");
    assert_eq!(execution.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(runtime.transfer_calls, 1);
    assert_eq!(runtime.ownership_calls, 2);

    let mut unresolved = Runtime::new();
    unresolved.transfer_unknown = true;
    unresolved.reconcile_available = false;
    let error = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut unresolved,
    )
    .expect_err("unknown transfer denied");
    assert!(error.to_string().contains("outcome is unknown"));
    assert_eq!(unresolved.transfer_calls, 1);
    assert_eq!(unresolved.activation_calls, 0);

    let mut ambiguous = Runtime::new();
    ambiguous.post_transfer_source_active = true;
    let execution = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut ambiguous,
    )
    .expect("fresh ownership ambiguity emits a denial decision");
    assert_eq!(
        execution.activation.expect("linear denial").diagnostic,
        WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous
    );
    assert_eq!(ambiguous.activation_calls, 0);

    let mut stale_generation = Runtime::new();
    stale_generation.post_transfer_generation = SOURCE_GENERATION;
    let execution = execute_world_branch_authority(
        &facts(CapabilityKind::ExclusiveLease, WorldBranchAction::Transfer),
        &mut stale_generation,
    )
    .expect("stale ownership generation emits a denial decision");
    assert_eq!(
        execution.activation.expect("stale generation denial").diagnostic,
        WorldBranchAuthorityDiagnostic::LinearOwnershipAmbiguous
    );
    assert_eq!(stale_generation.activation_calls, 0);
}

#[test]
fn activation_rechecks_policy_and_reconciles_unknown_outcomes() {
    let mut stale_policy = Runtime::new();
    stale_policy.activation_policy_current = false;
    let stale = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut stale_policy,
    )
    .expect("stale activation policy emits a denial decision");
    assert_eq!(
        stale.activation.expect("stale policy decision").diagnostic,
        WorldBranchAuthorityDiagnostic::PolicyStale
    );
    assert_eq!(stale_policy.policy_calls, 2);
    assert_eq!(stale_policy.activation_calls, 0);

    let mut reconciled = Runtime::new();
    reconciled.activation_unknown = true;
    let activation = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut reconciled,
    )
    .expect("unknown activation is observed before completion");
    assert_eq!(activation.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(reconciled.activation_calls, 1);
    assert_eq!(reconciled.activation_reconcile_calls, 1);

    let mut unresolved = Runtime::new();
    unresolved.activation_unknown = true;
    unresolved.activation_reconcile_outcome = ActivationOutcome::Unknown;
    let activation = execute_world_branch_authority(
        &facts(CapabilityKind::PublicArtifact, WorldBranchAction::Create),
        &mut unresolved,
    )
    .expect("unresolved activation remains explicit");
    assert_eq!(activation.activation_outcome, Some(ActivationOutcome::Unknown));
    assert_eq!(unresolved.activation_calls, 1);
    assert_eq!(unresolved.activation_reconcile_calls, 1);
    let receipt = unresolved.receipt_bytes.last().expect("unknown activation outcome receipt");
    crate::preserves_rail::strict_canonical_decode(receipt).expect("canonical activation outcome receipt");
    assert!(contains_bytes(receipt, b"activation-outcome"));
    assert!(contains_bytes(receipt, b"unknown"));
    assert!(!contains_bytes(receipt, b"secret="));
}

// r[verify molten.world_branch_authority.activation]
#[test]
fn promotion_adapter_requires_complete_committed_exact_reservations() {
    let authority = plan_world_branch_authority(
        DEFAULT_WORLD_BRANCH_AUTHORITY_POLICY_JSON,
        &facts(CapabilityKind::DeferredEffect, WorldBranchAction::Promote),
        &current("promotion-current"),
    );
    let (promotion, committed, selected_ref) = promotion_material(&authority);
    let admission = bind_world_branch_promotion_reservation(&authority, &promotion, &committed, &selected_ref)
        .expect("promotion admission");
    assert!(admission.reservation_committed);
    assert!(admission.complete_reservation_set);
    assert!(!admission.dispatch_authorized);

    assert!(bind_world_branch_promotion_reservation(&authority, &promotion, &[], &selected_ref).is_err());

    let mut uncommitted = committed.clone();
    uncommitted[0].state = WorldReleaseState::Planned;
    assert!(bind_world_branch_promotion_reservation(&authority, &promotion, &uncommitted, &selected_ref,).is_err());

    let mut crossed = committed;
    crossed[0].candidate_head = WorldCommitRef::new(content_ref("crossed-candidate")).expect("candidate ref");
    assert!(bind_world_branch_promotion_reservation(&authority, &promotion, &crossed, &selected_ref).is_err());
}

#[test]
fn bearer_denies_and_promotion_requires_committed_non_dispatching_reservation() {
    let mut bearer_runtime = Runtime::new();
    let bearer = execute_world_branch_authority(
        &facts(CapabilityKind::BearerCredential, WorldBranchAction::Create),
        &mut bearer_runtime,
    )
    .expect("metadata-only denial");
    assert!(!bearer.plan.allowed);
    assert_eq!(bearer_runtime.activation_calls, 0);
    assert_eq!(bearer_runtime.receipt_bytes.len(), 1);

    let mut promotion_runtime = Runtime::new();
    let promotion = execute_world_branch_authority(
        &facts(CapabilityKind::DeferredEffect, WorldBranchAction::Promote),
        &mut promotion_runtime,
    )
    .expect("promotion reservation admission");
    assert_eq!(promotion.activation_outcome, Some(ActivationOutcome::Activated));
    assert_eq!(promotion_runtime.promotion_calls, 1);
    assert_eq!(promotion_runtime.grant_calls, 0);
    assert_eq!(promotion_runtime.simulation_calls, 0);
    assert_eq!(promotion_runtime.transfer_calls, 0);
    assert_eq!(promotion_runtime.activation_calls, 1);
    let promotion_receipt = promotion_runtime.receipt_bytes.last().expect("promotion receipt");
    assert!(contains_bytes(promotion_receipt, b"promotion-plan-ref"));
    assert!(contains_bytes(promotion_receipt, b"release-reservation-ref"));
    assert!(contains_bytes(
        promotion_receipt,
        b"release reservation admission does not authorize effect dispatch"
    ));
    assert!(!contains_bytes(promotion_receipt, b"dispatch-authorized"));

    let mut dispatch_overclaim = Runtime::new();
    dispatch_overclaim.promotion_dispatch_overclaim = true;
    let denied = execute_world_branch_authority(
        &facts(CapabilityKind::DeferredEffect, WorldBranchAction::Promote),
        &mut dispatch_overclaim,
    )
    .expect("dispatch overclaim emits denial receipt");
    assert_eq!(
        denied.activation.expect("activation denial").diagnostic,
        WorldBranchAuthorityDiagnostic::PromotionDispatchOverclaim
    );
    assert_eq!(dispatch_overclaim.activation_calls, 0);

    let mut uncommitted = Runtime::new();
    uncommitted.promotion_uncommitted = true;
    let denied = execute_world_branch_authority(
        &facts(CapabilityKind::DeferredEffect, WorldBranchAction::Promote),
        &mut uncommitted,
    )
    .expect("uncommitted reservation emits denial receipt");
    assert_eq!(
        denied.activation.expect("activation denial").diagnostic,
        WorldBranchAuthorityDiagnostic::PromotionReservationMissing
    );
    assert_eq!(uncommitted.activation_calls, 0);
    assert!(promotion_runtime.receipt_bytes.iter().all(|bytes| {
        let text = String::from_utf8_lossy(bytes);
        !text.contains("secret=") && !text.contains("bearer-token")
    }));
}
