//! Bounded growth of collections and checked count conversions.
//!
//! Every helper denies before mutating: a push or extend that would exceed its maximum leaves the
//! collection unchanged. Counts and maxima are [`Count`]s, a host `usize` length or a wire `u64`
//! bound, compared as `u64`; no public signature names `usize`, and narrowing to a host index is
//! the checked [`Count::to_usize`]. The `molten` shell's `crate::bounded` delegates here and
//! converts [`BoundError`] into `Failure::InvalidHarness` with the same message.

mod denial;

use alloc::string::ToString;

pub use denial::BoundError;

/// A count or bound: a host `usize` length or a wire `u64` value.
pub trait Count: Copy + core::fmt::Display {
    /// The count as `u64`; `None` when a host count does not fit.
    fn widened(self) -> Option<u64>;

    /// The count as a host index: `{label} {count} does not fit usize` when it cannot be one.
    fn to_usize(self, label: &str) -> Result<usize, BoundError>;

    /// The count as a wire `u64`: `{label} {count} does not fit u64` when it cannot be one.
    fn to_u64(self, label: &str) -> Result<u64, BoundError>;
}

impl Count for usize {
    fn widened(self) -> Option<u64> {
        u64::try_from(self).ok()
    }

    fn to_usize(self, _label: &str) -> Result<usize, BoundError> {
        Ok(self)
    }

    fn to_u64(self, label: &str) -> Result<u64, BoundError> {
        u64::try_from(self).map_err(|_| BoundError::U64Overflow {
            label: label.to_string(),
            value: self.to_string(),
        })
    }
}

impl Count for u64 {
    fn widened(self) -> Option<u64> {
        Some(self)
    }

    fn to_usize(self, label: &str) -> Result<usize, BoundError> {
        usize::try_from(self).map_err(|_| BoundError::UsizeOverflow {
            label: label.to_string(),
            value: self,
        })
    }

    fn to_u64(self, _label: &str) -> Result<u64, BoundError> {
        Ok(self)
    }
}

/// A growable collection of `T` whose length can be checked before it grows.
pub trait VecSink<T> {
    fn item_count(&self) -> usize;
    fn reserve_items(&mut self, additional: usize);
    fn push_item(&mut self, value: T);
    fn extend_cloned_items(&mut self, incoming: &[T])
    where T: Clone;
    fn extend_items(&mut self, incoming: impl IntoIterator<Item = T>);
}

impl<T> VecSink<T> for alloc::vec::Vec<T> {
    fn item_count(&self) -> usize {
        self.len()
    }

    fn reserve_items(&mut self, additional: usize) {
        self.reserve(additional);
    }

    fn push_item(&mut self, value: T) {
        self.push(value);
    }

    fn extend_cloned_items(&mut self, incoming: &[T])
    where T: Clone {
        self.extend(incoming.iter().cloned());
    }

    fn extend_items(&mut self, incoming: impl IntoIterator<Item = T>) {
        self.extend(incoming);
    }
}

fn overflow(label: &str) -> BoundError {
    BoundError::CountOverflow {
        label: label.to_string(),
    }
}

/// `{label} count {count} exceeds maximum {maximum}` past the maximum; a count or maximum that
/// does not fit `u64` denies as `{label} count overflow`.
// r[impl molten.runtime_spine.no_std_preserves_packed_kernel]
pub fn ensure_count_at_most(count: impl Count, maximum: impl Count, label: &str) -> Result<(), BoundError> {
    let count = count.widened().ok_or_else(|| overflow(label))?;
    let maximum = maximum.widened().ok_or_else(|| overflow(label))?;
    if count <= maximum {
        return Ok(());
    }
    Err(BoundError::CountExceeded {
        label: label.to_string(),
        count,
        maximum,
    })
}

/// `left + right`, denied as `{label} count overflow` when the sum overflows and as exceeding the
/// maximum past it.
pub fn checked_count_sum(
    left: impl Count,
    right: impl Count,
    maximum: impl Count,
    label: &str,
) -> Result<u64, BoundError> {
    let total = left
        .widened()
        .zip(right.widened())
        .and_then(|(left, right)| left.checked_add(right))
        .ok_or_else(|| overflow(label))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

pub fn push<T>(values: &mut impl VecSink<T>, value: T, maximum: impl Count, label: &str) -> Result<(), BoundError> {
    checked_count_sum(values.item_count(), 1_u64, maximum, label)?;
    values.push_item(value);
    Ok(())
}

pub fn extend<T>(
    values: &mut impl VecSink<T>,
    incoming: &[T],
    maximum: impl Count,
    label: &str,
) -> Result<(), BoundError>
where
    T: Clone,
{
    checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(incoming.len());
    values.extend_cloned_items(incoming);
    Ok(())
}

pub trait PushLimited<T>: VecSink<T> {
    fn push_limited(&mut self, value: T, maximum: impl Count, label: &str) -> Result<(), BoundError>;
}

impl<T, S> PushLimited<T> for S
where S: VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: impl Count, label: &str) -> Result<(), BoundError> {
        push(self, value, maximum, label)
    }
}

/// A diagnostics list with one fixed maximum and label.
pub struct DiagnosticSink<'a, S, M> {
    sink: &'a mut S,
    maximum: M,
    label: &'a str,
}

impl<'a, S, M> DiagnosticSink<'a, S, M>
where
    S: VecSink<alloc::string::String>,
    M: Count,
{
    pub fn new(sink: &'a mut S, maximum: M, label: &'a str) -> Self {
        Self { sink, maximum, label }
    }

    pub fn push(&mut self, diagnostic: impl Into<alloc::string::String>) -> Result<(), BoundError> {
        push(self.sink, diagnostic.into(), self.maximum, self.label)
    }
}

#[cfg(test)]
mod tests;
