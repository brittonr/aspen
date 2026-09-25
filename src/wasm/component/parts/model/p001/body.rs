
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
