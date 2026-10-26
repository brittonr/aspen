// SPDX-License-Identifier: AGPL-3.0-or-later
//! Application-owned output ports. Implementations depend inward on these
//! contracts; the application never imports concrete adapters.
use molten_orchestrator_core::Allocation;
use molten_orchestrator_core::Fence;
use molten_orchestrator_core::RunId;
use molten_orchestrator_core::TerminalResult;
use molten_orchestrator_core::Transition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortError {
    Unavailable,
    Denied,
    InvalidEvidence,
    UnsupportedProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitDisposition {
    /// The EXACT proposal durably committed, after atomically matching the entire
    /// expected record (including revision AND fence). No weaker acknowledgment.
    Applied,
    /// The expected record did not match; this call performed no mutation.
    Conflict,
    /// Commit may have happened. This is NEVER permission to retry or dispatch.
    Unknown,
}

/// Store namespace and deployment profile are bound when constructing the
/// adapter. No ambient store selection or fallback. CAS must be atomic across
/// every caller for the advertised scope; a process-local fake is NOT durable.
/// Preserve the full record and monotonic revision across recovery; never reset
/// a used assignment to Assigned. Future canonical restore needs its own codec.
pub trait AllocationStore {
    fn load(&mut self, run: RunId) -> Result<Option<Allocation>, PortError>;
    fn compare_and_commit(&mut self, proposal: &Transition) -> Result<CommitDisposition, PortError>;
}

/// Request-bound admission, not an authority grant. Completion binds the exact
/// result, so an adapter can validate its provenance/content, not just its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionAction {
    ReserveDispatch,
    Execute,
    AcceptCompletion(TerminalResult),
}

/// Resolve current policy/capability, generation, assignment, reservations,
/// provenance and exact execution profile. Hash shape or a transport identity
/// alone MUST NOT satisfy admission. Errors deny; no cached-grant fallback.
pub trait AdmissionPort {
    fn check(&mut self, fence: Fence, action: AdmissionAction) -> Result<(), PortError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartDisposition {
    Started,
    RejectedBeforeStart,
    Unknown,
}

/// Worker ingress/execution adapter. Must revalidate the current assignment at
/// its own effect boundary and durably deduplicate exact attempt identities.
/// The controller's earlier check cannot eliminate a revocation race remotely.
/// Returned errors are conservatively treated as possibly started. Only the
/// explicit RejectedBeforeStart disposition asserts that no start occurred.
/// A successful start does NOT assert workload success or exactly-once effects.
pub trait WorkerPort {
    fn start(&mut self, fence: Fence) -> Result<StartDisposition, PortError>;
}
