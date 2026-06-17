pub(super) fn parse_build_params(values: &[String]) -> molten::error::Result<Vec<molten::provenance::BuildParam>> {
    let mut params = BoundedItems::new(super::PROVENANCE_CLI_EVIDENCE_LIMIT, "provenance build params");
    for value in values {
        let Some((key, param_value)) = value.split_once('=') else {
            return Err(molten::error::MoltenError::invalid_harness(format!(
                "provenance build param `{value}` must use key=value"
            )));
        };
        params.push(molten::provenance::BuildParam {
            key: key.to_string(),
            value: param_value.to_string(),
        })?;
    }
    Ok(params.into_vec())
}

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
