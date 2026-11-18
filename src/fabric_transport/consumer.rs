//! Application-owned current-authority and protected local socket effects.
use std::future::Future;

/// The consumer resolves membership, principal, trust, capability, expiry and
/// revocation for the exact operation. Transport identity never substitutes for
/// this port. No production default or local fixture fallback is supplied.
pub trait ProtectedSocketService {
    /// Called only after the typed source gate and authenticated binding.
    ///
    /// Raw observations cannot call this costly handler:
    /// ```compile_fail
    /// use molten::fabric_transport::{ProtectedSocketService, ReceiveObservation, DirectStreamRequest};
    /// fn bypass<S: ProtectedSocketService>(service: &mut S, raw: ReceiveObservation, request: &DirectStreamRequest) {
    ///     let _ = service.authorize(raw, request);
    /// }
    /// ```
    /// Returns a canonical current authority evidence reference, not a grant
    /// synthesized from successful connectivity.
    fn authorize(
        &mut self,
        inbound: super::GatedInbound,
        request: &super::DirectStreamRequest,
    ) -> crate::fabric::FabricPortResult<String>;

    /// Recheck current authority immediately before opening and every frame.
    fn check_current(&mut self, admitted: &super::direct::AdmittedOperation) -> crate::fabric::FabricPortResult<()>;

    /// Resolve an already admitted resource through the consumer's protected
    /// capability root. The transport never reconstructs ambient socket paths.
    ///
    /// The transport treats this future as possibly effectful from its first
    /// poll. If it is cancelled, exceeds its deadline or returns any error, the
    /// operation's delivery remains uncertain even when no socket was returned;
    /// an error class does not prove that no protected effect occurred.
    fn open(
        &mut self,
        admitted: &super::direct::AdmittedOperation,
    ) -> impl Future<Output = crate::fabric::FabricPortResult<tokio::net::UnixStream>> + Send;
}
