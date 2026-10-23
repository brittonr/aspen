use super::*;

fn availability(peer: &str, has_item: Option<bool>) -> PeerAvailability {
    PeerAvailability {
        peer_id: peer.to_string(),
        has_item,
    }
}

fn progress_input(peers: Vec<PeerAvailability>) -> RecoveryProgressInput {
    RecoveryProgressInput {
        participant_id: SELF.to_string(),
        required: entry(3, "required"),
        required_index: INDEX,
        is_locally_sufficient: false,
        is_repaired: false,
        admitted_peers: peers,
        is_disrupted: false,
        is_final_drain_observed: true,
        is_remote_repair_requested: true,
        horizon: VirtualHorizon {
            elapsed_ticks: 100,
            limit_ticks: 100,
        },
    }
}

fn verdict(input: &RecoveryProgressInput) -> (ProgressVerdict, ProgressFinding) {
    let progress = evaluate_recovery_progress(input);
    assert_eq!(progress.participant_id, SELF, "the result binds its participant");
    assert_eq!(progress.required_index, INDEX);
    (progress.verdict, progress.finding)
}

// r[verify molten.consensus.storage_fault_recovery_progress]
#[test]
fn local_sufficiency_passes_without_remote_repair_and_fails_an_unnecessary_one() {
    let mut local = progress_input(Vec::new());
    local.is_locally_sufficient = true;
    local.is_remote_repair_requested = false;
    assert_eq!(verdict(&local), (ProgressVerdict::Pass, ProgressFinding::LocallySufficient));
    local.is_remote_repair_requested = true;
    assert_eq!(verdict(&local), (ProgressVerdict::Fail, ProgressFinding::UnnecessaryRemoteRepair));
}

// r[verify molten.consensus.storage_fault_recovery_progress]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn an_available_item_must_be_repaired_within_the_horizon() {
    let peers = || vec![availability("node-b", Some(true)), availability("node-c", Some(false))];
    let mut repaired = progress_input(peers());
    repaired.is_repaired = true;
    assert_eq!(verdict(&repaired), (ProgressVerdict::Pass, ProgressFinding::Repaired));
    assert_eq!(verdict(&progress_input(peers())), (ProgressVerdict::Fail, ProgressFinding::MissedAvailableItem));
    let mut early = progress_input(peers());
    early.horizon.elapsed_ticks = 10;
    assert_eq!(verdict(&early), (ProgressVerdict::NotEvaluated, ProgressFinding::HorizonNotReached));
    let mut disrupted = progress_input(peers());
    disrupted.is_disrupted = true;
    assert_eq!(verdict(&disrupted), (ProgressVerdict::NotEvaluated, ProgressFinding::UnstableWindow));
}

// r[verify molten.consensus.storage_fault_recovery_progress]
#[test]
fn global_absence_needs_complete_observation_and_a_gap_is_incomplete() {
    let absent = progress_input(vec![availability("node-b", Some(false)), availability("node-c", Some(false))]);
    assert_eq!(verdict(&absent), (ProgressVerdict::Pass, ProgressFinding::GloballyUnavailable));
    let gap = progress_input(vec![availability("node-b", Some(false)), availability("node-c", None)]);
    assert_eq!(
        verdict(&gap),
        (ProgressVerdict::Incomplete, ProgressFinding::MissingPeerObservation("node-c".to_string()))
    );
    let mut undrained = absent;
    undrained.is_final_drain_observed = false;
    assert_eq!(verdict(&undrained), (ProgressVerdict::Incomplete, ProgressFinding::MissingFinalDrain));
}

fn admission_input(hard_state: HardStateDecision) -> RepairAdmissionInput {
    RepairAdmissionInput {
        is_persistent_state_suspect: true,
        is_storage_serving: true,
        is_protocol_reconciled: true,
        is_log_or_snapshot_validated: true,
        is_membership_current: true,
        hard_state,
        current_term: 10,
    }
}

