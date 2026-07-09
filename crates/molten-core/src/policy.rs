#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFieldSet<'a> {
    pub generated_fields: &'a [&'a str],
    pub required_schema_fields: &'a [&'a str],
    pub source_export_ref: &'a str,
    pub checked_export_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyFreshnessIssue {
    pub field: String,
    pub message: String,
}

pub fn validate_policy_freshness(input: &PolicyFieldSet<'_>) -> Vec<PolicyFreshnessIssue> {
    let mut issues = Vec::new();
    for required_field in input.required_schema_fields {
        if !input.generated_fields.iter().any(|field| field == required_field) {
            issues.push(PolicyFreshnessIssue {
                field: (*required_field).to_string(),
                message: format!("generated policy is missing required schema field {required_field}"),
            });
        }
    }
    if input.source_export_ref != input.checked_export_ref {
        issues.push(PolicyFreshnessIssue {
            field: "checked_export_ref".to_string(),
            message: "checked generated policy ref does not match reviewed source export".to_string(),
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY_SCHEMA_COMPATIBILITY_FIELD: &str = "policy_schema_compatibility";
    const TRACEABILITY_FIELD: &str = "traceability_policy";
    const STACK_PROVENANCE_FIELD: &str = "stack_provenance_gate";
    const RUNTIME_EVIDENCE_FIELD: &str = "runtime_evidence_policy";
    const POLICY_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const STALE_POLICY_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const MISSING_FIELD_AND_STALE_REF_ISSUES: usize = 2;

    #[test]
    fn policy_freshness_accepts_required_fields_and_matching_export_ref() {
        let issues = validate_policy_freshness(&PolicyFieldSet {
            generated_fields: &[
                POLICY_SCHEMA_COMPATIBILITY_FIELD,
                TRACEABILITY_FIELD,
                STACK_PROVENANCE_FIELD,
                RUNTIME_EVIDENCE_FIELD,
            ],
            required_schema_fields: &[
                POLICY_SCHEMA_COMPATIBILITY_FIELD,
                TRACEABILITY_FIELD,
                STACK_PROVENANCE_FIELD,
                RUNTIME_EVIDENCE_FIELD,
            ],
            source_export_ref: POLICY_REF,
            checked_export_ref: POLICY_REF,
        });

        assert!(issues.is_empty());
    }

    #[test]
    fn policy_freshness_reports_missing_schema_field_and_stale_export_ref() {
        let issues = validate_policy_freshness(&PolicyFieldSet {
            generated_fields: &[STACK_PROVENANCE_FIELD, RUNTIME_EVIDENCE_FIELD],
            required_schema_fields: &[TRACEABILITY_FIELD, STACK_PROVENANCE_FIELD, RUNTIME_EVIDENCE_FIELD],
            source_export_ref: POLICY_REF,
            checked_export_ref: STALE_POLICY_REF,
        });

        assert_eq!(issues.len(), MISSING_FIELD_AND_STALE_REF_ISSUES);
        assert_eq!(issues[0].field, TRACEABILITY_FIELD);
        assert_eq!(issues[1].field, "checked_export_ref");
    }
}
