pub(super) struct BoundedItems<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> BoundedItems<T> {
    pub(super) fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    pub(super) fn push(&mut self, value: T) -> molten::error::Result<()> {
        if self.values.len() >= self.maximum {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "{} count exceeds {}",
                self.label, self.maximum
            )));
        }
        self.values.push(value);
        Ok(())
    }

    pub(super) fn into_vec(self) -> Vec<T> {
        self.values
    }
}
