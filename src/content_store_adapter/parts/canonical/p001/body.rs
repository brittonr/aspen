
fn issue_code(issue: &ContentIssue) -> &'static str {
    match issue {
        ContentIssue::SchemaMismatch(_) => "schema-mismatch",
        ContentIssue::EmptyField(_) => "empty-field",
        ContentIssue::MalformedToken(_) => "malformed-token",
        ContentIssue::MalformedRef(_) => "malformed-ref",
        ContentIssue::ZeroBound(_) => "zero-bound",
        ContentIssue::DuplicateValue(_) => "duplicate-value",
        ContentIssue::MissingNonClaim(_) => "missing-non-claim",
        ContentIssue::ProfileMismatch => "profile-mismatch",
        ContentIssue::AdapterMismatch => "adapter-mismatch",
        ContentIssue::ManifestMismatch => "manifest-mismatch",
        ContentIssue::UnsupportedCapability => "unsupported-capability",
        ContentIssue::UnsupportedTransform(_) => "unsupported-transform",
        ContentIssue::TotalBytesExceeded => "total-bytes-exceeded",
        ContentIssue::ChunkCountExceeded => "chunk-count-exceeded",
        ContentIssue::ChunkBytesExceeded => "chunk-bytes-exceeded",
        ContentIssue::RangeExceeded => "range-exceeded",
        ContentIssue::ConcurrencyExceeded => "concurrency-exceeded",
        ContentIssue::QueueExceeded => "queue-exceeded",
        ContentIssue::MemoryExceeded => "memory-exceeded",
        ContentIssue::DeadlineExceeded => "deadline-exceeded",
        ContentIssue::RetryExceeded => "retry-exceeded",
        ContentIssue::EventLimitExceeded => "event-limit-exceeded",
        ContentIssue::ArithmeticOverflow => "arithmetic-overflow",
        ContentIssue::Cancelled => "cancelled",
        ContentIssue::PartialStateMismatch => "partial-state-mismatch",
        ContentIssue::UnexpectedChunk(_) => "unexpected-chunk",
        ContentIssue::ReorderedChunk(_) => "reordered-chunk",
        ContentIssue::CorruptChunk(_) => "corrupt-chunk",
        ContentIssue::TruncatedChunk(_) => "truncated-chunk",
        ContentIssue::DuplicateChunk(_) => "duplicate-chunk",
        ContentIssue::BackendHintCannotReplaceIdentity => "backend-hint-cannot-replace-identity",
        ContentIssue::ProtectionCannotGrantAuthority => "protection-cannot-grant-authority",
    }
}

fn canonical_artifact<T>(artifact: T, value: preserves::IOValue) -> crate::error::Result<CanonicalContentArtifact<T>> {
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalContentArtifact {
        artifact,
        artifact_ref,
        value,
    })
}

fn require_valid(label: &str, issues: &[ContentIssue]) -> crate::error::Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!("{label} denied: {issues:?}")))
    }
}

fn range_value(range: Option<ContentRange>) -> preserves::IOValue {
    match range {
        Some(range) => record("some", vec![record("content-range", vec![
            field("offset", u64_value(range.offset)),
            field("length", u64_value(range.length)),
        ])]),
        None => record("none", Vec::new()),
    }
}

fn optional_failure(failure: Option<ContentFailure>) -> preserves::IOValue {
    match failure {
        Some(failure) => record("some", vec![string(failure.as_str())]),
        None => record("none", Vec::new()),
    }
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

fn non_claims_value(values: &[ContentNonClaim]) -> preserves::IOValue {
    strings(values.iter().map(|value| value.as_str()))
}

fn issues_value(values: &[ContentIssue]) -> preserves::IOValue {
    strings(values.iter().map(issue_code))
}

fn checks(names: &[&str]) -> preserves::IOValue {
    field(
        "checks",
        sequence(names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect()),
    )
}

fn strings<'a>(values: impl Iterator<Item = &'a str>) -> preserves::IOValue {
    sequence(values.map(string).collect())
}

fn usize_value(value: usize) -> preserves::IOValue {
    match u64::try_from(value) {
        Ok(value) => u64_value(value),
        Err(_) => record("usize-overflow", Vec::new()),
    }
}

fn bool_value(value: bool) -> preserves::IOValue {
    crate::preserves_rail::bool_value(value)
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
