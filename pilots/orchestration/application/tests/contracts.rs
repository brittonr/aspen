// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test adapters only. None of these fakes establishes durability, authority,
//! cross-process exclusion, transport delivery, or external exactly-once effects.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::num::NonZeroU64;
use std::rc::Rc;

use molten_orchestrator_application::ports::*;
use molten_orchestrator_application::*;
use molten_orchestrator_core::*;

type Trace = Rc<RefCell<Vec<&'static str>>>;

#[derive(Clone, Copy)]
enum WriteMode {
    Apply,
    Conflict,
    UnknownBefore,
    UnknownAfter,
    ErrorAfter,
}
struct Store {
    state: Option<Allocation>,
    modes: VecDeque<WriteMode>,
    trace: Trace,
}
impl AllocationStore for Store {
    fn load(&mut self, run: RunId) -> Result<Option<Allocation>, PortError> {
        self.trace.borrow_mut().push("load");
        Ok(self.state.filter(|state| state.fence().run == run))
    }
    fn compare_and_commit(&mut self, proposal: &Transition) -> Result<CommitDisposition, PortError> {
        self.trace.borrow_mut().push("commit");
        if self.state != Some(proposal.expected()) {
            return Ok(CommitDisposition::Conflict);
        }
        match self.modes.pop_front().unwrap_or(WriteMode::Apply) {
            WriteMode::Apply => {
                self.state = Some(proposal.next());
                Ok(CommitDisposition::Applied)
            }
            WriteMode::Conflict => Ok(CommitDisposition::Conflict),
            WriteMode::UnknownBefore => Ok(CommitDisposition::Unknown),
            WriteMode::UnknownAfter => {
                self.state = Some(proposal.next());
                Ok(CommitDisposition::Unknown)
            }
            WriteMode::ErrorAfter => {
                self.state = Some(proposal.next());
                Err(PortError::Unavailable)
            }
        }
    }
}
struct Admission {
    reject: Option<AdmissionAction>,
    trace: Trace,
}
impl AdmissionPort for Admission {
    fn check(&mut self, _: Fence, action: AdmissionAction) -> Result<(), PortError> {
        self.trace.borrow_mut().push(match action {
            AdmissionAction::ReserveDispatch => "admit-reserve",
            AdmissionAction::Execute => "admit-execute",
            AdmissionAction::AcceptCompletion(_) => "admit-complete",
        });
        if Some(action) == self.reject {
            Err(PortError::Denied)
        } else {
            Ok(())
        }
    }
}
struct Worker {
    result: Result<StartDisposition, PortError>,
    starts: usize,
    trace: Trace,
}
impl WorkerPort for Worker {
    fn start(&mut self, _: Fence) -> Result<StartDisposition, PortError> {
        self.trace.borrow_mut().push("worker-start");
        self.starts += 1;
        self.result
    }
}
fn fence() -> Fence {
    Fence {
        run: RunId([1; 32]),
        assignment: AssignmentRef([2; 32]),
        workload: WorkloadRef([3; 32]),
        node: NodeRef([4; 32]),
        generation: NonZeroU64::new(1).unwrap(),
        epoch: NonZeroU64::new(1).unwrap(),
    }
}
fn fixture(modes: &[WriteMode]) -> (Orchestrator<Store, Admission>, Worker, Trace) {
    let trace = Trace::default();
    let store = Store {
        state: Some(Allocation::assigned(fence())),
        modes: modes.iter().copied().collect(),
        trace: trace.clone(),
    };
    let admission = Admission {
        reject: None,
        trace: trace.clone(),
    };
    let worker = Worker {
        result: Ok(StartDisposition::Started),
        starts: 0,
        trace: trace.clone(),
    };
    (Orchestrator::new(store, admission), worker, trace)
}
fn result() -> TerminalResult {
    TerminalResult::Succeeded(ResultRef([7; 32]))
}
fn assert_phase(outcome: ServiceOutcome, phase: Phase) {
    match outcome {
        ServiceOutcome::Committed(state) => assert_eq!(state.phase(), phase),
        other => panic!("expected committed {phase:?}, got {other:?}"),
    }
}

#[test]
fn persist_then_recheck_then_dispatch_then_record() {
    let (mut service, mut worker, trace) = fixture(&[]);
    assert_phase(service.dispatch(fence(), &mut worker).unwrap(), Phase::Running);
    assert_eq!(&*trace.borrow(), &[
        "load",
        "admit-reserve",
        "commit",
        "admit-execute",
        "worker-start",
        "commit",
    ]);
    assert_eq!(worker.starts, 1);
}

