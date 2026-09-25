
fn aggregated_series_value(series: &AggregatedSeries) -> preserves::IOValue {
    record("aggregated-series", vec![
        field("descriptor-ref", string(&series.identity.descriptor_ref)),
        field("labels", labels_value(&series.identity.labels)),
        field("descriptor-id", string(&series.descriptor_id)),
        field("metric-name", string(&series.metric_name)),
        field("unit", string(&series.unit)),
        field("kind", string(series.kind.as_str())),
        field("aggregation", string(series.aggregation.as_str())),
        field("value", i64_value(series.value)),
        field("source-sample-refs", strings_value(series.source_sample_refs.iter().map(String::as_str))),
        field("latest-observed-tick", u64_value(series.latest_observed_tick)),
    ])
}

fn non_claims_value(non_claims: &[ObservabilityNonClaim]) -> preserves::IOValue {
    strings_value(non_claims.iter().map(|claim| claim.as_str()))
}

fn issues_value(issues: &[ObservabilityIssue]) -> preserves::IOValue {
    strings_value(issues.iter().map(issue_code))
}

fn issue_code(issue: &ObservabilityIssue) -> &'static str {
    match issue {
        ObservabilityIssue::SchemaMismatch(_) => "schema-mismatch",
        ObservabilityIssue::EmptyField(_) => "empty-field",
        ObservabilityIssue::MalformedToken(_) => "malformed-token",
        ObservabilityIssue::MalformedRef(_) => "malformed-ref",
        ObservabilityIssue::ZeroBound(_) => "zero-bound",
        ObservabilityIssue::CollectionLimitExceeded(_) => "collection-limit-exceeded",
        ObservabilityIssue::DuplicateValue(_) => "duplicate-value",
        ObservabilityIssue::MissingNonClaim(_) => "missing-non-claim",
        ObservabilityIssue::ProfileMismatch => "profile-mismatch",
        ObservabilityIssue::UnsupportedLabel(_) => "unsupported-label",
        ObservabilityIssue::LabelValueTooLarge(_) => "label-value-too-large",
        ObservabilityIssue::LabelRequiresRedaction(_) => "label-requires-redaction",
        ObservabilityIssue::RedactionRuleMissing(_) => "redaction-rule-missing",
        ObservabilityIssue::RedactionMarkerInvalid(_) => "redaction-marker-invalid",
        ObservabilityIssue::DescriptorMissing(_) => "descriptor-missing",
        ObservabilityIssue::DescriptorIncompatible => "descriptor-incompatible",
        ObservabilityIssue::CounterRequiresSum => "counter-requires-sum",
        ObservabilityIssue::ArithmeticOverflow => "arithmetic-overflow",
        ObservabilityIssue::ObservationStale(_) => "observation-stale",
        ObservabilityIssue::ObservationUnavailable(_) => "observation-unavailable",
        ObservabilityIssue::RequiredSourceMissing(_) => "required-source-missing",
        ObservabilityIssue::ClaimScopeOverreach => "claim-scope-overreach",
        ObservabilityIssue::MutationWithoutAuthority => "mutation-without-authority",
        ObservabilityIssue::PlanNotReadOnly => "plan-not-read-only",
        ObservabilityIssue::ScanTargetMissing(_) => "scan-target-missing",
        ObservabilityIssue::UnexpectedScanItem(_) => "unexpected-scan-item",
        ObservabilityIssue::ScanPlanMismatch => "scan-plan-mismatch",
        ObservabilityIssue::PartialScan => "partial-scan",
        ObservabilityIssue::FindingLimitExceeded => "finding-limit-exceeded",
        ObservabilityIssue::AdapterMismatch => "adapter-mismatch",
        ObservabilityIssue::ExportFrequencyExceeded => "export-frequency-exceeded",
        ObservabilityIssue::QueueBoundExceeded => "queue-bound-exceeded",
        ObservabilityIssue::DeadlineExceeded => "deadline-exceeded",
        ObservabilityIssue::ExporterUnavailable => "exporter-unavailable",
        ObservabilityIssue::ObservationDropped => "observation-dropped",
        ObservabilityIssue::Cancelled => "cancelled",
        ObservabilityIssue::PermissionDenied => "permission-denied",
        ObservabilityIssue::UnsupportedCapability => "unsupported-capability",
        ObservabilityIssue::CorruptInput => "corrupt-input",
        ObservabilityIssue::AdapterFailure => "adapter-failure",
        ObservabilityIssue::TelemetryCannotGrantAuthority => "telemetry-cannot-grant-authority",
    }
}

fn canonical_artifact<T>(artifact: T, value: preserves::IOValue) -> crate::error::Result<CanonicalArtifact<T>> {
    let artifact_ref = canonical_hash(&value)?;
    Ok(CanonicalArtifact {
        artifact,
        artifact_ref,
        value,
    })
}

fn require_valid(label: &str, issues: &[ObservabilityIssue]) -> crate::error::Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(validation_error(label, issues))
    }
}

fn validation_error(label: &str, issues: &[ObservabilityIssue]) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}

fn checks(names: &[&str]) -> preserves::IOValue {
    field(
        "checks",
        sequence(names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect()),
    )
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    sequence(values.map(string).collect())
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn usize_value(value: usize) -> preserves::IOValue {
    match u64::try_from(value) {
        Ok(value) => u64_value(value),
        Err(_) => record("usize-overflow", Vec::new()),
    }
}

fn i64_value(value: i64) -> preserves::IOValue {
    preserves::IOValue::new(value)
}

fn bool_value(value: bool) -> preserves::IOValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &preserves::IOValue) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    record(label, vec![value])
}

fn record(label: &'static str, fields: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<preserves::IOValue>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> preserves::IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn u64_value(value: u64) -> preserves::IOValue {
    crate::preserves_rail::u64_value(value)
}
