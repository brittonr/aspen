// SPDX-License-Identifier: AGPL-3.0-or-later
#![no_std]
#![forbid(unsafe_code)]
//! Imperative application shell over pure allocation laws and owned ports.
//! No concrete storage, network, execution, clock, or policy implementation.
pub mod ports;

use molten_orchestrator_core::transition;
use molten_orchestrator_core::Allocation;
use molten_orchestrator_core::Event;
use molten_orchestrator_core::Fence;
use molten_orchestrator_core::TerminalResult;
use molten_orchestrator_core::Transition;
use molten_orchestrator_core::TransitionError;
use ports::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryBoundary {
    DispatchCommit,
    ObservationCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceOutcome {
    Committed(Allocation),
    Conflict,
    ReconciliationRequired { fence: Fence, boundary: RecoveryBoundary },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    Read(PortError),
    Missing,
    Invalid(TransitionError),
    Admission(PortError),
}

pub struct Orchestrator<S, A> {
    store: S,
    admission: A,
}

impl<S: AllocationStore, A: AdmissionPort> Orchestrator<S, A> {
    pub const fn new(store: S, admission: A) -> Self {
        Self { store, admission }
    }
    pub fn into_ports(self) -> (S, A) {
        (self.store, self.admission)
    }

    /// One dispatch attempt, with no internal retry. A persisted Dispatching,
    /// Running or Unknown record cannot be dispatched again by this method.
    /// A crash after the first commit is deliberately reconciliation-required,
    /// even if a later reader sees the exact same Dispatching record.
    pub fn dispatch<W: WorkerPort>(&mut self, fence: Fence, worker: &mut W) -> Result<ServiceOutcome, ServiceError> {
        let state = self.load(fence)?;
        let start = transition(&state, fence, Event::BeginDispatch).map_err(ServiceError::Invalid)?;
        self.admission.check(fence, AdmissionAction::ReserveDispatch).map_err(ServiceError::Admission)?;
        match self.store.compare_and_commit(&start) {
            Ok(CommitDisposition::Applied) => {}
            Ok(CommitDisposition::Conflict) => return Ok(ServiceOutcome::Conflict),
            Ok(CommitDisposition::Unknown) | Err(_) => {
                return Ok(recovery(fence, RecoveryBoundary::DispatchCommit));
            }
        }
        // Fresh admission after the durable claim and immediately before routing.
        // A failure here is a definite local pre-start rejection: no worker call.
        let event = if self.admission.check(fence, AdmissionAction::Execute).is_err() {
            Event::StartRejected
        } else {
            match worker.start(fence) {
                Ok(StartDisposition::Started) => Event::StartObserved,
                Ok(StartDisposition::RejectedBeforeStart) => Event::StartRejected,
                Ok(StartDisposition::Unknown) | Err(_) => Event::StartUncertain,
            }
        };
        // No fallible core planning should fail here; still preserve uncertainty
        // if a future core change makes this event inadmissible after dispatch.
        let observation = match transition(&start.next(), fence, event) {
            Ok(proposal) => proposal,
            Err(_) => return Ok(recovery(fence, RecoveryBoundary::ObservationCommit)),
        };
        Ok(self.persist_observation(&observation))
    }

    /// Accept only an independently admitted, exactly fenced terminal observation.
    /// Never invokes the worker, redispatches, releases resources, or changes owner.
    pub fn report_terminal(&mut self, fence: Fence, result: TerminalResult) -> Result<ServiceOutcome, ServiceError> {
        let state = self.load(fence)?;
        let proposal = transition(&state, fence, Event::Complete(result)).map_err(ServiceError::Invalid)?;
        self.admission
            .check(fence, AdmissionAction::AcceptCompletion(result))
            .map_err(ServiceError::Admission)?;
        Ok(self.persist_observation(&proposal))
    }

    fn load(&mut self, fence: Fence) -> Result<Allocation, ServiceError> {
        self.store.load(fence.run).map_err(ServiceError::Read)?.ok_or(ServiceError::Missing)
    }

    fn persist_observation(&mut self, proposal: &Transition) -> ServiceOutcome {
        match self.store.compare_and_commit(proposal) {
            Ok(CommitDisposition::Applied) => ServiceOutcome::Committed(proposal.next()),
            // A competing terminal observation might already have won. Preserve
            // the ambiguity; don't overwrite it or repeat the external operation.
            Ok(CommitDisposition::Conflict | CommitDisposition::Unknown) | Err(_) => {
                recovery(proposal.next().fence(), RecoveryBoundary::ObservationCommit)
            }
        }
    }
}

const fn recovery(fence: Fence, boundary: RecoveryBoundary) -> ServiceOutcome {
    ServiceOutcome::ReconciliationRequired { fence, boundary }
}
