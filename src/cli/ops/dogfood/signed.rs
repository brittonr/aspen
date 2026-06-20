const MEMBER_LIMIT: usize = 64;
const _: () = assert!(MEMBER_LIMIT <= 100_000);

struct BoundedItems<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> BoundedItems<T> {
    fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    fn push(&mut self, value: T) -> molten::error::Result<()> {
        if self.values.len() >= self.maximum {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "{} count exceeds {}",
                self.label, self.maximum
            )));
        }
        self.values.push(value);
        Ok(())
    }

    fn into_vec(self) -> Vec<T> {
        self.values
    }
}

pub(super) fn read_preserves_files(paths: &[std::path::PathBuf]) -> molten::error::Result<Vec<preserves::IOValue>> {
    let mut values = BoundedItems::new(MEMBER_LIMIT, "dogfood signed members");
    for path in paths {
        values.push(super::io::read_preserves_file(path)?)?;
    }
    Ok(values.into_vec())
}
