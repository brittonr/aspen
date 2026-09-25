
fn parse_retry(value: &str) -> crate::error::Result<RetryDisposition> {
    match value {
        "not-applicable" => Ok(RetryDisposition::NotApplicable),
        "higher-level-policy-required" => Ok(RetryDisposition::HigherLevelPolicyRequired),
        "unsafe-without-reconciliation" => Ok(RetryDisposition::UnsafeWithoutReconciliation),
        other => Err(crate::error::MoltenError::invalid_harness(format!("unsupported retry disposition {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_membership_remains_bounded() {
        assert_eq!(expected_members().len(), EXPECTED_MEMBER_COUNT);
    }
}
