//! Verus verification shims for conditional compilation.
//!
//! This module provides shims for Verus ghost code that compile away completely
//! during normal cargo builds. The ghost annotations have zero runtime overhead.
//!
//! # Architecture
//!
//! The Verus verification workflow has two modes:
//!
//! 1. **Normal cargo builds**: All ghost code compiles to nothing via these shims.
//!
//! 2. **Verus verification**: When running `verus` directly, it provides vstd. The standalone specs
//!    in `crates/aspen-core/verus/` are verified separately.
//!
//! # Usage
//!
//! Production code can use ghost annotations that have zero runtime overhead:
//!
//! ```rust,ignore
//! use crate::spec::verus_shim::*;
//!
//! fn new_timestamp(&self) -> HlcTimestamp {
//!     ghost! {
//!         let pre_state = self.to_spec_state();
//!     }
//!
//!     // ... production code ...
//!
//!     proof! {
//!         // Link to standalone proofs
//!         assert(hlc_invariant(post_state));
//!     }
//! }
//! ```
//!
//! # Zero-Cost Abstraction
//!
//! All ghost code compiles away during normal builds:
//! - `ghost! { ... }` -> nothing
//! - `proof! { ... }` -> nothing
//! - `requires! { ... }` -> nothing
//! - `ensures! { ... }` -> nothing
//! - Ghost types are zero-sized

// ============================================================================
// No-op Macros - Always compile to nothing
// ============================================================================

/// Ghost code block - compiles away completely.
///
/// Use this for ghost variable declarations and tracking:
///
/// ```rust,ignore
/// ghost! {
///     let pre_state = self.to_spec_state();
/// }
/// ```
#[macro_export]
macro_rules! ghost {
    ($($t:tt)*) => {};
}

/// Proof block - compiles away completely.
///
/// Use this for proof assertions and lemma invocations:
///
/// ```rust,ignore
/// proof! {
///     new_timestamp_preserves_monotonicity(pre, post);
///     assert(hlc_invariant(post_state));
/// }
/// ```
#[macro_export]
macro_rules! proof {
    ($($t:tt)*) => {};
}

/// Requires clause - compiles away completely.
///
/// Use this for function preconditions:
///
/// ```rust,ignore
/// requires! {
///     hlc_invariant(state),
///     current_time > 0
/// }
/// ```
#[macro_export]
macro_rules! requires {
    ($($t:tt)*) => {};
}

/// Ensures clause - compiles away completely.
///
/// Use this for function postconditions:
///
/// ```rust,ignore
/// ensures! {
///     hlc_monotonicity(pre, post),
///     hlc_invariant(post)
/// }
/// ```
#[macro_export]
macro_rules! ensures {
    ($($t:tt)*) => {};
}

/// Invariant assertion - compiles away completely.
///
/// Use this for loop invariants:
///
/// ```rust,ignore
/// invariant! {
///     hlc_bounded_drift(state)
/// }
/// ```
#[macro_export]
macro_rules! invariant {
    ($($t:tt)*) => {};
}

/// Assert with proof context - compiles away completely.
///
/// Use this for verified assertions inside proof blocks:
///
/// ```rust,ignore
/// proof_assert! {
///     hlc_invariant(post_state)
/// }
/// ```
#[macro_export]
macro_rules! proof_assert {
    ($($t:tt)*) => {};
}

// Re-export macros at module level for easier use
pub use crate::ensures;
pub use crate::ghost;
pub use crate::invariant;
pub use crate::proof;
pub use crate::proof_assert;
pub use crate::requires;

// ============================================================================
// Ghost Types - Zero-sized
// ============================================================================

/// Ghost wrapper for tracking abstract state.
///
/// This is a zero-sized type that compiles away completely.
/// It's used to annotate where ghost state would be tracked in proofs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ghost<T>(std::marker::PhantomData<T>);

impl<T> Ghost<T> {
    /// Create a new ghost value (no-op, zero cost).
    #[inline(always)]
    pub fn new(_value: T) -> Self {
        Ghost(std::marker::PhantomData)
    }

    /// Track a reference as ghost (no-op, zero cost).
    #[inline(always)]
    pub fn track(_value: &T) -> Self {
        Ghost(std::marker::PhantomData)
    }
}

/// Tracked wrapper for values that participate in proofs.
///
/// This is a zero-sized type that compiles away completely.
/// It's used to annotate where tracked values would be used in proofs.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tracked<T>(std::marker::PhantomData<T>);

impl<T> Tracked<T> {
    /// Create a new tracked value (no-op, zero cost).
    #[inline(always)]
    pub fn new(_value: T) -> Self {
        Tracked(std::marker::PhantomData)
    }
}

// ============================================================================
// Spec Function Stubs
// ============================================================================

/// Marker trait for types that can be converted to their spec representation.
///
/// In actual Verus verification, this would convert exec types to spec types.
/// During normal builds, this is a no-op.
pub trait SpecView {
    /// The specification type corresponding to this exec type.
    type Spec;

    /// Convert to spec representation (no-op during normal builds).
    fn view(&self) -> Self::Spec;
}

// Implement SpecView for common types as no-ops
impl SpecView for u64 {
    type Spec = ();
    #[inline(always)]
    fn view(&self) -> Self::Spec {}
}

impl SpecView for u32 {
    type Spec = ();
    #[inline(always)]
    fn view(&self) -> Self::Spec {}
}

impl SpecView for i64 {
    type Spec = ();
    #[inline(always)]
    fn view(&self) -> Self::Spec {}
}

impl SpecView for String {
    type Spec = ();
    #[inline(always)]
    fn view(&self) -> Self::Spec {}
}

impl<T> SpecView for Option<T> {
    type Spec = ();
    #[inline(always)]
    fn view(&self) -> Self::Spec {}
}

impl<T> SpecView for Vec<T> {
    type Spec = ();
    #[inline(always)]
    fn view(&self) -> Self::Spec {}
}

// ============================================================================
// External Body Attribute Stub
// ============================================================================

/// Marker for external body functions.
///
/// When running actual Verus verification, this tells the verifier to trust
/// the implementation but verify callers against the ensures clause.
///
/// During normal builds, this is a no-op.
pub use core::marker::PhantomData as external_body;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghost_compiles_away() {
        // This should compile to nothing
        ghost! {
            let _x = 42;
            let _y = "ghost variable";
        }
    }

    #[test]
    fn test_proof_compiles_away() {
        // This should compile to nothing
        proof! {
            assert(true);
            let _proof_var = 1 + 1;
        }
    }

    #[test]
    fn test_requires_compiles_away() {
        requires! {
            true,
            1 > 0
        }
    }

    #[test]
    fn test_ensures_compiles_away() {
        ensures! {
            true,
            1 == 1
        }
    }

    #[test]
    fn test_ghost_type_is_zero_sized() {
        assert_eq!(std::mem::size_of::<Ghost<u64>>(), 0);
        assert_eq!(std::mem::size_of::<Ghost<String>>(), 0);
        assert_eq!(std::mem::size_of::<Tracked<String>>(), 0);
    }

    #[test]
    fn test_spec_view_compiles() {
        let val: u64 = 42;
        val.view();

        let val32: u32 = 42;
        val32.view();

        let val_i64: i64 = -42;
        val_i64.view();

        let s = String::from("test");
        s.view();

        let opt: Option<u64> = Some(1);
        opt.view();

        let vec: Vec<u8> = vec![1, 2, 3];
        vec.view();
    }
}
