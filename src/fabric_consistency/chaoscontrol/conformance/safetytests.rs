use super::safety::ViolationClass;

fn applied(
    plan: &chaoscontrol_smr::SmrWorkloadPlan,
    participant: &str,
    index: u64,
    prior: &str,
    command: &[u8],
    operation: &str,
    receipt: &str,
) -> super::safety::AppliedChain {
    super::testfixtures::applied(plan, participant, index, prior, command, operation, receipt)
}

// r[verify molten.consensus.chaoscontrol_safety]
#[test]
fn matching_replicas_pass_with_independent_receipts() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let report = super::safety::evaluate(&plan, &[
        applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a"),
        applied(&plan, "replica-b", 1, &genesis, b"one", "operation-1", "receipt-b"),
    ])
    .expect("safety evaluation");
    assert!(report.violations.is_empty());
    assert_eq!(report.verdict(), chaoscontrol_smr::SafetyVerdict::Pass);
    assert!(!report.is_safety_failure());
    assert!(!report.is_observer_incomplete());
}

#[test]
fn one_divergent_digest_fails_even_when_later_indices_agree() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let divergent = applied(&plan, "replica-b", 1, &genesis, b"other", "operation-1", "receipt-b");
    let report = super::safety::evaluate(&plan, &[
        applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a"),
        divergent,
    ])
    .expect("safety evaluation");
    assert_eq!(report.history.verdict, chaoscontrol_smr::SafetyVerdict::Fail);
    assert!(
        report
            .history
            .violations
            .iter()
            .any(|violation| violation.class == chaoscontrol_smr::SafetyViolationClass::Divergence)
    );
    assert!(report.is_safety_failure());
}

#[test]
fn rollback_is_retained_after_later_agreement() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let first = applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a1");
    let second = applied(&plan, "replica-a", 2, &first.chain.digest, b"two", "operation-2", "receipt-a2");
    let returned =
        super::testfixtures::applied_at(&plan, "replica-a", 1, 3, &genesis, b"one", "operation-1", "receipt-a1");
    let report = super::safety::evaluate(&plan, &[first, second, returned]).expect("safety evaluation");
    assert!(
        report
            .history
            .violations
            .iter()
            .any(|violation| violation.class == chaoscontrol_smr::SafetyViolationClass::Rollback)
    );
    assert!(report.is_safety_failure());
}

#[test]
fn duplicate_application_is_not_hidden_by_a_well_formed_chain() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let first = applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a1");
    let duplicate = applied(&plan, "replica-a", 2, &first.chain.digest, b"two", "operation-1", "receipt-a2");
    let report = super::safety::evaluate(&plan, &[first, duplicate]).expect("safety evaluation");
    assert_eq!(report.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert!(report.violations.iter().any(|violation| violation.class == ViolationClass::DuplicateApplication));
    assert_eq!(report.verdict(), chaoscontrol_smr::SafetyVerdict::Fail);
    assert!(report.is_safety_failure());
}

#[test]
fn reused_application_receipt_is_a_molten_violation() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let report = super::safety::evaluate(&plan, &[
        applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "shared-receipt"),
        applied(&plan, "replica-b", 1, &genesis, b"one", "operation-2", "shared-receipt"),
    ])
    .expect("safety evaluation");
    assert_eq!(report.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert!(report.violations.iter().any(|violation| violation.class == ViolationClass::ReceiptReuse));
}

#[test]
fn restart_reobservation_of_an_observed_index_is_not_a_safety_failure() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let first = applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a1");
    let recovered =
        super::testfixtures::applied_at(&plan, "replica-a", 1, 2, &genesis, b"one", "operation-1", "receipt-a1");
    let report = super::safety::evaluate(&plan, &[first, recovered]).expect("safety evaluation");
    assert_eq!(report.history.exact_duplicates, 1);
    assert_eq!(report.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert!(!report.is_safety_failure());
}

#[test]
fn lossless_gap_is_an_observer_failure_not_a_safety_failure() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let first = applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a1");
    let third = super::testfixtures::applied_at(
        &plan,
        "replica-a",
        3,
        2,
        &first.chain.digest,
        b"three",
        "operation-3",
        "receipt-a3",
    );
    let report = super::safety::evaluate(&plan, &[first, third]).expect("safety evaluation");
    assert!(
        report
            .history
            .violations
            .iter()
            .any(|violation| violation.class == chaoscontrol_smr::SafetyViolationClass::LosslessGap)
    );
    assert!(!report.is_safety_failure());
    assert!(report.is_observer_incomplete());
}

#[test]
fn operation_index_and_command_identity_are_bound() {
    let plan = super::testfixtures::plan();
    let genesis = plan.genesis_digest.clone();
    let first = applied(&plan, "replica-a", 1, &genesis, b"one", "operation-1", "receipt-a1");
    let other = applied(&plan, "replica-b", 1, &genesis, b"one", "operation-2", "receipt-b1");
    let moved = applied(&plan, "replica-b", 2, &first.chain.digest, b"two", "operation-1", "receipt-b2");
    let report = super::safety::evaluate(&plan, &[first, other.clone(), moved]).expect("safety evaluation");
    assert!(report.violations.iter().any(|violation| violation.class == ViolationClass::OperationIndexDrift));

    let mut drifted_command = other;
    drifted_command.command_ref = super::testfixtures::id("substituted-command");
    let report = super::safety::evaluate(&plan, &[
        applied(&plan, "replica-a", 1, &genesis, b"one", "operation-2", "receipt-a1"),
        drifted_command,
    ])
    .expect("safety evaluation");
    assert!(report.violations.iter().any(|violation| violation.class == ViolationClass::CommandDrift));
}
