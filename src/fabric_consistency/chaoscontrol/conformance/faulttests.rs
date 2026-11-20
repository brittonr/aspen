//! Deliberately faulty local adapters. They exercise the consumer boundary and
//! diagnostic classification; they are not Molten guest execution evidence.

use chaoscontrol_smr::SmrConsumerAdapter;

use super::safety::ViolationClass;

struct FixtureAdapter {
    plan: chaoscontrol_smr::SmrWorkloadPlan,
    participant: String,
    lifecycle: chaoscontrol_smr::LifecycleState,
    pending: Vec<(String, Vec<u8>)>,
    applied: Vec<super::safety::AppliedChain>,
    observations: Vec<chaoscontrol_smr::ChainObservation>,
    terminal: Option<chaoscontrol_smr::TerminalStatus>,
}

impl FixtureAdapter {
    fn new(plan: &chaoscontrol_smr::SmrWorkloadPlan, participant: &str) -> Self {
        Self {
            plan: plan.clone(),
            participant: participant.into(),
            lifecycle: chaoscontrol_smr::LifecycleState::Ready,
            pending: Vec::new(),
            applied: Vec::new(),
            observations: Vec::new(),
            terminal: None,
        }
    }

    fn commit_next(&mut self) {
        let (operation, command) = self.pending.remove(0);
        self.commit(&operation, &command);
    }

    fn commit(&mut self, operation: &str, command: &[u8]) {
        let index = u64::try_from(self.applied.len()).expect("fixture index") + 1;
        let prior = self
            .applied
            .last()
            .map_or_else(|| self.plan.genesis_digest.clone(), |apply| apply.chain.digest.clone());
        let receipt = super::testfixtures::id(&format!("adapter-receipt-{operation}-{index}"));
        let apply =
            super::testfixtures::applied(&self.plan, &self.participant, index, &prior, command, operation, &receipt);
        self.observations.push(apply.chain.clone());
        self.applied.push(apply);
    }

    fn commit_duplicate(&mut self, operation: &str, command: &[u8]) {
        self.commit(operation, command);
    }

    fn append_rollback(&mut self) {
        let first = self.applied.first().expect("rollback needs a first observation").chain.clone();
        let event_sequence = u64::try_from(self.observations.len()).expect("fixture sequence") + 1;
        let apply = super::testfixtures::applied_at(
            &self.plan,
            &self.participant,
            first.command_index,
            event_sequence,
            &first.prior_digest,
            b"one",
            "operation-1",
            "rollback-receipt",
        );
        self.observations.push(apply.chain.clone());
        self.applied.push(apply);
    }

    fn applied(&self) -> &[super::safety::AppliedChain] {
        &self.applied
    }
}

impl chaoscontrol_smr::SmrConsumerAdapter for FixtureAdapter {
    fn lifecycle(&self) -> chaoscontrol_smr::LifecycleState {
        self.lifecycle
    }

    fn propose(&mut self, operation_id: &str, command: &[u8]) -> chaoscontrol_smr::ProposalOutcome {
        let maximum_bytes = usize::try_from(self.plan.profile.bounds.maximum_command_bytes).unwrap_or(usize::MAX);
        let maximum_commands = usize::try_from(self.plan.profile.bounds.maximum_commands).unwrap_or(usize::MAX);
        if operation_id.is_empty()
            || command.is_empty()
            || command.len() > maximum_bytes
            || self.pending.len() >= maximum_commands
        {
            return chaoscontrol_smr::ProposalOutcome::DefinitelyRejected;
        }
        self.pending.push((operation_id.to_string(), command.to_vec()));
        chaoscontrol_smr::ProposalOutcome::Indefinite
    }

    fn observations(&self) -> &[chaoscontrol_smr::ChainObservation] {
        &self.observations
    }

    fn observation_mode(&self) -> chaoscontrol_smr::ObservationMode {
        chaoscontrol_smr::ObservationMode::Lossless
    }

    fn dropped_observations(&self) -> u64 {
        0
    }

