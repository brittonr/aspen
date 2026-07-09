#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservesBoundaryRow<'a> {
    pub artifact_family: &'a str,
    pub schema_label: &'a str,
    pub canonical_bytes_required: bool,
    pub blake3_identity_field: &'a str,
    pub adapter_owner: &'a str,
    pub core_dto: &'a str,
    pub allowed_consumers: &'a [&'a str],
    pub non_claims: &'a [&'a str],
    pub adapter_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreservesArtifactMeasurement<'a> {
    pub artifact_family: &'a str,
    pub schema_label: Option<&'a str>,
    pub canonical_bytes: bool,
    pub blake3_ref: &'a str,
    pub consumer: &'a str,
    pub raw_preserves_core_coupling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreservesBoundaryIssue {
    MissingSchemaLabel(String),
    UnknownProfileSurface(String),
    NonCanonicalBytes(String),
    StaleBlake3Ref(String),
    UnsupportedConsumer { family: String, consumer: String },
    MissingBoundaryNonClaim(String),
    RawPreservesCoreCoupling(String),
}

const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_CHAR_COUNT: usize = 64;
const BLAKE3_REF_CHAR_COUNT: usize = BLAKE3_REF_PREFIX.len() + BLAKE3_HEX_CHAR_COUNT;
const BOUNDARY_IDENTITY_NON_CLAIM: &str = "canonical-boundary-identity-only";

