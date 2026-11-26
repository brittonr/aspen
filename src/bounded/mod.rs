//! Bounded collection growth for the shell, delegating to `molten_preserves_core::bounded`.
//!
//! The kernel owns the deny-before-mutate rules and the messages; these wrappers keep the
//! `crate::error::Result` signatures existing callers use, converting each `BoundError` into the
//! same `Failure::InvalidHarness` message through `From<BoundError> for Failure`.

use molten_preserves_core::bounded::Count;
pub(crate) use molten_preserves_core::bounded::VecSink;

pub(crate) fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> crate::error::Result<()> {
    Ok(molten_preserves_core::bounded::ensure_count_at_most(count, maximum, label)?)
}

pub(crate) fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> crate::error::Result<usize> {
    let total = molten_preserves_core::bounded::checked_count_sum(left, right, maximum, label)?;
    Ok(total.to_usize(label)?)
}

/// Converts a public `u64` bound or count into the host `usize`, denying values the platform cannot
/// index.
pub(crate) fn usize_from_u64(value: u64, label: &str) -> crate::error::Result<usize> {
    Ok(value.to_usize(label)?)
}

/// Converts a host `usize` count into the public `u64` representation, denying a count that does
/// not fit.
pub(crate) fn u64_from_usize(value: usize, label: &str) -> crate::error::Result<u64> {
    Ok(value.to_u64(label)?)
}

pub(crate) fn push<T>(values: &mut impl VecSink<T>, value: T, maximum: usize, label: &str) -> crate::error::Result<()> {
    Ok(molten_preserves_core::bounded::push(values, value, maximum, label)?)
}

pub(crate) fn extend<T>(
    values: &mut impl VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> crate::error::Result<()>
where
    T: Clone,
{
    Ok(molten_preserves_core::bounded::extend(values, incoming, maximum, label)?)
}

pub(crate) trait PushLimited<T>: VecSink<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> crate::error::Result<()>;
}

impl<T, S> PushLimited<T> for S
where S: VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> crate::error::Result<()> {
        push(self, value, maximum, label)
    }
}

pub(crate) struct DiagnosticSink<'a, S>(molten_preserves_core::bounded::DiagnosticSink<'a, S, usize>);

impl<'a, S> DiagnosticSink<'a, S>
where S: VecSink<String>
{
    pub(crate) fn new(sink: &'a mut S, maximum: usize, label: &'a str) -> Self {
        Self(molten_preserves_core::bounded::DiagnosticSink::new(sink, maximum, label))
    }

    pub(crate) fn push(&mut self, diagnostic: impl Into<String>) -> crate::error::Result<()> {
        Ok(self.0.push(diagnostic)?)
    }
}

#[cfg(test)]
mod tests {
    const TEST_LIMIT: usize = 2;

    /// The kernel's denial reaches shell callers as the same `InvalidHarness` failure as before.
    // r[verify molten.runtime_spine.no_std_preserves_packed_kernel]
    #[test]
    fn shell_wrappers_keep_the_failure_class_and_message() {
        let mut values = vec!["first", "second"];
        let error = super::push(&mut values, "third", TEST_LIMIT, "test values").expect_err("one past limit");
        assert_eq!(error, crate::error::Failure::invalid_harness("test values count 3 exceeds maximum 2"));
        assert_eq!(values, ["first", "second"]);

        let overflow = super::checked_count_sum(usize::MAX, 1, usize::MAX, "overflow values").expect_err("overflow");
        assert_eq!(overflow, crate::error::Failure::invalid_harness("overflow values count overflow"));
    }

    #[test]
    fn shell_sinks_delegate_to_the_kernel_bounds() {
        let mut diagnostics: Vec<String> = Vec::new();
        let mut sink = super::DiagnosticSink::new(&mut diagnostics, 1, "diagnostics");
        sink.push("first").expect("first diagnostic");
        let error = sink.push("second").expect_err("past the limit");
        assert_eq!(error.to_string(), "invalid harness artifact: diagnostics count 2 exceeds maximum 1");
        let mut values = vec![1_u8];
        super::PushLimited::push_limited(&mut values, 2, TEST_LIMIT, "values").expect("within limit");
        assert!(super::extend(&mut values, &[3], TEST_LIMIT, "values").is_err());
        assert_eq!(super::ensure_count_at_most(TEST_LIMIT, TEST_LIMIT, "values"), Ok(()));
    }
}
