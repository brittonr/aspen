
pub(crate) fn valid_content_ref(value: &str) -> bool {
    value.len() == CONTENT_REF_LENGTH
        && value.starts_with(BLAKE3_REF_PREFIX)
        && value[BLAKE3_REF_PREFIX.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn valid_ref_collection(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| valid_content_ref(value)) && sorted_unique(values) == values
}

pub(crate) fn sorted_unique(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

/// The time-sliced execution binding: the scheduling quantum and the total
/// execution budget, bound independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlicingProfile {
    pub quantum_fuel: u64,
    pub total_budget_fuel: u64,
}
