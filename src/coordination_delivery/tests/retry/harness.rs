use molten_core::coordination_delivery::*;
use molten_core::fabric_time::AdmittedTimeProfile;

use super::super::super::*;
use super::super::support::*;
use super::super::trace::*;

pub(super) struct RetryHarness {
    pub(super) policy: DeliveryPolicy,
    pub(super) manifest: DeliveryManifest,
    pub(super) time: AdmittedTimeProfile,
    pub(super) commit: MemoryCommitPort,
    pub(super) timers: MemoryTimerPort,
    pub(super) statuses: MemoryStatusPort,
    trace: DeliveryCallTrace,
    next_operation: u64,
}

impl RetryHarness {
    pub(super) fn new(policy: DeliveryPolicy) -> Self {
        assert!(validate_delivery_policy(&policy).is_ok());
        let manifest = manifest(&policy);
        assert!(validate_delivery_manifest(&manifest, &policy).is_ok());
        let trace = DeliveryCallTrace::default();
        let mut commit = MemoryCommitPort::new(CommitMode::Apply);
        commit.trace = Some(trace.clone());
        let mut timers = timer_port(false);
        timers.trace = Some(trace.clone());
        let statuses = MemoryStatusPort {
            trace: Some(trace.clone()),
            ..MemoryStatusPort::default()
        };
        Self {
            time: time_profile(&manifest),
            policy,
            manifest,
            commit,
            timers,
            statuses,
            trace,
            next_operation: 1,
        }
    }

    pub(super) fn apply(&mut self, now: u64, operation: DeliveryOperation) -> DeliveryServiceOutcome {
        let mut request = request(&self.manifest, '0', now, operation);
        request.operation_id = format!("blake3:{:0width$x}", self.next_operation, width = BLAKE3_HEX_LENGTH);
        self.next_operation = self.next_operation.checked_add(1).expect("bounded test operation sequence");
        let expected = self.commit.head.as_ref().map_or_else(empty_expected, expected);
        let outcome =
            apply_delivery_request(&mut self.commit, &mut self.timers, &mut self.statuses, &DeliveryServiceRequest {
                manifest: &self.manifest,
                policy: &self.policy,
                time_profile: &self.time,
                host_binding: &host_binding(&self.manifest),
                expected,
                request: &request,
            })
            .expect("delivery service outcome");
        assert!(!outcome.receipt.bytes.is_empty());
        assert_eq!(outcome.transition.request_ref, identify_delivery_request(&request));
        outcome
    }

    pub(super) fn enqueue(&mut self) {
        let operation = enqueue_request(&self.manifest, '0').operation;
        let outcome = self.apply(INITIAL_TICK, operation);
        assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
        assert_eq!(outcome.transition.kind, DeliveryTransitionKind::Enqueued);
    }

    pub(super) fn claim(&mut self, now: u64) -> DeliveryToken {
        let outcome = self.apply(now, DeliveryOperation::Claim);
        assert_eq!(outcome.receipt.status, DeliveryServiceStatus::Applied);
        assert_eq!(outcome.transition.kind, DeliveryTransitionKind::Claimed);
        outcome.transition.token.expect("claimed token")
    }

    pub(super) fn clear_trace(&self) {
        self.trace.lock().expect("test call trace").clear();
    }

    pub(super) fn calls(&self) -> Vec<DeliveryPortCall> {
        self.trace.lock().expect("test call trace").clone()
    }
}