    fn terminal_status(&self) -> Option<chaoscontrol_smr::TerminalStatus> {
        self.terminal
    }
}

// r[verify molten.testing.chaoscontrol_smr_validation]
#[test]
fn divergent_adapters_are_detected_at_the_shared_index() {
    let plan = super::testfixtures::plan();
    let mut left = FixtureAdapter::new(&plan, "replica-a");
    let mut right = FixtureAdapter::new(&plan, "replica-b");
    assert_eq!(left.propose("operation-1", b"one"), chaoscontrol_smr::ProposalOutcome::Indefinite);
    assert_eq!(right.propose("operation-1", b"other"), chaoscontrol_smr::ProposalOutcome::Indefinite);
    left.commit_next();
    right.commit_next();
    let observations = left.applied().iter().chain(right.applied()).cloned().collect::<Vec<_>>();
    let report = super::safety::evaluate(&plan, &observations).expect("safety evaluation");
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
fn duplicate_application_adapter_is_detected() {
    let plan = super::testfixtures::plan();
    let mut adapter = FixtureAdapter::new(&plan, "replica-a");
    adapter.commit("operation-1", b"one");
    adapter.commit_duplicate("operation-1", b"two");
    let report = super::safety::evaluate(&plan, adapter.applied()).expect("safety evaluation");
    assert_eq!(report.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert!(report.violations.iter().any(|violation| violation.class == ViolationClass::DuplicateApplication));
}

#[test]
fn rollback_adapter_is_detected() {
    let plan = super::testfixtures::plan();
    let mut adapter = FixtureAdapter::new(&plan, "replica-a");
    adapter.commit("operation-1", b"one");
    adapter.commit("operation-2", b"two");
    adapter.append_rollback();
    let report = super::safety::evaluate(&plan, adapter.applied()).expect("safety evaluation");
    assert!(
        report
            .history
            .violations
            .iter()
            .any(|violation| violation.class == chaoscontrol_smr::SafetyViolationClass::Rollback)
    );
    assert!(report.is_safety_failure());
}

// r[verify molten.consensus.chaoscontrol_liveness]
#[test]
fn stalled_recovery_stays_not_evaluated_and_is_not_a_safety_failure() {
    let plan = super::testfixtures::plan();
    let adapter = FixtureAdapter::new(&plan, "replica-a");
    assert_eq!(adapter.lifecycle(), chaoscontrol_smr::LifecycleState::Ready);
    assert!(adapter.observations().is_empty());
    let report = super::safety::evaluate(&plan, adapter.applied()).expect("safety evaluation");
    assert!(!report.is_safety_failure());
    assert!(!report.is_observer_incomplete());
    let facts = chaoscontrol_smr::StabilizationFacts {
        profile_id: plan.profile.liveness.profile_id.clone(),
        quorum_available: false,
        lifecycle_ready: true,
        disruptive_faults_active: false,
        virtual_progress_start: 0,
        virtual_progress_end: 16,
        committed_before: 0,
        committed_after: 0,
    };
    let liveness = chaoscontrol_smr::evaluate_liveness(&plan, Some(&facts)).expect("liveness evaluation");
    assert_eq!(liveness.verdict, chaoscontrol_smr::LivenessVerdict::NotEvaluated);
    assert!(liveness.blockers.iter().any(|blocker| blocker.contains("quorum")));
}

#[test]
fn adapter_bounds_reject_unsupported_commands() {
    let plan = super::testfixtures::plan();
    let mut adapter = FixtureAdapter::new(&plan, "replica-a");
    assert_eq!(adapter.propose("", b"one"), chaoscontrol_smr::ProposalOutcome::DefinitelyRejected);
    assert_eq!(adapter.propose("operation-1", &[0_u8; 33]), chaoscontrol_smr::ProposalOutcome::DefinitelyRejected);
    assert_eq!(adapter.propose("operation-1", b"one"), chaoscontrol_smr::ProposalOutcome::Indefinite);
}
