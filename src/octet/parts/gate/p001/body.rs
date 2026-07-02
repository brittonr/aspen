
const PASS_CHECKS: &[Check] = &[
    Check {
        name: "artifacts-dir-present",
        status: "pass",
    },
    Check {
        name: "profile-supported",
        status: "pass",
    },
    Check {
        name: "command-artifact-present",
        status: "pass",
    },
    Check {
        name: "status-artifact-present",
        status: "pass",
    },
    Check {
        name: "summary-artifact-present",
        status: "pass",
    },
    Check {
        name: "object-corpus-artifact-present",
        status: "pass",
    },
    Check {
        name: "status-json-parse",
        status: "pass",
    },
    Check {
        name: "status-metadata-complete",
        status: "pass",
    },
    Check {
        name: "status-tool-version-supported",
        status: "pass",
    },
    Check {
        name: "status-exit-consistent",
        status: "pass",
    },
    Check {
        name: "summary-lints-parse",
        status: "pass",
    },
    Check {
        name: "object-corpus-json-parse",
        status: "pass",
    },
    Check {
        name: "object-corpus-schema",
        status: "pass",
    },
    Check {
        name: "object-corpus-nonempty",
        status: "pass",
    },
    Check {
        name: "object-corpus-fingerprint",
        status: "pass",
    },
    Check {
        name: "object-corpus-critical-paths",
        status: "pass",
    },
    Check {
        name: SOURCE_SCOPE_OBJECT_CORPUS_CHECK,
        status: "pass",
    },
    Check {
        name: "command-shape",
        status: "pass",
    },
    Check {
        name: "status-config-current",
        status: "pass",
    },
    Check {
        name: "status-profile-current",
        status: "pass",
    },
    Check {
        name: "strict-status-clean",
        status: "pass",
    },
    Check {
        name: "no-critical-findings",
        status: "pass",
    },
    Check {
        name: "artifact-ref-binding",
        status: "pass",
    },
    Check {
        name: "structured-findings-bound",
        status: "pass",
    },
    Check {
        name: "structured-findings-keyed",
        status: "pass",
    },
    Check {
        name: "fingerprint-evidence-bound",
        status: "pass",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateFile {
    artifact_ref: String,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputFiles {
    command: Option<GateFile>,
    status_file: Option<GateFile>,
    summary: Option<GateFile>,
    object_corpus: Option<GateFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DerivedEvidence {
    structured_findings_ref: Option<String>,
    structured_unkeyed: u64,
    fingerprint_evidence_ref: Option<String>,
}

struct EvidenceInput<'a> {
    status: Option<&'a StatusArtifact>,
    status_file: Option<&'a GateFile>,
    summary: Option<&'a GateFile>,
    object_corpus_receipt: Option<&'a ObjectCorpusReceipt>,
    object_corpus: Option<&'a GateFile>,
}

struct OutcomeFacts<'a> {
    status: Option<&'a StatusArtifact>,
    counts: &'a FindingCounts,
    has_artifact_bindings: bool,
    has_structured_findings_ref: bool,
    structured_unkeyed: u64,
    has_fingerprint_evidence_ref: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FindingCounts {
    total: u64,
    warnings: u64,
    errors: u64,
    autofixable: u64,
    critical: u64,
    uncovered: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FindingEntry {
    key: String,
    lint: String,
    crate_name: String,
    location: String,
    count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentOctetRun {
    status_ref: String,
    summary_ref: String,
    object_corpus_ref: String,
    status: StatusArtifact,
    findings: OrderedMap<String, FindingEntry>,
    unkeyed_findings: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedWarningBaseline {
    baseline_ref: String,
    expires_at: String,
    config_hash: String,
    profile_hash: String,
    findings: OrderedMap<String, FindingEntry>,
    allowed_profiles: Vec<String>,
    target_next: u64,
    review_refs: Vec<String>,
}

struct BaselineFacts {
    has_bound_review_refs: bool,
    is_profile_allowed: bool,
    is_baseline_current: bool,
    is_config_current: bool,
    is_profile_hash_current: bool,
    is_within_shrink_target: bool,
    has_zero_unkeyed_findings: bool,
}

struct DiagnosticInput<'a> {
    input: &'a OctetBaselineCheckInput,
    baseline: &'a ParsedWarningBaseline,
    run: &'a CurrentOctetRun,
    facts: &'a BaselineFacts,
    new_findings: &'a [FindingEntry],
    critical_unreviewed: &'a [FindingEntry],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedReviewManifest {
    review_ref: String,
    profile: String,
    expires_at: String,
    finding_keys: Vec<String>,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct StatusArtifact {
    status: String,
    exit_code: i64,
    metadata: Metadata,
    total_findings: u64,
    warning_findings: u64,
    error_findings: u64,
    autofixable_findings: u64,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct ObjectCorpusReceipt {
    schema: Option<String>,
    schema_version: Option<u64>,
    object_count: Option<u64>,
    source_paths: Option<Vec<String>>,
    object_set_hash: Option<String>,
    pure_cache_blocked_count: Option<u64>,
    replay: Option<ObjectCorpusReplay>,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct ObjectCorpusReplay {
    command: Option<String>,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct Metadata {
    tool_name: String,
    tool_version: String,
    rustc_version: String,
    toolchain: String,
    profile_hash: String,
    config_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceOctetConfig {
    default_scope: Vec<String>,
    cargo_check_args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EffectiveOctetCommand {
    scope_args: Vec<String>,
    cargo_check_args: Vec<String>,
    output_format: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedMetadata {
    config_hash: String,
    profile_hash: String,
}
