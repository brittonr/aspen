use crate::error::Failure;
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
    Err(Failure::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
}

pub(crate) fn checked_count_sum(left: usize, right: usize, maximum: usize, label: &str) -> Result<usize> {
    let total = left.checked_add(right).ok_or_else(|| Failure::invalid_harness(format!("{label} count overflow")))?;
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
