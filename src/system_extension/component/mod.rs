//! Manifest-installed Component Model callbacks using the shared, admitted runtime.
//!
//! Guests receive `canonical_callback_envelope` and must return the existing
//! materialized callback-outcome wire format. A host response is not a callback
//! outcome: guests must explicitly construct their outputs and checkpoint/state.
//! Dispatcher owns lifecycle/event admission; this executor owns materialization,
//! sandboxed execution, output decoding, and publication after outcome admission.
//!
//! `Executor` records each `HostedObservation`, including original malformed
//! publication receipts. Unknown acceptance is sticky: no later invocation or
//! restart can proceed until reconciliation resolves the external publication.

mod admission;
mod contract;
mod execution;
mod publication;

pub use admission::Installation;
pub use contract::Binding;
pub use contract::ComponentCallbackHostFactory;
pub use contract::HostedObservation;
pub use contract::PublicationObservation;
pub use execution::Executor;
