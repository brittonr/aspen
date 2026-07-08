use crate::error::MoltenError;
use crate::error::Result;

pub(crate) trait VecSink<T> {
    fn item_count(&self) -> usize;
    fn reserve_items(&mut self, additional: usize);
    fn push_item(&mut self, value: T);
    fn extend_cloned_items(&mut self, incoming: &[T])
    where T: Clone;
}

impl<T> VecSink<T> for Vec<T> {
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
}

pub(crate) fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
}

pub(crate) fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left
        .checked_add(right)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    ensure_count_at_most(total, maximum, label)?;
    Ok(total)
}

pub(crate) fn push_bounded<T>(values: &mut impl VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    checked_count_sum(values.item_count(), 1, maximum, label)?;
    values.push_item(value);
    Ok(())
}

pub(crate) fn extend_bounded<T>(
    values: &mut impl VecSink<T>,
    incoming: &[T],
    maximum: usize,
    label: &str,
) -> Result<()>
where
    T: Clone,
{
    checked_count_sum(values.item_count(), incoming.len(), maximum, label)?;
    values.reserve_items(incoming.len());
    values.extend_cloned_items(incoming);
    Ok(())
}

pub(crate) trait PushLimited<T>: VecSink<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T, S> PushLimited<T> for S
where S: VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        push_bounded(self, value, maximum, label)
    }
}

pub(crate) struct DiagnosticSink<'a, S> {
    sink: &'a mut S,
    maximum: usize,
    label: &'a str,
}

impl<'a, S> DiagnosticSink<'a, S>
where S: VecSink<String>
{
    pub(crate) fn new(sink: &'a mut S, maximum: usize, label: &'a str) -> Self {
        Self { sink, maximum, label }
    }

    pub(crate) fn push(&mut self, diagnostic: impl Into<String>) -> Result<()> {
        push_bounded(self.sink, diagnostic.into(), self.maximum, self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LIMIT: usize = 2;
    const TEST_OVERFLOW_LEFT: usize = usize::MAX;
    const TEST_ONE: usize = 1;

    #[test]
    fn bounded_push_allows_exact_limit_and_denies_one_past_without_mutation() {
        let mut values = Vec::new();
        push_bounded(&mut values, "first", TEST_LIMIT, "test values").expect("first push");
        push_bounded(&mut values, "second", TEST_LIMIT, "test values").expect("second push");
        assert_eq!(values, vec!["first", "second"]);

        let before = values.clone();
        let error =
            push_bounded(&mut values, "third", TEST_LIMIT, "test values").expect_err("one-past-limit push denies");
        assert!(error.to_string().contains("test values count 3 exceeds maximum 2"));
        assert_eq!(values, before);
    }

    #[test]
    fn checked_count_sum_denies_arithmetic_overflow() {
        let error = checked_count_sum(TEST_OVERFLOW_LEFT, TEST_ONE, TEST_OVERFLOW_LEFT, "overflow values")
            .expect_err("overflow denies");
        assert!(error.to_string().contains("overflow values count overflow"));
    }

    #[test]
    fn bounded_extend_denies_before_appending_any_item() {
        let mut values = vec!["existing".to_string()];
        let incoming = vec!["first".to_string(), "second".to_string()];
        let before = values.clone();
        let error =
            extend_bounded(&mut values, &incoming, TEST_LIMIT, "extend values").expect_err("extend over limit denies");
        assert!(error.to_string().contains("extend values count 3 exceeds maximum 2"));
        assert_eq!(values, before);
    }

    #[test]
    fn diagnostic_sink_uses_bounded_push_behavior() {
        let mut diagnostics = Vec::new();
        {
            let mut sink = DiagnosticSink::new(&mut diagnostics, TEST_LIMIT, "diagnostics");
            sink.push("first").expect("first diagnostic");
            sink.push("second").expect("second diagnostic");
            let error = sink.push("third").expect_err("diagnostic overflow denies");
            assert!(error.to_string().contains("diagnostics count 3 exceeds maximum 2"));
        }
        assert_eq!(diagnostics, vec!["first".to_string(), "second".to_string()]);
    }
}