#[test]
fn conflict_performs_no_worker_effect() {
    let (mut service, mut worker, trace) = fixture(&[WriteMode::Conflict]);
    assert_eq!(service.dispatch(fence(), &mut worker).unwrap(), ServiceOutcome::Conflict);
    assert_eq!(worker.starts, 0);
    assert_eq!(&*trace.borrow(), &["load", "admit-reserve", "commit"]);
}

#[test]
fn unknown_before_commit_does_not_dispatch_or_blindly_retry() {
    let (mut service, mut worker, trace) = fixture(&[WriteMode::UnknownBefore]);
    assert_eq!(service.dispatch(fence(), &mut worker).unwrap(), ServiceOutcome::ReconciliationRequired {
        fence: fence(),
        boundary: RecoveryBoundary::DispatchCommit,
    });
    assert_eq!(worker.starts, 0);
    assert_eq!(trace.borrow().iter().filter(|x| **x == "commit").count(), 1);
}

#[test]
fn unknown_after_commit_cannot_be_redispatched_after_host_restart() {
    let (mut service, mut worker, _) = fixture(&[WriteMode::UnknownAfter]);
    assert!(matches!(
        service.dispatch(fence(), &mut worker).unwrap(),
        ServiceOutcome::ReconciliationRequired { .. }
    ));
    let (store, admission) = service.into_ports();
    assert_eq!(store.state.unwrap().phase(), Phase::Dispatching);
    let mut reopened = Orchestrator::new(store, admission);
    assert!(matches!(reopened.dispatch(fence(), &mut worker), Err(ServiceError::Invalid(_))));
    assert_eq!(worker.starts, 0);
}

#[test]
fn storage_error_after_claim_is_not_a_prestart_failure_or_retry_grant() {
    let (mut service, mut worker, _) = fixture(&[WriteMode::ErrorAfter]);
    assert!(matches!(service.dispatch(fence(), &mut worker).unwrap(), ServiceOutcome::ReconciliationRequired {
        boundary: RecoveryBoundary::DispatchCommit,
        ..
    }));
    assert_eq!(worker.starts, 0);
    assert_eq!(service.into_ports().0.state.unwrap().phase(), Phase::Dispatching);
}

#[test]
fn admission_failure_before_claim_has_no_mutation() {
    let (service, mut worker, trace) = fixture(&[]);
    let (store, mut admission) = service.into_ports();
    admission.reject = Some(AdmissionAction::ReserveDispatch);
    let mut service = Orchestrator::new(store, admission);
    assert_eq!(service.dispatch(fence(), &mut worker), Err(ServiceError::Admission(PortError::Denied)));
    assert_eq!(worker.starts, 0);
    assert_eq!(&*trace.borrow(), &["load", "admit-reserve"]);
}

#[test]
fn revocation_after_commit_prevents_worker_effect() {
    let (service, mut worker, trace) = fixture(&[]);
    let (store, mut admission) = service.into_ports();
    admission.reject = Some(AdmissionAction::Execute);
    let mut service = Orchestrator::new(store, admission);
    assert_phase(service.dispatch(fence(), &mut worker).unwrap(), Phase::RejectedBeforeStart);
    assert_eq!(worker.starts, 0);
    assert_eq!(&*trace.borrow(), &["load", "admit-reserve", "commit", "admit-execute", "commit"]);
}

#[test]
fn lost_observation_commit_never_repeats_start() {
    for mode in [
        WriteMode::UnknownBefore,
        WriteMode::UnknownAfter,
        WriteMode::ErrorAfter,
        WriteMode::Conflict,
    ] {
        let (mut service, mut worker, _) = fixture(&[WriteMode::Apply, mode]);
        assert!(matches!(service.dispatch(fence(), &mut worker).unwrap(), ServiceOutcome::ReconciliationRequired {
            boundary: RecoveryBoundary::ObservationCommit,
            ..
        }));
        let (store, admission) = service.into_ports();
        let mut reopened = Orchestrator::new(store, admission);
        assert!(reopened.dispatch(fence(), &mut worker).is_err());
        assert_eq!(worker.starts, 1);
    }
}