// r[verify molten.consensus.repair_admission]
#[test]
fn a_repaired_participant_votes_only_after_full_recovery_admission() {
    assert_eq!(
        admit_repaired_participant(&admission_input(HardStateDecision::KeepLocal)).role,
        Some(RejoinRole::Voter)
    );
    let mut local_repair_only = admission_input(HardStateDecision::KeepLocal);
    local_repair_only.is_protocol_reconciled = false;
    local_repair_only.is_log_or_snapshot_validated = false;
    let blocked = admit_repaired_participant(&local_repair_only);
    assert_eq!(blocked.role, None, "local repair alone never admits a voter");
    assert_eq!(blocked.blockers, vec![
        AdmissionBlocker::ProtocolNotReconciled,
        AdmissionBlocker::LogOrSnapshotNotValidated
    ]);
    let mut quarantined = admission_input(HardStateDecision::KeepLocal);
    quarantined.is_storage_serving = false;
    assert_eq!(admit_repaired_participant(&quarantined).blockers, vec![AdmissionBlocker::StorageNotServing]);
    let mut outsider = admission_input(HardStateDecision::KeepLocal);
    outsider.is_membership_current = false;
    assert_eq!(admit_repaired_participant(&outsider).role, None);
}

// r[verify molten.consensus.repair_admission]
// r[verify molten.consensus.storage_fault_validation]
#[test]
fn a_lost_vote_admits_only_a_non_voting_role_until_the_term_passes_its_fence() {
    let fenced = admit_repaired_participant(&admission_input(HardStateDecision::VoteFenced { through_term: 10 }));
    assert_eq!(fenced.role, Some(RejoinRole::NonVoting));
    assert_eq!(fenced.blockers, vec![AdmissionBlocker::VoteFenced]);
    let past = admit_repaired_participant(&admission_input(HardStateDecision::VoteFenced { through_term: 9 }));
    assert_eq!(past.role, Some(RejoinRole::Voter));
    let unresolved = admit_repaired_participant(&admission_input(HardStateDecision::Wait));
    assert_eq!(unresolved.role, None);
    assert_eq!(unresolved.blockers, vec![AdmissionBlocker::HardStateUnresolved]);
}

fn cohort() -> StorageFaultCohort {
    StorageFaultCohort {
        candidate_ref: entry(0, "candidate").entry_ref,
        molten_profile_ref: entry(0, "molten-profile").entry_ref,
        redb_version: "4.1.0".to_string(),
        kernel_ref: entry(0, "kernel").entry_ref,
        filesystem_ref: entry(0, "filesystem").entry_ref,
        mount_options_ref: entry(0, "mount-options").entry_ref,
        device_profile_ref: entry(0, "device").entry_ref,
        fault_schedule_ref: entry(0, "fault-schedule").entry_ref,
    }
}

fn receipt(cohort: StorageFaultCohort) -> StorageFaultReceipt {
    let mut local = progress_input(Vec::new());
    local.is_locally_sufficient = true;
    local.is_remote_repair_requested = false;
    let mut receipt = StorageFaultReceipt {
        schema: STORAGE_FAULT_RECEIPT_SCHEMA.to_string(),
        receipt_ref: String::new(),
        cohort,
        results: vec![evaluate_recovery_progress(&local)],
        non_claims: STORAGE_FAULT_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    receipt.receipt_ref = identify_storage_fault_receipt(&receipt);
    receipt
}

// r[verify molten.consensus.storage_fault_claim_boundary]
#[test]
fn a_receipt_binds_its_exact_cohort_and_non_claims() {
    assert_eq!(validate_storage_fault_receipt(&receipt(cohort()), &cohort()), Vec::new());

    let mut missing = cohort();
    missing.kernel_ref = String::new();
    assert_eq!(validate_storage_fault_receipt(&receipt(missing), &cohort()), vec![
        StorageFaultReceiptIssue::MissingCohortIdentity("kernel")
    ]);

    let mut changed = cohort();
    changed.filesystem_ref = entry(0, "other-filesystem").entry_ref;
    assert_eq!(validate_storage_fault_receipt(&receipt(changed), &cohort()), vec![
        StorageFaultReceiptIssue::CohortChanged("filesystem")
    ]);

    let mut overclaim = receipt(cohort());
    overclaim.non_claims.pop();
    overclaim.receipt_ref = identify_storage_fault_receipt(&overclaim);
    assert!(matches!(validate_storage_fault_receipt(&overclaim, &cohort()).as_slice(), [
        StorageFaultReceiptIssue::MissingNonClaim(_)
    ]));

    let mut tampered = receipt(cohort());
    tampered.results.clear();
    assert_eq!(validate_storage_fault_receipt(&tampered, &cohort()), vec![
        StorageFaultReceiptIssue::IdentityMismatch
    ]);
}