pub fn validate_preserves_boundary_profile(
    rows: &[PreservesBoundaryRow<'_>],
    measurements: &[PreservesArtifactMeasurement<'_>],
) -> Vec<PreservesBoundaryIssue> {
    let mut issues = Vec::new();
    for measurement in measurements {
        let Some(schema_label) = measurement.schema_label else {
            issues.push(PreservesBoundaryIssue::MissingSchemaLabel(measurement.artifact_family.to_string()));
            continue;
        };
        let Some(row) = rows
            .iter()
            .find(|row| row.artifact_family == measurement.artifact_family && row.schema_label == schema_label)
        else {
            issues.push(PreservesBoundaryIssue::UnknownProfileSurface(format!(
                "{}:{schema_label}",
                measurement.artifact_family
            )));
            continue;
        };
        validate_measurement(row, measurement, &mut issues);
    }
    issues
}

fn validate_measurement(
    row: &PreservesBoundaryRow<'_>,
    measurement: &PreservesArtifactMeasurement<'_>,
    issues: &mut Vec<PreservesBoundaryIssue>,
) {
    if row.canonical_bytes_required && !measurement.canonical_bytes {
        issues.push(PreservesBoundaryIssue::NonCanonicalBytes(measurement.artifact_family.to_string()));
    }
    if !valid_blake3_ref(measurement.blake3_ref) {
        issues.push(PreservesBoundaryIssue::StaleBlake3Ref(measurement.artifact_family.to_string()));
    }
    if !row.allowed_consumers.contains(&measurement.consumer) {
        issues.push(PreservesBoundaryIssue::UnsupportedConsumer {
            family: measurement.artifact_family.to_string(),
            consumer: measurement.consumer.to_string(),
        });
    }
    if !row.non_claims.iter().any(|claim| claim.contains(BOUNDARY_IDENTITY_NON_CLAIM)) {
        issues.push(PreservesBoundaryIssue::MissingBoundaryNonClaim(measurement.artifact_family.to_string()));
    }
    if row.adapter_only && measurement.raw_preserves_core_coupling {
        issues.push(PreservesBoundaryIssue::RawPreservesCoreCoupling(measurement.artifact_family.to_string()));
    }
}

fn valid_blake3_ref(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return false;
    };
    value.len() == BLAKE3_REF_CHAR_COUNT && hex.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE_CONTROL: &str = "node-control-envelope";
    const TICKET: &str = "ticket";
    const WORKFLOW_BUNDLE: &str = "workflow-bundle";
    const RECEIPT: &str = "receipt";
    const EVIDENCE_ENVELOPE: &str = "evidence-envelope";
    const NODE_SCHEMA: &str = "molten.node-control.envelope.v1";
    const TICKET_SCHEMA: &str = "molten.node-control.ticket.v1";
    const WORKFLOW_SCHEMA: &str = "molten.node-control.workflow-bundle.v1";
    const RECEIPT_SCHEMA: &str = "molten.receipt.v1";
    const EVIDENCE_SCHEMA: &str = "molten.stack-evidence.envelope.v1";
    const IDENTITY_FIELD: &str = "artifact_ref";
    const ADAPTER_OWNER: &str = "preserves-adapter";
    const CORE_DTO: &str = "typed-boundary-dto";
    const RUNTIME_CONSUMER: &str = "runtime";
    const EVIDENCE_CONSUMER: &str = "evidence";
    const NON_CLAIM: &str = "canonical-boundary-identity-only: does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance";
    const VALID_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const STALE_REF: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn row(artifact_family: &'static str, schema_label: &'static str) -> PreservesBoundaryRow<'static> {
        PreservesBoundaryRow {
            artifact_family,
            schema_label,
            canonical_bytes_required: true,
            blake3_identity_field: IDENTITY_FIELD,
            adapter_owner: ADAPTER_OWNER,
            core_dto: CORE_DTO,
            allowed_consumers: &[RUNTIME_CONSUMER, EVIDENCE_CONSUMER],
            non_claims: &[NON_CLAIM],
            adapter_only: true,
        }
    }

    fn rows() -> [PreservesBoundaryRow<'static>; PROFILE_ROW_COUNT] {
        [
            row(NODE_CONTROL, NODE_SCHEMA),
            row(TICKET, TICKET_SCHEMA),
            row(WORKFLOW_BUNDLE, WORKFLOW_SCHEMA),
            row(RECEIPT, RECEIPT_SCHEMA),
            row(EVIDENCE_ENVELOPE, EVIDENCE_SCHEMA),
        ]
    }

    fn measurement(artifact_family: &'static str, schema_label: &'static str) -> PreservesArtifactMeasurement<'static> {
        PreservesArtifactMeasurement {
            artifact_family,
            schema_label: Some(schema_label),
            canonical_bytes: true,
            blake3_ref: VALID_REF,
            consumer: RUNTIME_CONSUMER,
            raw_preserves_core_coupling: false,
        }
    }

    const PROFILE_ROW_COUNT: usize = 5;

    #[test]
    fn preserves_profile_accepts_canonical_boundary_artifacts() {
        let rows = rows();
        let measurements = [
            measurement(NODE_CONTROL, NODE_SCHEMA),
            measurement(TICKET, TICKET_SCHEMA),
            measurement(WORKFLOW_BUNDLE, WORKFLOW_SCHEMA),
            measurement(RECEIPT, RECEIPT_SCHEMA),
            measurement(EVIDENCE_ENVELOPE, EVIDENCE_SCHEMA),
        ];

        assert!(validate_preserves_boundary_profile(&rows, &measurements).is_empty());
    }

    #[test]
    fn preserves_profile_rejects_non_canonical_bytes_missing_schema_stale_refs_and_raw_core_coupling() {
        let rows = rows();
        let measurements = [
            PreservesArtifactMeasurement {
                canonical_bytes: false,
                ..measurement(NODE_CONTROL, NODE_SCHEMA)
            },
            PreservesArtifactMeasurement {
                schema_label: None,
                ..measurement(TICKET, TICKET_SCHEMA)
            },
            PreservesArtifactMeasurement {
                blake3_ref: STALE_REF,
                ..measurement(WORKFLOW_BUNDLE, WORKFLOW_SCHEMA)
            },
            PreservesArtifactMeasurement {
                raw_preserves_core_coupling: true,
                ..measurement(RECEIPT, RECEIPT_SCHEMA)
            },
        ];

        let issues = validate_preserves_boundary_profile(&rows, &measurements);

        assert!(issues.contains(&PreservesBoundaryIssue::NonCanonicalBytes(NODE_CONTROL.to_string())));
        assert!(issues.contains(&PreservesBoundaryIssue::MissingSchemaLabel(TICKET.to_string())));
        assert!(issues.contains(&PreservesBoundaryIssue::StaleBlake3Ref(WORKFLOW_BUNDLE.to_string())));
        assert!(issues.contains(&PreservesBoundaryIssue::RawPreservesCoreCoupling(RECEIPT.to_string())));
    }

    #[test]
    fn preserves_profile_rejects_unsupported_consumers_and_missing_non_claims() {
        let rows = [PreservesBoundaryRow {
            non_claims: &[],
            ..row(NODE_CONTROL, NODE_SCHEMA)
        }];
        let measurements = [PreservesArtifactMeasurement {
            consumer: "ambient-debugger",
            ..measurement(NODE_CONTROL, NODE_SCHEMA)
        }];

        let issues = validate_preserves_boundary_profile(&rows, &measurements);

        assert!(issues.contains(&PreservesBoundaryIssue::UnsupportedConsumer {
            family: NODE_CONTROL.to_string(),
            consumer: "ambient-debugger".to_string(),
        }));
        assert!(issues.contains(&PreservesBoundaryIssue::MissingBoundaryNonClaim(NODE_CONTROL.to_string())));
    }
}