#[test]
fn worker_error_is_recorded_as_unknown_not_safe_to_retry() {
    let (mut service, mut worker, _) = fixture(&[]);
    worker.result = Err(PortError::Unavailable);
    assert_phase(service.dispatch(fence(), &mut worker).unwrap(), Phase::Unknown);
    assert!(service.dispatch(fence(), &mut worker).is_err());
    assert_eq!(worker.starts, 1);
}

#[test]
fn explicit_unknown_worker_result_is_not_safe_to_retry() {
    let (mut service, mut worker, _) = fixture(&[]);
    worker.result = Ok(StartDisposition::Unknown);
    assert_phase(service.dispatch(fence(), &mut worker).unwrap(), Phase::Unknown);
    assert!(service.dispatch(fence(), &mut worker).is_err());
    assert_eq!(worker.starts, 1);
}

#[test]
fn explicit_worker_rejection_does_not_implicitly_create_another_attempt() {
    let (mut service, mut worker, _) = fixture(&[]);
    worker.result = Ok(StartDisposition::RejectedBeforeStart);
    assert_phase(service.dispatch(fence(), &mut worker).unwrap(), Phase::RejectedBeforeStart);
    assert!(service.dispatch(fence(), &mut worker).is_err());
    assert_eq!(worker.starts, 1);
}

#[test]
fn current_completion_can_reconcile_unknown_without_worker_call() {
    let (mut service, mut worker, trace) = fixture(&[]);
    worker.result = Ok(StartDisposition::Unknown);
    service.dispatch(fence(), &mut worker).unwrap();
    trace.borrow_mut().clear();
    assert_phase(service.report_terminal(fence(), result()).unwrap(), Phase::Terminal(result()));
    assert_eq!(&*trace.borrow(), &["load", "admit-complete", "commit"]);
    assert_eq!(worker.starts, 1);
}

#[test]
fn stale_completion_has_no_admission_or_commit_effect() {
    let (mut service, mut worker, trace) = fixture(&[]);
    service.dispatch(fence(), &mut worker).unwrap();
    trace.borrow_mut().clear();
    let stale = Fence {
        epoch: NonZeroU64::new(2).unwrap(),
        ..fence()
    };
    assert_eq!(service.report_terminal(stale, result()), Err(ServiceError::Invalid(TransitionError::FenceMismatch)));
    assert_eq!(&*trace.borrow(), &["load"]);
}

#[test]
fn completion_admission_is_bound_to_exact_result() {
    let (mut service, mut worker, _) = fixture(&[]);
    service.dispatch(fence(), &mut worker).unwrap();
    let (store, mut admission) = service.into_ports();
    admission.reject = Some(AdmissionAction::AcceptCompletion(result()));
    let mut service = Orchestrator::new(store, admission);
    assert_eq!(service.report_terminal(fence(), result()), Err(ServiceError::Admission(PortError::Denied)));
    assert_eq!(service.into_ports().0.state.unwrap().phase(), Phase::Running);
}

#[test]
fn completion_is_not_overwritten_by_second_result() {
    let (mut service, mut worker, _) = fixture(&[]);
    service.dispatch(fence(), &mut worker).unwrap();
    service.report_terminal(fence(), result()).unwrap();
    let different = TerminalResult::Failed(ResultRef([8; 32]));
    assert!(service.report_terminal(fence(), different).is_err());
    assert_eq!(service.into_ports().0.state.unwrap().phase(), Phase::Terminal(result()));
}

#[test]
fn two_stale_proposals_cannot_both_commit_in_the_contract_fake() {
    let (service, _, _) = fixture(&[]);
    let (mut store, _) = service.into_ports();
    let snapshot = store.state.unwrap();
    let first = transition(&snapshot, fence(), Event::BeginDispatch).unwrap();
    let second = transition(&snapshot, fence(), Event::BeginDispatch).unwrap();
    assert_eq!(store.compare_and_commit(&first), Ok(CommitDisposition::Applied));
    assert_eq!(store.compare_and_commit(&second), Ok(CommitDisposition::Conflict));
}

#[test]
fn missing_run_never_dispatches() {
    let (service, mut worker, _) = fixture(&[]);
    let (mut store, admission) = service.into_ports();
    store.state = None;
    let mut service = Orchestrator::new(store, admission);
    assert_eq!(service.dispatch(fence(), &mut worker), Err(ServiceError::Missing));
    assert_eq!(worker.starts, 0);
}
