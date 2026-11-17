//! Allocation-capacity floors shared by the bounded walks.
//!
//! `usize::max` is `Ord::max`, and the Lean backend renders that default
//! method by handing the trait instance to a model that expects the
//! comparison function. The floor is spelled out instead, so the translated
//! walk only uses the scalar comparison the backend already models.

/// The larger of `count` and `floor`, without `Ord::max`.
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; at_least computes the commutative maximum of two element counts, so swapping the arguments preserves the result."
)]
pub(crate) const fn at_least(count: usize, floor: usize) -> usize {
    if count < floor {
        floor
    } else {
        count
    }
}
