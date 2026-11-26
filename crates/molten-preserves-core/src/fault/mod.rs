//! The error convention shared by the `no_std` kernels built on this crate.
//!
//! A kernel never returns the `std` shell's `Failure`. Each kernel defines its own typed
//! `core::error::Error` enum instead:
//!
//! - variants that wrap the shared [`crate::view::ViewError`] and [`crate::bounded::BoundError`],
//!   so record-reader and count-bound denials keep their class and exact message;
//! - domain variants for the kernel's own denials, each rendering the message the `molten` shell
//!   produced before the extraction.
//!
//! The enum implements [`Classified`], naming the shell failure class of each variant. The shell
//! maps any classified error back with `Failure::from_kernel` in `molten-node-host`, which keeps
//! the class and the message: [`FailureKind::InvalidHarness`] becomes
//! `Failure::InvalidHarness(message)` and [`FailureKind::Preserves`] becomes
//! `Failure::Preserves(message)`. `ViewError` and `BoundError` already convert with `From`, so `?`
//! keeps working in shell code. A kernel error crosses into the shell with
//! `.map_err(Failure::from_kernel)` at the boundary; where unchanged shell callers must keep using
//! `?` on a re-exported kernel function, its `From<KernelError> for Failure` sits beside the
//! `ViewError`/`BoundError` conversions in `molten-node-host` and calls `Failure::from_kernel`.
//! Shells branch on the typed variant or on `Failure::class()`, never on message text.
//!
//! ```ignore
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! pub enum RetentionError {
//!     View(molten_preserves_core::view::ViewError),
//!     Bound(molten_preserves_core::bounded::BoundError),
//!     PinWithoutClass { pin: String },
//! }
//!
//! impl Classified for RetentionError {
//!     fn failure_kind(&self) -> FailureKind {
//!         match self {
//!             Self::View(error) => error.failure_kind(),
//!             Self::Bound(error) => error.failure_kind(),
//!             Self::PinWithoutClass { .. } => FailureKind::InvalidHarness,
//!         }
//!     }
//! }
//! ```

/// The `molten` shell failure class a kernel denial belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FailureKind {
    /// An artifact, record, or input the shell rejects: `Failure::InvalidHarness`.
    InvalidHarness,
    /// Bytes or values no Preserves codec accepts: `Failure::Preserves`.
    Preserves,
}

/// A kernel error that knows its shell failure class; its `Display` text is the shell message.
pub trait Classified: core::error::Error {
    fn failure_kind(&self) -> FailureKind;
}
