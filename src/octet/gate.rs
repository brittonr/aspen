use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;
use preserves::Value;
use serde::Deserialize;

use crate::error::MoltenError;
use crate::error::Result;
use crate::ledger;
use crate::preserves_rail::OCTET_COMMAND_ARTIFACT_SCHEMA;
use crate::preserves_rail::OCTET_FINGERPRINT_EVIDENCE_SCHEMA;
use crate::preserves_rail::OCTET_GATE_RECEIPT_SCHEMA;
use crate::preserves_rail::OCTET_OBJECT_CORPUS_ARTIFACT_SCHEMA;
use crate::preserves_rail::OCTET_REVIEW_MANIFEST_SCHEMA;
use crate::preserves_rail::OCTET_SOURCE_GATE_REQUIREMENT_SCHEMA;
use crate::preserves_rail::OCTET_SOURCE_GATE_VALIDATION_SCHEMA;
use crate::preserves_rail::OCTET_STATUS_ARTIFACT_SCHEMA;
use crate::preserves_rail::OCTET_SUMMARY_ARTIFACT_SCHEMA;
use crate::preserves_rail::OCTET_WARNING_BASELINE_SCHEMA;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::content_ref_hex;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;

const STRICT_PROFILE: &str = "strict-ci";
const QUARANTINE_PROFILE: &str = "quarantine-ci";
const OBJECT_CORPUS_RECEIPT_NAME: &str = "object-corpus-receipt.json";
const COMMAND_NAME: &str = "command.txt";
const STATUS_NAME: &str = "status.json";
const SUMMARY_NAME: &str = "summary.txt";
const OCTET_OBJECT_CORPUS_SCHEMA: &str = "octet.function-object-corpus-receipt.v1";
const MAX_DIAGNOSTICS: usize = 64;
const MAX_OCTET_ARTIFACT_VALUES: usize = 8;
const MAX_OCTET_IMPORTED_REFS: usize = MAX_OCTET_ARTIFACT_VALUES;
const MAX_OCTET_SUMMARY_LINTS: usize = 128;
const MAX_OCTET_COMMAND_TOKENS: usize = 256;
const MAX_OCTET_FINDING_ENTRIES: usize = 100_000;
const MAX_OCTET_STRING_SEQUENCE: usize = 100_000;
const SUPPORTED_OCTET_TOOL_VERSION: &str = "0.1.0";

const _: () = assert!(MAX_OCTET_IMPORTED_REFS <= MAX_OCTET_ARTIFACT_VALUES);
const _: () = assert!(MAX_OCTET_SUMMARY_LINTS > 0);
const _: () = assert!(MAX_OCTET_COMMAND_TOKENS > 0);
const REQUIRED_OBJECT_CORPUS_SOURCE_PATHS: &[&str] = &["src/job/dag.rs", "src/main.rs", "src/node/runtime.rs"];
const DEFAULT_GATE_COMMAND: &str = "cargo octet check --artifact-dir target/octet";
const SOURCE_GATE_CONSUMERS: &[&str] = &[
    "node-startup",
    "job-remote-admission",
    "upgrade-plan",
    "node-control-gate",
];

const CRITICAL_LINTS: &[&str] = &[
    "no_panic",
    "no_unwrap",
    "ambient_clock",
    "unbounded_loop",
    "unbounded_collection_growth",
    "secret_rendering",
    "harness_backdoor",
    "authority_typing",
    "adapter_boundary_gate",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetGateInput {
    pub artifacts_dir: PathBuf,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetGateEvaluation {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetWarningBaselineInput {
    pub artifacts_dir: PathBuf,
    pub created_at: String,
    pub expires_at: String,
    pub target_next: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetWarningBaselineArtifact {
    pub baseline_ref: String,
    pub baseline_value: IOValue,
    pub finding_count: u64,
    pub critical_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetBaselineCheckInput {
    pub artifacts_dir: PathBuf,
    pub baseline_value: IOValue,
    pub profile: String,
    pub as_of: String,
    pub review_values: Vec<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetBaselineEvaluation {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetReviewManifestInput {
    pub profile: String,
    pub expires_at: String,
    pub finding_keys: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetReviewManifestArtifact {
    pub review_ref: String,
    pub review_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetArtifactLedgerInput {
    pub artifacts_dir: PathBuf,
    pub ledger_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetArtifactLedgerImport {
    pub decision: String,
    pub imported_refs: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetSourceGateValidationInput {
    pub consumer: String,
    pub subject_ref: String,
    pub gate_receipt_value: Option<IOValue>,
    pub source_scope: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetSourceGateValidation {
    pub decision: String,
    pub requirement_ref: String,
    pub validation_ref: String,
    pub gate_receipt_ref: Option<String>,
    pub value: IOValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedOctetGateReceipt {
    receipt_ref: String,
    decision: String,
    policy_ref: String,
    command_ref: Option<String>,
    status_ref: Option<String>,
    summary_ref: Option<String>,
    findings_ref: Option<String>,
    object_corpus_ref: Option<String>,
    fingerprint_ref: Option<String>,
    config_hash: Option<String>,
    profile_hash: Option<String>,
    toolchain: Option<String>,
    counts: FindingCounts,
    diagnostics: Vec<String>,
    checks: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ValidationRefs {
    counts: FindingCounts,
    receipt_ref: Option<String>,
    policy_ref: Option<String>,
    status_ref: Option<String>,
    summary_ref: Option<String>,
    findings_ref: Option<String>,
    object_corpus_ref: Option<String>,
    fingerprint_ref: Option<String>,
}

struct ReceiptCheckInput<'a> {
    parsed: Option<&'a ParsedOctetGateReceipt>,
    expected: Option<&'a ExpectedMetadata>,
}

struct SourceSetup {
    source_scope: Vec<String>,
    expected: Option<ExpectedMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateCheck {
    name: &'static str,
    status: &'static str,
}

const PASS_CHECKS: &[GateCheck] = &[
    GateCheck {
        name: "artifacts-dir-present",
        status: "pass",
    },
    GateCheck {
        name: "profile-supported",
        status: "pass",
    },
    GateCheck {
        name: "command-artifact-present",
        status: "pass",
    },
    GateCheck {
        name: "status-artifact-present",
        status: "pass",
    },
    GateCheck {
        name: "summary-artifact-present",
        status: "pass",
    },
    GateCheck {
        name: "object-corpus-artifact-present",
        status: "pass",
    },
    GateCheck {
        name: "status-json-parse",
        status: "pass",
    },
    GateCheck {
        name: "status-metadata-complete",
        status: "pass",
    },
    GateCheck {
        name: "status-tool-version-supported",
        status: "pass",
    },
    GateCheck {
        name: "status-exit-consistent",
        status: "pass",
    },
    GateCheck {
        name: "summary-lints-parse",
        status: "pass",
    },
    GateCheck {
        name: "object-corpus-json-parse",
        status: "pass",
    },
    GateCheck {
        name: "object-corpus-schema",
        status: "pass",
    },
    GateCheck {
        name: "object-corpus-nonempty",
        status: "pass",
    },
    GateCheck {
        name: "object-corpus-fingerprint",
        status: "pass",
    },
    GateCheck {
        name: "object-corpus-critical-paths",
        status: "pass",
    },
    GateCheck {
        name: "command-shape",
        status: "pass",
    },
    GateCheck {
        name: "status-config-current",
        status: "pass",
    },
    GateCheck {
        name: "status-profile-current",
        status: "pass",
    },
    GateCheck {
        name: "strict-status-clean",
        status: "pass",
    },
    GateCheck {
        name: "no-critical-findings",
        status: "pass",
    },
    GateCheck {
        name: "artifact-ref-binding",
        status: "pass",
    },
    GateCheck {
        name: "structured-findings-bound",
        status: "pass",
    },
    GateCheck {
        name: "structured-findings-keyed",
        status: "pass",
    },
    GateCheck {
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
    status: Option<&'a OctetStatusArtifact>,
    status_file: Option<&'a GateFile>,
    summary: Option<&'a GateFile>,
    object_corpus_receipt: Option<&'a OctetObjectCorpusReceipt>,
    object_corpus: Option<&'a GateFile>,
}

struct OutcomeFacts<'a> {
    status: Option<&'a OctetStatusArtifact>,
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
    status: OctetStatusArtifact,
    findings: BTreeMap<String, FindingEntry>,
    unkeyed_findings: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedWarningBaseline {
    baseline_ref: String,
    expires_at: String,
    config_hash: String,
    profile_hash: String,
    findings: BTreeMap<String, FindingEntry>,
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

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct OctetStatusArtifact {
    status: String,
    exit_code: i64,
    metadata: OctetMetadata,
    total_findings: u64,
    warning_findings: u64,
    error_findings: u64,
    autofixable_findings: u64,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct OctetObjectCorpusReceipt {
    schema: Option<String>,
    schema_version: Option<u64>,
    object_count: Option<u64>,
    source_paths: Option<Vec<String>>,
    object_set_hash: Option<String>,
    pure_cache_blocked_count: Option<u64>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct OctetMetadata {
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

pub fn evaluate_octet_gate(input: &OctetGateInput) -> Result<OctetGateEvaluation> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();

    push_initial_checks(input, &mut checks, &mut diagnostics);
    let files = read_required_inputs(&input.artifacts_dir, &mut checks, &mut diagnostics);
    let status = parse_status(files.status_file.as_ref(), &mut checks, &mut diagnostics);
    let lint_counts = parse_summary_lints(files.summary.as_ref(), &mut checks, &mut diagnostics);
    let object_corpus_receipt = validate_object_corpus(files.object_corpus.as_ref(), &mut checks, &mut diagnostics);
    let has_valid_object_corpus = object_corpus_receipt.is_some();
    let has_valid_command_shape = validate_command(files.command.as_ref(), &mut checks, &mut diagnostics);
    let has_current_metadata_binding =
        validate_metadata_binding(files.command.as_ref(), status.as_ref(), &mut checks, &mut diagnostics);
    let counts = finding_counts(status.as_ref(), &lint_counts);
    let evidence = derive_evidence(EvidenceInput {
        status: status.as_ref(),
        status_file: files.status_file.as_ref(),
        summary: files.summary.as_ref(),
        object_corpus_receipt: object_corpus_receipt.as_ref(),
        object_corpus: files.object_corpus.as_ref(),
    })?;
    let has_artifact_bindings = files.command.is_some()
        && files.status_file.is_some()
        && files.summary.is_some()
        && files.object_corpus.is_some();

    push_outcome_checks(
        OutcomeFacts {
            status: status.as_ref(),
            counts: &counts,
            has_artifact_bindings,
            has_structured_findings_ref: evidence.structured_findings_ref.is_some(),
            structured_unkeyed: evidence.structured_unkeyed,
            has_fingerprint_evidence_ref: evidence.fingerprint_evidence_ref.is_some(),
        },
        &mut checks,
        &mut diagnostics,
    );

    let has_passing_gate_checks = checks.iter().all(|check| check.status == "pass")
        && has_valid_object_corpus
        && has_valid_command_shape
        && has_current_metadata_binding;
    let decision = if has_passing_gate_checks { "pass" } else { "deny" }.to_string();
    let policy_value = octet_gate_policy_value(input);
    let policy_ref = canonical_hash(&policy_value)?;
    let receipt_value = octet_gate_receipt_value(OctetGateReceiptInput {
        decision: &decision,
        policy_ref: &policy_ref,
        command_ref: files.command.as_ref().map(|file| file.artifact_ref.as_str()),
        status_ref: files.status_file.as_ref().map(|file| file.artifact_ref.as_str()),
        summary_ref: files.summary.as_ref().map(|file| file.artifact_ref.as_str()),
        structured_findings_ref: evidence.structured_findings_ref.as_deref(),
        object_corpus_ref: files.object_corpus.as_ref().map(|file| file.artifact_ref.as_str()),
        fingerprint_evidence_ref: evidence.fingerprint_evidence_ref.as_deref(),
        config_hash: status.as_ref().map(|status| status.metadata.config_hash.as_str()),
        profile_hash: status.as_ref().map(|status| status.metadata.profile_hash.as_str()),
        toolchain: status.as_ref().map(|status| status.metadata.toolchain.as_str()),
        counts: &counts,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(OctetGateEvaluation {
        decision,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn push_initial_checks(
    input: &OctetGateInput,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let has_artifacts_dir = input.artifacts_dir.is_dir();
    push_check(checks, "artifacts-dir-present", has_artifacts_dir);
    if !has_artifacts_dir {
        push_diagnostic(diagnostics, format!("artifacts directory missing: {}", input.artifacts_dir.display()));
    }

    let is_profile_supported = input.profile == STRICT_PROFILE;
    push_check(checks, "profile-supported", is_profile_supported);
    if !is_profile_supported {
        push_diagnostic(diagnostics, format!("unsupported octet gate profile: {}", input.profile));
    }
}

fn read_required_inputs(
    artifacts_dir: &Path,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> InputFiles {
    InputFiles {
        command: read_required_file(artifacts_dir, COMMAND_NAME, "command-artifact-present", checks, diagnostics),
        status_file: read_required_file(artifacts_dir, STATUS_NAME, "status-artifact-present", checks, diagnostics),
        summary: read_required_file(artifacts_dir, SUMMARY_NAME, "summary-artifact-present", checks, diagnostics),
        object_corpus: read_required_file(
            artifacts_dir,
            OBJECT_CORPUS_RECEIPT_NAME,
            "object-corpus-artifact-present",
            checks,
            diagnostics,
        ),
    }
}

fn derive_evidence(input: EvidenceInput<'_>) -> Result<DerivedEvidence> {
    let structured_findings = input
        .status
        .zip(input.status_file)
        .zip(input.summary)
        .map(|((status, status_file), summary)| octet_structured_findings_value(status_file, summary, status));
    let structured_findings_ref =
        structured_findings.as_ref().map(|(value, _unkeyed)| canonical_hash(value)).transpose()?;
    let structured_unkeyed = structured_findings
        .as_ref()
        .map(|(_value, unkeyed)| *unkeyed)
        .unwrap_or_else(|| input.status.map_or(0, |status| status.total_findings));
    let fingerprint_evidence = input
        .object_corpus_receipt
        .zip(input.object_corpus)
        .map(|(receipt, file)| octet_fingerprint_evidence_value(file, receipt))
        .transpose()?;
    let fingerprint_evidence_ref = fingerprint_evidence.as_ref().map(canonical_hash).transpose()?;
    Ok(DerivedEvidence {
        structured_findings_ref,
        structured_unkeyed,
        fingerprint_evidence_ref,
    })
}

fn push_outcome_checks(
    facts: OutcomeFacts<'_>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let is_strict_status_clean = facts.status.is_some_and(|status| status.status == "clean");
    push_check(checks, "strict-status-clean", is_strict_status_clean);
    let denied_status = match facts.status {
        Some(status) if status.status == "clean" => None,
        Some(status) => Some(status),
        None => None,
    };
    if let Some(status) = denied_status {
        push_diagnostic(
            diagnostics,
            format!("strict profile denies octet status `{}` with {} findings", status.status, status.total_findings),
        );
    }

    let has_zero_critical_findings = facts.counts.critical == 0;
    push_check(checks, "no-critical-findings", has_zero_critical_findings);
    if facts.counts.critical > 0 {
        push_diagnostic(diagnostics, format!("unreviewed critical octet findings: {}", facts.counts.critical));
    }
    push_check(checks, "artifact-ref-binding", facts.has_artifact_bindings);
    push_check(checks, "structured-findings-bound", facts.has_structured_findings_ref);
    push_check(
        checks,
        "structured-findings-keyed",
        facts.has_structured_findings_ref && facts.structured_unkeyed == 0,
    );
    push_check(checks, "fingerprint-evidence-bound", facts.has_fingerprint_evidence_ref);

    if !facts.has_structured_findings_ref {
        push_diagnostic(diagnostics, "missing structured octet findings artifact".to_string());
    }
    if facts.has_structured_findings_ref && facts.structured_unkeyed > 0 {
        push_diagnostic(
            diagnostics,
            format!("structured octet findings omitted stable keys for {} findings", facts.structured_unkeyed),
        );
    }
    if !facts.has_fingerprint_evidence_ref {
        push_diagnostic(diagnostics, "missing octet fingerprint evidence artifact".to_string());
    }
}

#[doc(hidden)]
pub fn synthetic_clean_octet_gate_receipt_for_tests() -> Result<IOValue> {
    let metadata = expected_metadata_for_command(DEFAULT_GATE_COMMAND)
        .map_err(|message| MoltenError::invalid_harness(format!("current octet metadata fixture: {message}")))?;
    let policy = octet_gate_policy_value(&OctetGateInput {
        artifacts_dir: PathBuf::from("target/octet"),
        profile: STRICT_PROFILE.to_string(),
    });
    let counts = FindingCounts::default();
    Ok(octet_gate_receipt_value(OctetGateReceiptInput {
        decision: "pass",
        policy_ref: &canonical_hash(&policy)?,
        command_ref: Some("blake3:test-command"),
        status_ref: Some("blake3:test-status"),
        summary_ref: Some("blake3:test-summary"),
        structured_findings_ref: Some("blake3:test-findings"),
        object_corpus_ref: Some("blake3:test-object-corpus"),
        fingerprint_evidence_ref: Some("blake3:test-fingerprint"),
        config_hash: Some(&metadata.config_hash),
        profile_hash: Some(&metadata.profile_hash),
        toolchain: Some("nightly-test-toolchain"),
        counts: &counts,
        diagnostics: &[],
        checks: PASS_CHECKS,
    }))
}

pub fn build_octet_warning_baseline(input: &OctetWarningBaselineInput) -> Result<OctetWarningBaselineArtifact> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let run = load_current_octet_run(&input.artifacts_dir, &mut checks, &mut diagnostics)?;
    if checks.iter().any(|check| check.status != "pass") {
        return Err(MoltenError::invalid_harness(format!(
            "cannot create octet warning baseline from invalid artifacts: {}",
            diagnostics.join("; ")
        )));
    }
    let target_next = input.target_next.unwrap_or(run.status.total_findings);
    let critical_keys = critical_keys(&run.findings);
    let source_snapshot_ref = source_snapshot_ref(&run)?;
    let baseline_value = octet_warning_baseline_value(&OctetWarningBaselineValueInput {
        run: &run,
        created_at: &input.created_at,
        expires_at: &input.expires_at,
        target_next,
        source_snapshot_ref: &source_snapshot_ref,
        checks: &checks,
    });
    let baseline_ref = canonical_hash(&baseline_value)?;
    Ok(OctetWarningBaselineArtifact {
        baseline_ref,
        baseline_value,
        finding_count: run.status.total_findings,
        critical_count: critical_keys.len() as u64,
    })
}

pub fn import_octet_artifacts_to_ledger(input: &OctetArtifactLedgerInput) -> Result<OctetArtifactLedgerImport> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let files = read_import_files(&input.artifacts_dir, &mut checks, &mut diagnostics);
    let mut values = raw_values(&files);
    add_structured_value(&mut values, &files, &mut checks, &mut diagnostics);
    add_fingerprint_value(&mut values, &files, &mut checks, &mut diagnostics)?;
    ensure_count_at_most(values.len(), MAX_OCTET_IMPORTED_REFS, "octet imported artifacts")?;
    let mut imported_refs = Vec::with_capacity(values.len());
    for value in &values {
        imported_refs.push(ledger::import_artifact(&input.ledger_root, value)?.artifact_ref);
    }
    push_check(&mut checks, "octet-ledger-imports", !imported_refs.is_empty());
    let decision = if checks.iter().all(|check| check.status == "pass") {
        "pass"
    } else {
        "deny"
    };
    let receipt_value = octet_artifact_ledger_receipt_value(
        decision,
        &input.artifacts_dir.to_string_lossy(),
        &imported_refs,
        &diagnostics,
        &checks,
    );
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(OctetArtifactLedgerImport {
        decision: decision.to_string(),
        imported_refs,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn read_import_files(
    artifacts_dir: &Path,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> InputFiles {
    InputFiles {
        command: read_required_file(
            artifacts_dir,
            COMMAND_NAME,
            "ledger-command-artifact-present",
            checks,
            diagnostics,
        ),
        status_file: read_required_file(
            artifacts_dir,
            STATUS_NAME,
            "ledger-status-artifact-present",
            checks,
            diagnostics,
        ),
        summary: read_required_file(
            artifacts_dir,
            SUMMARY_NAME,
            "ledger-summary-artifact-present",
            checks,
            diagnostics,
        ),
        object_corpus: read_required_file(
            artifacts_dir,
            OBJECT_CORPUS_RECEIPT_NAME,
            "ledger-object-corpus-artifact-present",
            checks,
            diagnostics,
        ),
    }
}

fn raw_values(files: &InputFiles) -> Vec<IOValue> {
    let mut values = Vec::with_capacity(MAX_OCTET_ARTIFACT_VALUES);
    if let Some(command) = files.command.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-command-artifact-v1",
            OCTET_COMMAND_ARTIFACT_SCHEMA,
            COMMAND_NAME,
            command,
        ));
    }
    if let Some(status_file) = files.status_file.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-status-artifact-v1",
            OCTET_STATUS_ARTIFACT_SCHEMA,
            STATUS_NAME,
            status_file,
        ));
    }
    if let Some(summary) = files.summary.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-summary-artifact-v1",
            OCTET_SUMMARY_ARTIFACT_SCHEMA,
            SUMMARY_NAME,
            summary,
        ));
    }
    if let Some(object_corpus) = files.object_corpus.as_ref() {
        values.push(octet_raw_artifact_value(
            "octet-object-corpus-artifact-v1",
            OCTET_OBJECT_CORPUS_ARTIFACT_SCHEMA,
            OBJECT_CORPUS_RECEIPT_NAME,
            object_corpus,
        ));
    }
    values
}

fn add_structured_value(
    values: &mut impl crate::bounded::VecSink<IOValue>,
    files: &InputFiles,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let status = parse_status(files.status_file.as_ref(), checks, diagnostics);
    if let Some((status, status_file, summary)) = status
        .as_ref()
        .zip(files.status_file.as_ref())
        .zip(files.summary.as_ref())
        .map(|((status, status_file), summary)| (status, status_file, summary))
    {
        let (structured, unkeyed) = octet_structured_findings_value(status_file, summary, status);
        if unkeyed == 0 {
            values.push_item(structured);
        } else {
            push_diagnostic(diagnostics, format!("structured findings omitted stable keys for {unkeyed} findings"));
        }
    }
}

fn add_fingerprint_value(
    values: &mut impl crate::bounded::VecSink<IOValue>,
    files: &InputFiles,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if let Some((object_corpus_receipt, object_corpus)) =
        validate_object_corpus(files.object_corpus.as_ref(), checks, diagnostics)
            .as_ref()
            .zip(files.object_corpus.as_ref())
    {
        values.push_item(octet_fingerprint_evidence_value(object_corpus, object_corpus_receipt)?);
    }
    Ok(())
}

pub fn build_octet_review_manifest(input: &OctetReviewManifestInput) -> Result<OctetReviewManifestArtifact> {
    if input.finding_keys.is_empty() {
        return Err(MoltenError::invalid_harness("octet review manifest requires at least one finding key"));
    }
    let review_value = record("octet-review-manifest-v1", vec![
        string(OCTET_REVIEW_MANIFEST_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        record("expires-at", vec![string(&input.expires_at)]),
        record("finding-keys", vec![sequence(input.finding_keys.iter().map(string).collect())]),
        record("rationale", vec![string(&input.rationale)]),
        checks_value(&[
            GateCheck {
                name: "exact-finding-keys",
                status: "pass",
            },
            GateCheck {
                name: "temporary-review",
                status: "pass",
            },
        ]),
    ]);
    let review_ref = canonical_hash(&review_value)?;
    Ok(OctetReviewManifestArtifact {
        review_ref,
        review_value,
    })
}

pub fn check_octet_warning_baseline(input: &OctetBaselineCheckInput) -> Result<OctetBaselineEvaluation> {
    let baseline = parse_warning_baseline(&input.baseline_value)?;
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let run = load_current_octet_run(&input.artifacts_dir, &mut checks, &mut diagnostics)?;
    let reviews = parse_review_manifests(&input.review_values)?;
    let review_refs = reviews.iter().map(|review| review.review_ref.clone()).collect::<Vec<_>>();
    let facts = BaselineFacts {
        has_bound_review_refs: baseline.review_refs.is_empty()
            || baseline
                .review_refs
                .iter()
                .all(|baseline_ref| review_refs.iter().any(|review_ref| review_ref == baseline_ref)),
        is_profile_allowed: baseline.allowed_profiles.iter().any(|profile| profile == &input.profile),
        is_baseline_current: baseline.expires_at.as_str() >= input.as_of.as_str(),
        is_config_current: run.status.metadata.config_hash == baseline.config_hash,
        is_profile_hash_current: run.status.metadata.profile_hash == baseline.profile_hash,
        is_within_shrink_target: run.status.total_findings <= baseline.target_next,
        has_zero_unkeyed_findings: run.unkeyed_findings == 0,
    };
    let new_findings = finding_count_delta(&run.findings, &baseline.findings, DeltaKind::NewOrIncreased);
    let removed_findings = finding_count_delta(&run.findings, &baseline.findings, DeltaKind::Removed);
    let unchanged_findings = finding_intersection(&run.findings, &baseline.findings);
    let critical_unreviewed = run
        .findings
        .values()
        .filter(|finding| is_critical_lint(&finding.lint))
        .filter(|finding| !finding_is_reviewed(finding, &reviews, &input.profile, &input.as_of))
        .cloned()
        .collect::<Vec<_>>();

    push_baseline_checks(&mut checks, &facts, &new_findings, &critical_unreviewed);
    push_baseline_diagnostics(&mut diagnostics, DiagnosticInput {
        input,
        baseline: &baseline,
        run: &run,
        facts: &facts,
        new_findings: &new_findings,
        critical_unreviewed: &critical_unreviewed,
    });

    let decision = if checks.iter().all(|check| check.status == "pass") {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = octet_baseline_receipt_value(OctetBaselineReceiptInput {
        decision: &decision,
        baseline_ref: &baseline.baseline_ref,
        status_ref: &run.status_ref,
        new_findings: &new_findings,
        removed_findings: &removed_findings,
        unchanged_findings: &unchanged_findings,
        critical_unreviewed: &critical_unreviewed,
        review_refs: &review_refs,
        expired: !facts.is_baseline_current,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let receipt_ref = canonical_hash(&receipt_value)?;
    Ok(OctetBaselineEvaluation {
        decision,
        receipt_ref,
        receipt_value,
        diagnostics,
    })
}

fn push_baseline_checks(
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    facts: &BaselineFacts,
    new_findings: &[FindingEntry],
    critical_unreviewed: &[FindingEntry],
) {
    push_check(checks, "baseline-profile-allowed", facts.is_profile_allowed);
    push_check(checks, "baseline-not-expired", facts.is_baseline_current);
    push_check(checks, "baseline-config-current", facts.is_config_current);
    push_check(checks, "baseline-profile-current", facts.is_profile_hash_current);
    push_check(checks, "baseline-no-new-findings", new_findings.is_empty());
    push_check(checks, "baseline-no-unkeyed-findings", facts.has_zero_unkeyed_findings);
    push_check(checks, "baseline-critical-reviewed", critical_unreviewed.is_empty());
    push_check(checks, "baseline-review-refs-bound", facts.has_bound_review_refs);
    push_check(checks, "baseline-shrink-target", facts.is_within_shrink_target);
}

fn push_baseline_diagnostics(diagnostics: &mut impl crate::bounded::VecSink<String>, context: DiagnosticInput<'_>) {
    if !context.facts.is_profile_allowed {
        push_diagnostic(diagnostics, format!("baseline does not allow profile `{}`", context.input.profile));
    }
    if !context.facts.is_baseline_current {
        push_diagnostic(
            diagnostics,
            format!("octet warning baseline expired at {} as_of {}", context.baseline.expires_at, context.input.as_of),
        );
    }
    if !context.facts.is_config_current {
        push_diagnostic(
            diagnostics,
            format!(
                "baseline config hash mismatch: baseline={} current={}",
                context.baseline.config_hash, context.run.status.metadata.config_hash
            ),
        );
    }
    if !context.facts.is_profile_hash_current {
        push_diagnostic(
            diagnostics,
            format!(
                "baseline profile hash mismatch: baseline={} current={}",
                context.baseline.profile_hash, context.run.status.metadata.profile_hash
            ),
        );
    }
    if !context.new_findings.is_empty() {
        push_diagnostic(diagnostics, format!("new or increased octet findings: {}", context.new_findings.len()));
    }
    if !context.facts.has_zero_unkeyed_findings {
        push_diagnostic(diagnostics, format!("unkeyed octet findings: {}", context.run.unkeyed_findings));
    }
    if !context.critical_unreviewed.is_empty() {
        push_diagnostic(
            diagnostics,
            format!("unreviewed critical baseline findings: {}", context.critical_unreviewed.len()),
        );
    }
    if !context.facts.has_bound_review_refs {
        push_diagnostic(diagnostics, "baseline references review manifests not supplied to check".to_string());
    }
    if !context.facts.is_within_shrink_target {
        push_diagnostic(
            diagnostics,
            format!(
                "baseline burn-down target exceeded: current={} target={}",
                context.run.status.total_findings, context.baseline.target_next
            ),
        );
    }
}

pub fn validate_octet_source_gate(input: &OctetSourceGateValidationInput) -> Result<OctetSourceGateValidation> {
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();
    let setup = prepare_source_validation(input, &mut checks, &mut diagnostics)?;
    let requirement_value = octet_source_gate_requirement_value(
        &input.consumer,
        &input.subject_ref,
        &setup.source_scope,
        setup.expected.as_ref(),
        &checks,
    );
    let requirement_ref = canonical_hash(&requirement_value)?;
    let parsed = parse_source_receipt(input.gate_receipt_value.as_ref(), &mut checks, &mut diagnostics);
    let validation_refs = validate_source_receipt(
        ReceiptCheckInput {
            parsed: parsed.as_ref(),
            expected: setup.expected.as_ref(),
        },
        &mut checks,
        &mut diagnostics,
    );
    let gate_receipt_ref = validation_refs.receipt_ref.clone();
    let decision = if checks.iter().all(|check| check.status == "pass") {
        "pass"
    } else {
        "deny"
    };
    let value = octet_source_gate_validation_value(OctetSourceGateValidationValueInput {
        decision,
        requirement_ref: &requirement_ref,
        gate_receipt_ref: gate_receipt_ref.as_deref(),
        policy_ref: validation_refs.policy_ref.as_deref(),
        status_ref: validation_refs.status_ref.as_deref(),
        summary_ref: validation_refs.summary_ref.as_deref(),
        findings_ref: validation_refs.findings_ref.as_deref(),
        object_corpus_ref: validation_refs.object_corpus_ref.as_deref(),
        fingerprint_ref: validation_refs.fingerprint_ref.as_deref(),
        counts: &validation_refs.counts,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let validation_ref = canonical_hash(&value)?;
    Ok(OctetSourceGateValidation {
        decision: decision.to_string(),
        requirement_ref,
        validation_ref,
        gate_receipt_ref,
        value,
        diagnostics,
    })
}

fn normalized_source_scope(input: &OctetSourceGateValidationInput) -> Result<Vec<String>> {
    if input.source_scope.is_empty() {
        return default_source_scope(&input.consumer);
    }
    let mut scope = input.source_scope.clone();
    scope.sort();
    scope.dedup();
    Ok(scope)
}

fn prepare_source_validation(
    input: &OctetSourceGateValidationInput,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<SourceSetup> {
    let source_scope = normalized_source_scope(input)?;
    let is_consumer_supported = SOURCE_GATE_CONSUMERS.iter().any(|consumer| consumer == &input.consumer.as_str());
    push_check(checks, "source-gate-consumer-supported", is_consumer_supported);
    if !is_consumer_supported {
        push_diagnostic(diagnostics, format!("unsupported octet source-gate consumer {}", input.consumer));
    }
    let is_subject_ref_valid = is_content_ref(&input.subject_ref);
    push_check(checks, "source-gate-subject-ref", is_subject_ref_valid);
    if !is_subject_ref_valid {
        push_diagnostic(diagnostics, format!("invalid octet source-gate subject ref {}", input.subject_ref));
    }
    let expected = expected_metadata_for_command(DEFAULT_GATE_COMMAND).ok();
    push_check(checks, "current-octet-metadata", expected.is_some());
    if expected.is_none() {
        push_diagnostic(diagnostics, "cannot derive current Octet workspace metadata".to_string());
    }
    Ok(SourceSetup { source_scope, expected })
}

fn parse_source_receipt(
    value: Option<&IOValue>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<ParsedOctetGateReceipt> {
    match value {
        Some(value) => match parse_octet_gate_receipt(value) {
            Ok(parsed) => {
                push_check(checks, "gate-receipt-present", true);
                push_check(checks, "gate-receipt-parse", true);
                Some(parsed)
            }
            Err(error) => {
                push_check(checks, "gate-receipt-parse", false);
                push_diagnostic(diagnostics, format!("invalid octet gate receipt: {error}"));
                None
            }
        },
        None => {
            push_check(checks, "gate-receipt-present", false);
            push_diagnostic(diagnostics, "missing octet gate receipt value".to_string());
            None
        }
    }
}

fn validate_source_receipt(
    input: ReceiptCheckInput<'_>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> ValidationRefs {
    let Some(parsed) = input.parsed else {
        push_missing_receipt_checks(checks);
        return ValidationRefs::default();
    };
    let refs = receipt_refs(parsed);
    check_receipt_basics(parsed, checks, diagnostics);
    check_receipt_freshness(parsed, input.expected, checks, diagnostics);
    refs
}

fn receipt_refs(parsed: &ParsedOctetGateReceipt) -> ValidationRefs {
    ValidationRefs {
        counts: parsed.counts.clone(),
        receipt_ref: Some(parsed.receipt_ref.clone()),
        policy_ref: Some(parsed.policy_ref.clone()),
        status_ref: parsed.status_ref.clone(),
        summary_ref: parsed.summary_ref.clone(),
        findings_ref: parsed.findings_ref.clone(),
        object_corpus_ref: parsed.object_corpus_ref.clone(),
        fingerprint_ref: parsed.fingerprint_ref.clone(),
    }
}

fn check_receipt_basics(
    parsed: &ParsedOctetGateReceipt,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let is_receipt_pass = parsed.decision == "pass";
    push_check(checks, "gate-receipt-pass", is_receipt_pass);
    if !is_receipt_pass {
        push_diagnostic(diagnostics, format!("octet gate receipt decision is {}", parsed.decision));
    }
    let has_strict_profile_checks = parsed_check_pass(parsed, "profile-supported")
        && parsed_check_pass(parsed, "strict-status-clean")
        && parsed_check_pass(parsed, "no-critical-findings");
    push_check(checks, "strict-profile-required", has_strict_profile_checks);
    if !has_strict_profile_checks {
        push_diagnostic(diagnostics, "octet gate receipt is not strict clean source-gate pass evidence".to_string());
    }
    let has_required_artifact_refs = parsed.command_ref.is_some()
        && parsed.status_ref.is_some()
        && parsed.summary_ref.is_some()
        && parsed.findings_ref.is_some()
        && parsed.object_corpus_ref.is_some()
        && parsed.fingerprint_ref.is_some();
    push_check(checks, "required-artifact-refs", has_required_artifact_refs);
    if !has_required_artifact_refs {
        push_diagnostic(diagnostics, "octet gate receipt is missing required artifact refs".to_string());
    }
    let has_clean_finding_counts = parsed.counts.total == 0
        && parsed.counts.warnings == 0
        && parsed.counts.errors == 0
        && parsed.counts.critical == 0
        && parsed.counts.uncovered == 0;
    push_check(checks, "no-uncovered-findings", has_clean_finding_counts);
    if !has_clean_finding_counts {
        push_diagnostic(
            diagnostics,
            format!(
                "octet gate receipt has findings={} warnings={} errors={} critical={} uncovered={}",
                parsed.counts.total,
                parsed.counts.warnings,
                parsed.counts.errors,
                parsed.counts.critical,
                parsed.counts.uncovered
            ),
        );
    }
}

fn check_receipt_freshness(
    parsed: &ParsedOctetGateReceipt,
    expected: Option<&ExpectedMetadata>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    let is_config_hash_current = expected
        .zip(parsed.config_hash.as_ref())
        .is_some_and(|(expected, actual)| expected.config_hash == *actual);
    let is_profile_hash_current = expected
        .zip(parsed.profile_hash.as_ref())
        .is_some_and(|(expected, actual)| expected.profile_hash == *actual);
    push_check(checks, "current-config-ref", is_config_hash_current);
    push_check(checks, "current-profile-ref", is_profile_hash_current);
    if !is_config_hash_current {
        push_diagnostic(diagnostics, "octet gate receipt config hash is stale or missing".to_string());
    }
    if !is_profile_hash_current {
        push_diagnostic(diagnostics, "octet gate receipt profile hash is stale or missing".to_string());
    }
    let has_scope_fingerprint_coverage = parsed.fingerprint_ref.as_deref().is_some_and(is_content_ref)
        && parsed.object_corpus_ref.as_deref().is_some_and(is_content_ref)
        && parsed_check_pass(parsed, "fingerprint-evidence-bound")
        && parsed_check_pass(parsed, "object-corpus-critical-paths")
        && parsed_check_pass(parsed, "object-corpus-fingerprint");
    push_check(checks, "scope-fingerprint-coverage", has_scope_fingerprint_coverage);
    if !has_scope_fingerprint_coverage {
        push_diagnostic(
            diagnostics,
            "octet gate receipt lacks object-corpus/fingerprint coverage for required source scope".to_string(),
        );
    }
    let has_toolchain = parsed.toolchain.is_some();
    push_check(checks, "toolchain-bound", has_toolchain);
    if !has_toolchain {
        push_diagnostic(diagnostics, "octet gate receipt missing toolchain metadata".to_string());
    }
}

fn push_missing_receipt_checks(checks: &mut impl crate::bounded::VecSink<GateCheck>) {
    push_check(checks, "gate-receipt-pass", false);
    push_check(checks, "strict-profile-required", false);
    push_check(checks, "required-artifact-refs", false);
    push_check(checks, "no-uncovered-findings", false);
    push_check(checks, "current-config-ref", false);
    push_check(checks, "current-profile-ref", false);
    push_check(checks, "scope-fingerprint-coverage", false);
    push_check(checks, "toolchain-bound", false);
}

fn octet_source_gate_requirement_value(
    consumer: &str,
    subject_ref: &str,
    source_scope: &[String],
    expected: Option<&ExpectedMetadata>,
    checks: &[GateCheck],
) -> IOValue {
    record("octet-source-gate-requirement-v1", vec![
        string(OCTET_SOURCE_GATE_REQUIREMENT_SCHEMA),
        record("consumer", vec![string(consumer)]),
        record("subject", vec![string(subject_ref)]),
        record("required-profile", vec![string(STRICT_PROFILE)]),
        record("source-scope", vec![sequence(source_scope.iter().map(string).collect())]),
        record("current-config", vec![optional_ref(expected.map(|metadata| metadata.config_hash.as_str()))]),
        record("current-profile", vec![optional_ref(expected.map(|metadata| metadata.profile_hash.as_str()))]),
        record("required-evidence", vec![sequence(vec![
            string("status"),
            string("summary"),
            string("structured-findings"),
            string("object-corpus"),
            string("fingerprint"),
        ])]),
        record("freshness", vec![string("same-workspace-metadata")]),
        checks_value(checks),
    ])
}

pub fn default_source_scope(consumer: &str) -> Result<Vec<String>> {
    let scope = match consumer {
        "node-startup" => vec!["src/main.rs", "src/node/runtime.rs", "src/octet/gate.rs"],
        "job-remote-admission" => vec!["src/job/dag.rs", "src/main.rs", "src/octet/gate.rs"],
        "upgrade-plan" => vec!["src/main.rs", "src/octet/gate.rs", "src/upgrades/mod.rs"],
        "node-control-gate" => vec![
            "src/main.rs",
            "src/node/daemon.rs",
            "src/node/runtime.rs",
            "src/octet/gate.rs",
        ],
        other => return Err(MoltenError::invalid_harness(format!("unsupported octet source-gate consumer {other}"))),
    };
    Ok(scope.into_iter().map(ToOwned::to_owned).collect())
}

pub fn octet_gate_policy_value(input: &OctetGateInput) -> IOValue {
    record("octet-gate-policy-v1", vec![
        string(crate::preserves_rail::OCTET_GATE_POLICY_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        record("command", vec![sequence(vec![
            string("cargo"),
            string("octet"),
            string("check"),
            string("--artifact-dir"),
            string(input.artifacts_dir.to_string_lossy()),
        ])]),
        record("required-artifacts", vec![sequence(vec![
            string(COMMAND_NAME),
            string(STATUS_NAME),
            string(SUMMARY_NAME),
            string(OBJECT_CORPUS_RECEIPT_NAME),
        ])]),
        record("deny-statuses", vec![sequence(vec![
            string("warning-only"),
            string("lint-failure"),
            string("integration-failure"),
            string("missing"),
            string("malformed"),
            string("stale"),
        ])]),
        record("critical-lints", vec![sequence(CRITICAL_LINTS.iter().map(string).collect())]),
        record("quarantine-policy", vec![record("none", Vec::new())]),
        checks_value(&[
            GateCheck {
                name: "strict-profile",
                status: "pass",
            },
            GateCheck {
                name: "warning-only-denies",
                status: "pass",
            },
            GateCheck {
                name: "required-artifacts-bound",
                status: "pass",
            },
        ]),
    ])
}

struct OctetGateReceiptInput<'a> {
    decision: &'a str,
    policy_ref: &'a str,
    command_ref: Option<&'a str>,
    status_ref: Option<&'a str>,
    summary_ref: Option<&'a str>,
    structured_findings_ref: Option<&'a str>,
    object_corpus_ref: Option<&'a str>,
    fingerprint_evidence_ref: Option<&'a str>,
    config_hash: Option<&'a str>,
    profile_hash: Option<&'a str>,
    toolchain: Option<&'a str>,
    counts: &'a FindingCounts,
    diagnostics: &'a [String],
    checks: &'a [GateCheck],
}

struct OctetSourceGateValidationValueInput<'a> {
    decision: &'a str,
    requirement_ref: &'a str,
    gate_receipt_ref: Option<&'a str>,
    policy_ref: Option<&'a str>,
    status_ref: Option<&'a str>,
    summary_ref: Option<&'a str>,
    findings_ref: Option<&'a str>,
    object_corpus_ref: Option<&'a str>,
    fingerprint_ref: Option<&'a str>,
    counts: &'a FindingCounts,
    diagnostics: &'a [String],
    checks: &'a [GateCheck],
}

fn octet_source_gate_validation_value(input: OctetSourceGateValidationValueInput<'_>) -> IOValue {
    record("octet-source-gate-validation-v1", vec![
        string(OCTET_SOURCE_GATE_VALIDATION_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("requirement", vec![string(input.requirement_ref)]),
        record("gate-receipt", vec![optional_ref(input.gate_receipt_ref)]),
        record("gate-policy", vec![optional_ref(input.policy_ref)]),
        record("status", vec![optional_ref(input.status_ref)]),
        record("summary", vec![optional_ref(input.summary_ref)]),
        record("findings", vec![optional_ref(input.findings_ref)]),
        record("object-corpus", vec![optional_ref(input.object_corpus_ref)]),
        record("fingerprint", vec![optional_ref(input.fingerprint_ref)]),
        counts_value(input.counts),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn parse_octet_gate_receipt(value: &IOValue) -> Result<ParsedOctetGateReceipt> {
    let fields = value
        .collect_simple_record("octet-gate-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <octet-gate-receipt-v1 ...>"))?;
    require_schema(&fields[0], OCTET_GATE_RECEIPT_SCHEMA, "octet gate receipt")?;
    let metadata = value_to_iovalue(&fields[11]);
    let metadata_fields = metadata
        .collect_simple_record("metadata", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected octet gate metadata record"))?;
    Ok(ParsedOctetGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        policy_ref: record_string(&fields[2], "policy")?,
        command_ref: record_optional_string(&fields[3], "command")?,
        status_ref: record_optional_string(&fields[4], "status")?,
        summary_ref: record_optional_string(&fields[5], "summary")?,
        findings_ref: record_optional_string(&fields[6], "findings")?,
        object_corpus_ref: record_optional_string(&fields[7], "object-corpus")?,
        fingerprint_ref: record_optional_string(&fields[8], "fingerprint")?,
        config_hash: record_optional_string(&metadata_fields[0], "config-hash")?,
        profile_hash: record_optional_string(&metadata_fields[1], "profile-hash")?,
        toolchain: record_optional_string(&metadata_fields[2], "toolchain")?,
        counts: parse_counts(&fields[12])?,
        diagnostics: record_string_sequence(&fields[13], "diagnostics")?,
        checks: parse_check_pairs(&fields[14])?,
    })
}

fn parsed_check_pass(parsed: &ParsedOctetGateReceipt, name: &str) -> bool {
    parsed.checks.iter().any(|(check_name, status)| check_name == name && status == "pass")
}

fn parse_counts(value: &Value<IOValue>) -> Result<FindingCounts> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("counts", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected counts record"))?;
    Ok(FindingCounts {
        total: record_u64(&fields[0], "findings")?,
        warnings: record_u64(&fields[1], "warnings")?,
        errors: record_u64(&fields[2], "errors")?,
        autofixable: record_u64(&fields[3], "autofixable")?,
        critical: record_u64(&fields[4], "critical")?,
        uncovered: record_u64(&fields[5], "uncovered")?,
    })
}

fn parse_check_pairs(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected checks record"))?;
    let items = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("expected checks sequence"))?;
    items
        .iter()
        .map(|item| {
            let item = value_to_iovalue(item);
            let fields = item
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected check record"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn record_optional_string(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    optional_string(&fields[0], label)
}

fn optional_string(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional {label}")))?;
    Ok(Some(required_string(&fields[0], label)?))
}

fn is_content_ref(value: &str) -> bool {
    (value.starts_with("blake3:") || value.starts_with("b3:"))
        && value.split_once(':').is_some_and(|(_, hash)| !hash.is_empty())
}

fn octet_raw_artifact_value(label: &'static str, schema: &str, name: &str, file: &GateFile) -> IOValue {
    record(label, vec![
        string(schema),
        record("name", vec![string(name)]),
        record("content-ref", vec![string(&file.artifact_ref)]),
        record("content", vec![string(&file.text)]),
        checks_value(&[
            GateCheck {
                name: "raw-content-ref-bound",
                status: "pass",
            },
            GateCheck {
                name: "diagnostic-artifact",
                status: "pass",
            },
        ]),
    ])
}

fn octet_artifact_ledger_receipt_value(
    decision: &str,
    artifacts_dir: &str,
    imported_refs: &[String],
    diagnostics: &[String],
    checks: &[GateCheck],
) -> IOValue {
    record("octet-artifact-ledger-receipt-v1", vec![
        string(crate::preserves_rail::OCTET_ARTIFACT_LEDGER_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("artifacts-dir", vec![string(artifacts_dir)]),
        record("imported", vec![sequence(imported_refs.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(checks),
    ])
}

fn octet_gate_receipt_value(input: OctetGateReceiptInput<'_>) -> IOValue {
    record("octet-gate-receipt-v1", vec![
        string(OCTET_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("policy", vec![string(input.policy_ref)]),
        record("command", vec![optional_ref(input.command_ref)]),
        record("status", vec![optional_ref(input.status_ref)]),
        record("summary", vec![optional_ref(input.summary_ref)]),
        record("findings", vec![optional_ref(input.structured_findings_ref)]),
        record("object-corpus", vec![optional_ref(input.object_corpus_ref)]),
        record("fingerprint", vec![optional_ref(input.fingerprint_evidence_ref)]),
        record("baseline", vec![record("none", Vec::new())]),
        record("review-refs", vec![sequence(Vec::new())]),
        record("metadata", vec![
            record("config-hash", vec![optional_ref(input.config_hash)]),
            record("profile-hash", vec![optional_ref(input.profile_hash)]),
            record("toolchain", vec![optional_ref(input.toolchain)]),
        ]),
        counts_value(input.counts),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn read_required_file(
    artifacts_dir: &Path,
    name: &'static str,
    check_name: &'static str,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<GateFile> {
    let path = artifacts_dir.join(name);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            push_check(checks, check_name, false);
            push_diagnostic(diagnostics, format!("missing or unreadable {name}: {error}"));
            return None;
        }
    };
    let text = match String::from_utf8(bytes.clone()) {
        Ok(text) => text,
        Err(error) => {
            push_check(checks, check_name, false);
            push_diagnostic(diagnostics, format!("{name} is not UTF-8 text: {error}"));
            return None;
        }
    };
    push_check(checks, check_name, true);
    Some(GateFile {
        artifact_ref: bytes_ref(&bytes),
        text,
    })
}

fn parse_status(
    status_file: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<OctetStatusArtifact> {
    let Some(status_file) = status_file else {
        push_check(checks, "status-json-parse", false);
        return None;
    };
    match serde_json::from_str::<OctetStatusArtifact>(&status_file.text) {
        Ok(status) => {
            let has_complete_metadata = status.metadata.tool_name == "cargo-octet"
                && !status.metadata.tool_version.is_empty()
                && !status.metadata.rustc_version.is_empty()
                && !status.metadata.toolchain.is_empty()
                && !status.metadata.config_hash.is_empty()
                && !status.metadata.profile_hash.is_empty();
            let has_consistent_exit_status =
                status.exit_code == 0 || status.status == "lint-failure" || status.status == "integration-failure";
            let is_tool_version_supported = status.metadata.tool_version == SUPPORTED_OCTET_TOOL_VERSION;
            push_check(checks, "status-json-parse", true);
            push_check(checks, "status-metadata-complete", has_complete_metadata);
            push_check(checks, "status-tool-version-supported", is_tool_version_supported);
            push_check(checks, "status-exit-consistent", has_consistent_exit_status);
            if !has_complete_metadata {
                push_diagnostic(diagnostics, "status metadata missing required cargo-octet fields".to_string());
            }
            if !is_tool_version_supported {
                push_diagnostic(
                    diagnostics,
                    format!("unsupported cargo-octet version `{}`", status.metadata.tool_version),
                );
            }
            if !has_consistent_exit_status {
                push_diagnostic(
                    diagnostics,
                    format!("status exit_code {} is inconsistent with status `{}`", status.exit_code, status.status),
                );
            }
            Some(status)
        }
        Err(error) => {
            push_check(checks, "status-json-parse", false);
            push_diagnostic(diagnostics, format!("malformed status.json: {error}"));
            None
        }
    }
}

fn parse_summary_lints(
    summary: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> BTreeMap<String, u64> {
    let Some(summary) = summary else {
        push_check(checks, "summary-lints-parse", false);
        return BTreeMap::new();
    };
    let mut lints = BTreeMap::new();
    let mut is_parsing_lints = false;
    for line in summary.text.lines() {
        let trimmed = line.trim();
        if trimmed == "By lint:" {
            is_parsing_lints = true;
            continue;
        }
        if is_parsing_lints && (trimmed.is_empty() || trimmed == "Index:") {
            break;
        }
        if !is_parsing_lints {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(count) = parts.next() else { continue };
        let Ok(count) = count.parse::<u64>() else { continue };
        if insert_bounded(&mut lints, name.to_string(), count, MAX_OCTET_SUMMARY_LINTS, "summary lint counts").is_err()
        {
            push_diagnostic(diagnostics, "summary lint count exceeds configured bound".to_string());
            break;
        }
    }
    push_check(checks, "summary-lints-parse", true);
    if lints.is_empty() && summary.text.contains("Findings:") && !summary.text.contains("Findings: 0") {
        push_diagnostic(diagnostics, "summary contains findings but no parseable lint counts".to_string());
    }
    lints
}

fn validate_object_corpus(
    object_corpus: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Option<OctetObjectCorpusReceipt> {
    let Some(object_corpus) = object_corpus else {
        push_check(checks, "object-corpus-json-parse", false);
        push_check(checks, "object-corpus-schema", false);
        push_check(checks, "object-corpus-nonempty", false);
        push_check(checks, "object-corpus-fingerprint", false);
        push_check(checks, "object-corpus-critical-paths", false);
        return None;
    };
    let parsed = serde_json::from_str::<OctetObjectCorpusReceipt>(&object_corpus.text);
    let Ok(receipt) = parsed else {
        push_check(checks, "object-corpus-json-parse", false);
        push_check(checks, "object-corpus-schema", false);
        push_check(checks, "object-corpus-nonempty", false);
        push_check(checks, "object-corpus-fingerprint", false);
        push_check(checks, "object-corpus-critical-paths", false);
        push_diagnostic(diagnostics, "malformed object corpus receipt JSON".to_string());
        return None;
    };
    let has_supported_schema =
        receipt.schema.as_deref() == Some(OCTET_OBJECT_CORPUS_SCHEMA) && receipt.schema_version == Some(1);
    let has_objects = receipt.object_count.is_some_and(|count| count > 0);
    let has_object_set_fingerprint = receipt.object_set_hash.as_deref().is_some_and(is_b3_ref);
    let has_required_source_paths = receipt.source_paths.as_ref().is_some_and(|paths| {
        REQUIRED_OBJECT_CORPUS_SOURCE_PATHS.iter().all(|required| paths.iter().any(|path| path == required))
    });
    push_check(checks, "object-corpus-json-parse", true);
    push_check(checks, "object-corpus-schema", has_supported_schema);
    push_check(checks, "object-corpus-nonempty", has_objects);
    push_check(checks, "object-corpus-fingerprint", has_object_set_fingerprint);
    push_check(checks, "object-corpus-critical-paths", has_required_source_paths);
    if !has_supported_schema {
        push_diagnostic(diagnostics, "object corpus receipt has missing or unsupported schema".to_string());
    }
    if !has_objects {
        push_diagnostic(diagnostics, "object corpus receipt has no focused objects".to_string());
    }
    if !has_object_set_fingerprint {
        push_diagnostic(diagnostics, "object corpus receipt is missing object_set_hash fingerprint".to_string());
    }
    if !has_required_source_paths {
        push_diagnostic(diagnostics, "object corpus receipt does not cover required critical paths".to_string());
    }
    if has_supported_schema && has_objects && has_object_set_fingerprint && has_required_source_paths {
        Some(receipt)
    } else {
        None
    }
}

fn octet_fingerprint_evidence_value(object_corpus: &GateFile, receipt: &OctetObjectCorpusReceipt) -> Result<IOValue> {
    let object_set_hash = receipt
        .object_set_hash
        .as_deref()
        .ok_or_else(|| MoltenError::invalid_harness("object corpus missing object_set_hash"))?;
    let source_paths = receipt
        .source_paths
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("object corpus missing source_paths"))?;
    let object_count = receipt
        .object_count
        .ok_or_else(|| MoltenError::invalid_harness("object corpus missing object_count"))?;
    let pure_cache_blocked = receipt.pure_cache_blocked_count.unwrap_or(0);
    let mut sorted_paths = source_paths.clone();
    sorted_paths.sort();
    Ok(record("octet-fingerprint-evidence-v1", vec![
        string(OCTET_FINGERPRINT_EVIDENCE_SCHEMA),
        record("object-corpus", vec![string(&object_corpus.artifact_ref)]),
        record("object-set-hash", vec![string(object_set_hash)]),
        record("source-paths", vec![sequence(sorted_paths.iter().map(string).collect())]),
        record("object-count", vec![u64_value(object_count)]),
        record("pure-cache-blocked", vec![u64_value(pure_cache_blocked)]),
        checks_value(&[
            GateCheck {
                name: "object-corpus-fingerprint",
                status: "pass",
            },
            GateCheck {
                name: "critical-path-coverage",
                status: "pass",
            },
        ]),
    ]))
}

fn is_b3_ref(value: &str) -> bool {
    value.starts_with("b3:") && value.len() > "b3:".len()
}

fn validate_command(
    command: Option<&GateFile>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> bool {
    let Some(command) = command else {
        push_check(checks, "command-shape", false);
        return false;
    };
    let normalized = command.text.trim();
    let has_canonical_command_shape =
        normalized.starts_with("cargo octet check") && normalized.contains("--artifact-dir");
    push_check(checks, "command-shape", has_canonical_command_shape);
    if !has_canonical_command_shape {
        push_diagnostic(diagnostics, format!("noncanonical octet command: {normalized}"));
    }
    has_canonical_command_shape
}

fn validate_metadata_binding(
    command: Option<&GateFile>,
    status: Option<&OctetStatusArtifact>,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> bool {
    let Some(command) = command else {
        push_check(checks, "status-config-current", false);
        push_check(checks, "status-profile-current", false);
        return false;
    };
    let Some(status) = status else {
        push_check(checks, "status-config-current", false);
        push_check(checks, "status-profile-current", false);
        return false;
    };
    let expected = match expected_metadata_for_command(command.text.trim()) {
        Ok(expected) => expected,
        Err(message) => {
            push_check(checks, "status-config-current", false);
            push_check(checks, "status-profile-current", false);
            push_diagnostic(diagnostics, message);
            return false;
        }
    };
    let is_status_config_current = status.metadata.config_hash == expected.config_hash;
    let is_status_profile_current = status.metadata.profile_hash == expected.profile_hash;
    push_check(checks, "status-config-current", is_status_config_current);
    push_check(checks, "status-profile-current", is_status_profile_current);
    if !is_status_config_current {
        push_diagnostic(
            diagnostics,
            format!("stale octet config hash: status={} current={}", status.metadata.config_hash, expected.config_hash),
        );
    }
    if !is_status_profile_current {
        push_diagnostic(
            diagnostics,
            format!(
                "stale octet profile hash: status={} current={}",
                status.metadata.profile_hash, expected.profile_hash
            ),
        );
    }
    is_status_config_current && is_status_profile_current
}

fn expected_metadata_for_command(command: &str) -> std::result::Result<ExpectedMetadata, String> {
    let workspace_root = std::env::current_dir().map_err(|error| format!("current_dir: {error}"))?;
    let workspace_config = load_workspace_octet_config(&workspace_root)?;
    let effective = parse_effective_command(command, &workspace_config)?;
    let config_hash = current_config_hash(&workspace_root, &effective.scope_args, &effective.cargo_check_args);
    let profile_hash = current_profile_hash(
        &effective.scope_args,
        &effective.cargo_check_args,
        &effective.output_format,
        &config_hash,
    );
    Ok(ExpectedMetadata {
        config_hash,
        profile_hash,
    })
}

fn parse_effective_command(
    command: &str,
    workspace_config: &WorkspaceOctetConfig,
) -> std::result::Result<EffectiveOctetCommand, String> {
    let mut tokens = Vec::with_capacity(MAX_OCTET_COMMAND_TOKENS.min(command.len()));
    for token in command.split_whitespace() {
        if tokens.len() >= MAX_OCTET_COMMAND_TOKENS {
            return Err(format!("octet command token count exceeds bound {MAX_OCTET_COMMAND_TOKENS}: {command}"));
        }
        tokens.push(token.to_string());
    }
    if tokens.len() < 3 || tokens[0] != "cargo" || tokens[1] != "octet" || tokens[2] != "check" {
        return Err(format!("cannot derive octet metadata from noncanonical command: {command}"));
    }
    let mut scope_args = Vec::new();
    let mut cargo_check_args = None;
    let mut output_format = "human".to_string();
    let mut index = 3;
    while index < tokens.len() {
        let token = &tokens[index];
        if token == "--" {
            let args_len = tokens.len().saturating_sub(index + 1);
            if args_len > MAX_OCTET_COMMAND_TOKENS {
                return Err("octet cargo-check argument count exceeds command token bound".to_string());
            }
            cargo_check_args = Some(tokens[index + 1..].to_vec());
            break;
        }
        if token == "--output-format" {
            let Some(value) = tokens.get(index + 1) else {
                return Err("missing value after --output-format in octet command".to_string());
            };
            output_format = value.clone();
            index += 2;
            continue;
        }
        if let Some(value) = token.strip_prefix("--output-format=") {
            output_format = value.to_string();
            index += 1;
            continue;
        }
        if option_takes_value(token) {
            if tokens.get(index + 1).is_none() {
                return Err(format!("missing value after {token} in octet command"));
            }
            index += 2;
            continue;
        }
        if option_with_inline_value(token) || token == "--cache" {
            index += 1;
            continue;
        }
        push_token_bounded(&mut scope_args, token.clone())?;
        index += 1;
    }
    let scope_args = if scope_args.is_empty() {
        workspace_config.default_scope.clone()
    } else {
        scope_args
    };
    let cargo_check_args = cargo_check_args.unwrap_or_else(|| workspace_config.cargo_check_args.clone());
    Ok(EffectiveOctetCommand {
        scope_args,
        cargo_check_args,
        output_format,
    })
}

fn option_takes_value(token: &str) -> bool {
    matches!(token, "--artifact-dir" | "--baseline" | "--write-baseline")
}

fn option_with_inline_value(token: &str) -> bool {
    token.starts_with("--artifact-dir=") || token.starts_with("--baseline=") || token.starts_with("--write-baseline=")
}

fn load_workspace_octet_config(workspace_root: &Path) -> std::result::Result<WorkspaceOctetConfig, String> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let source =
        fs::read_to_string(&manifest_path).map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let document = source
        .parse::<toml::Table>()
        .map_err(|error| format!("parse {}: {error}", manifest_path.display()))?;
    let octet = document
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("metadata"))
        .and_then(toml::Value::as_table)
        .and_then(|metadata| metadata.get("octet"))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("missing [workspace.metadata.octet] in {}", manifest_path.display()))?;
    Ok(WorkspaceOctetConfig {
        default_scope: string_array_field(octet, "default_scope", &manifest_path)?,
        cargo_check_args: string_array_field(octet, "cargo_check_args", &manifest_path)?,
    })
}

fn string_array_field(
    table: &toml::Table,
    key: &str,
    manifest_path: &Path,
) -> std::result::Result<Vec<String>, String> {
    let values = table
        .get(key)
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("missing `{key}` array in {}", manifest_path.display()))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("`{key}` must contain only strings in {}", manifest_path.display()))
        })
        .collect()
}

fn current_config_hash(workspace_root: &Path, scope_args: &[String], cargo_check_args: &[String]) -> String {
    let files = vec![
        file_hash_entry(workspace_root.join("Cargo.toml")),
        file_hash_entry(workspace_root.join("dylint.toml")),
    ];
    let payload = serde_json::json!({
        "files": files,
        "effective_scope_args": scope_args,
        "effective_cargo_check_args": cargo_check_args,
    });
    b3_full_hash(&payload.to_string())
}

fn current_profile_hash(
    scope_args: &[String],
    cargo_check_args: &[String],
    output_format: &str,
    config_hash: &str,
) -> String {
    let payload = serde_json::json!({
        "scope_args": scope_args,
        "cargo_check_args": cargo_check_args,
        "output_format": output_format,
        "config_hash": config_hash,
    });
    b3_full_hash(&payload.to_string())
}

fn file_hash_entry(path: PathBuf) -> serde_json::Value {
    let relative = path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown");
    serde_json::json!({
        "path": relative,
        "hash": fs::read(&path).ok().and_then(|bytes| b3_ref_from_bytes(&bytes).ok()),
    })
}

fn b3_full_hash(input: &str) -> String {
    format!("b3:{}", blake3::hash(input.as_bytes()).to_hex())
}

enum DeltaKind {
    NewOrIncreased,
    Removed,
}

struct OctetBaselineReceiptInput<'a> {
    decision: &'a str,
    baseline_ref: &'a str,
    status_ref: &'a str,
    new_findings: &'a [FindingEntry],
    removed_findings: &'a [FindingEntry],
    unchanged_findings: &'a [FindingEntry],
    critical_unreviewed: &'a [FindingEntry],
    review_refs: &'a [String],
    expired: bool,
    diagnostics: &'a [String],
    checks: &'a [GateCheck],
}

struct OctetWarningBaselineValueInput<'a> {
    run: &'a CurrentOctetRun,
    created_at: &'a str,
    expires_at: &'a str,
    target_next: u64,
    source_snapshot_ref: &'a str,
    checks: &'a [GateCheck],
}

fn load_current_octet_run(
    artifacts_dir: &Path,
    checks: &mut impl crate::bounded::VecSink<GateCheck>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<CurrentOctetRun> {
    let command =
        read_required_file(artifacts_dir, COMMAND_NAME, "baseline-command-artifact-present", checks, diagnostics);
    let status_file =
        read_required_file(artifacts_dir, STATUS_NAME, "baseline-status-artifact-present", checks, diagnostics);
    let summary =
        read_required_file(artifacts_dir, SUMMARY_NAME, "baseline-summary-artifact-present", checks, diagnostics);
    let object_corpus = read_required_file(
        artifacts_dir,
        OBJECT_CORPUS_RECEIPT_NAME,
        "baseline-object-corpus-artifact-present",
        checks,
        diagnostics,
    );
    let Some(status_file) = status_file else {
        return Err(MoltenError::invalid_harness("octet baseline requires status.json"));
    };
    let Some(summary) = summary else {
        return Err(MoltenError::invalid_harness("octet baseline requires summary.txt"));
    };
    let Some(object_corpus) = object_corpus else {
        return Err(MoltenError::invalid_harness("octet baseline requires object-corpus-receipt.json"));
    };
    let Some(status) = parse_status(Some(&status_file), checks, diagnostics) else {
        return Err(MoltenError::invalid_harness("octet baseline requires parseable status.json"));
    };
    validate_command(command.as_ref(), checks, diagnostics);
    validate_metadata_binding(command.as_ref(), Some(&status), checks, diagnostics);
    let has_valid_object_corpus = validate_object_corpus(Some(&object_corpus), checks, diagnostics).is_some();
    if !has_valid_object_corpus {
        return Err(MoltenError::invalid_harness("octet baseline requires valid object corpus receipt"));
    }
    let (findings, parsed_count) = parse_summary_findings(&summary, &status);
    let unkeyed_findings = status.total_findings.saturating_sub(parsed_count);
    push_check(checks, "baseline-findings-keyed", unkeyed_findings == 0);
    if unkeyed_findings > 0 {
        push_diagnostic(diagnostics, format!("summary omitted stable keys for {unkeyed_findings} findings"));
    }
    Ok(CurrentOctetRun {
        status_ref: status_file.artifact_ref,
        summary_ref: summary.artifact_ref,
        object_corpus_ref: object_corpus.artifact_ref,
        status,
        findings,
        unkeyed_findings,
    })
}

fn octet_warning_baseline_value(input: &OctetWarningBaselineValueInput<'_>) -> IOValue {
    let critical_keys = critical_keys(&input.run.findings);
    record("octet-warning-baseline-v1", vec![
        string(OCTET_WARNING_BASELINE_SCHEMA),
        record("scope", vec![string("workspace")]),
        record("created-at", vec![string(input.created_at)]),
        record("expires-at", vec![string(input.expires_at)]),
        record("octet-config-hash", vec![string(&input.run.status.metadata.config_hash)]),
        record("octet-profile-hash", vec![string(&input.run.status.metadata.profile_hash)]),
        record("toolchain", vec![string(&input.run.status.metadata.toolchain)]),
        record("source-snapshot", vec![string(input.source_snapshot_ref)]),
        record("finding-keys", vec![sequence(input.run.findings.values().map(finding_entry_value).collect())]),
        record("critical-finding-keys", vec![sequence(critical_keys.iter().map(string).collect())]),
        record("allowed-profiles", vec![sequence(vec![string(QUARANTINE_PROFILE)])]),
        record("burn-down", vec![
            record("total", vec![u64_value(input.run.status.total_findings)]),
            record("target-next", vec![u64_value(input.target_next)]),
            record("deadline", vec![string(input.expires_at)]),
        ]),
        record("review-refs", vec![sequence(Vec::new())]),
        checks_value(input.checks),
    ])
}

fn octet_baseline_receipt_value(input: OctetBaselineReceiptInput<'_>) -> IOValue {
    record("octet-baseline-receipt-v1", vec![
        string(crate::preserves_rail::OCTET_BASELINE_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("baseline", vec![string(input.baseline_ref)]),
        record("run-status", vec![string(input.status_ref)]),
        record("new-findings", vec![sequence(input.new_findings.iter().map(finding_entry_value).collect())]),
        record("removed-findings", vec![sequence(
            input.removed_findings.iter().map(finding_entry_value).collect(),
        )]),
        record("unchanged-findings", vec![sequence(
            input.unchanged_findings.iter().map(finding_entry_value).collect(),
        )]),
        record("critical-unreviewed", vec![sequence(
            input.critical_unreviewed.iter().map(finding_entry_value).collect(),
        )]),
        record("review-refs", vec![sequence(input.review_refs.iter().map(string).collect())]),
        record("expired", vec![bool_value(input.expired)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(input.checks),
    ])
}

fn parse_review_manifests(values: &[IOValue]) -> Result<Vec<ParsedReviewManifest>> {
    values.iter().map(parse_review_manifest).collect()
}

fn parse_review_manifest(value: &IOValue) -> Result<ParsedReviewManifest> {
    let fields = value
        .collect_simple_record("octet-review-manifest-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <octet-review-manifest-v1 ...>"))?;
    require_schema(&fields[0], OCTET_REVIEW_MANIFEST_SCHEMA, "octet review manifest")?;
    Ok(ParsedReviewManifest {
        review_ref: canonical_hash(value)?,
        profile: record_string(&fields[1], "profile")?,
        expires_at: record_string(&fields[2], "expires-at")?,
        finding_keys: record_string_sequence(&fields[3], "finding-keys")?,
    })
}

fn finding_is_reviewed(finding: &FindingEntry, reviews: &[ParsedReviewManifest], profile: &str, as_of: &str) -> bool {
    reviews.iter().any(|review| {
        review.profile == profile
            && review.expires_at.as_str() >= as_of
            && review.finding_keys.iter().any(|key| key == &finding.key)
    })
}

fn parse_warning_baseline(value: &IOValue) -> Result<ParsedWarningBaseline> {
    let fields = value
        .collect_simple_record("octet-warning-baseline-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <octet-warning-baseline-v1 ...>"))?;
    require_schema(&fields[0], OCTET_WARNING_BASELINE_SCHEMA, "octet warning baseline")?;
    let findings = record_finding_entries(&fields[8], "finding-keys")?;
    let _critical_keys = record_string_sequence(&fields[9], "critical-finding-keys")?;
    let allowed_profiles = record_string_sequence(&fields[10], "allowed-profiles")?;
    let burn_down = value_to_iovalue(&fields[11]);
    let burn_down_fields = burn_down
        .collect_simple_record("burn-down", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("expected baseline burn-down record"))?;
    Ok(ParsedWarningBaseline {
        baseline_ref: canonical_hash(value)?,
        expires_at: record_string(&fields[3], "expires-at")?,
        config_hash: record_string(&fields[4], "octet-config-hash")?,
        profile_hash: record_string(&fields[5], "octet-profile-hash")?,
        findings,
        allowed_profiles,
        target_next: record_u64(&burn_down_fields[1], "target-next")?,
        review_refs: record_string_sequence(&fields[12], "review-refs")?,
    })
}

fn octet_structured_findings_value(
    status_file: &GateFile,
    summary: &GateFile,
    status: &OctetStatusArtifact,
) -> (IOValue, u64) {
    let (findings, parsed_count) = parse_summary_findings(summary, status);
    let unkeyed_findings = status.total_findings.saturating_sub(parsed_count);
    let critical_count = findings.values().filter(|finding| is_critical_lint(&finding.lint)).count() as u64;
    let keyed_status = if unkeyed_findings == 0 { "pass" } else { "fail" };
    let value = record("octet-structured-findings-v1", vec![
        string(crate::preserves_rail::OCTET_STRUCTURED_FINDINGS_SCHEMA),
        record("status", vec![string(&status_file.artifact_ref)]),
        record("summary", vec![string(&summary.artifact_ref)]),
        record("metadata", vec![
            record("config-hash", vec![string(&status.metadata.config_hash)]),
            record("profile-hash", vec![string(&status.metadata.profile_hash)]),
            record("tool", vec![string(format!(
                "{}@{}",
                status.metadata.tool_name, status.metadata.tool_version
            ))]),
        ]),
        record("counts", vec![
            record("total", vec![u64_value(status.total_findings)]),
            record("parsed", vec![u64_value(parsed_count)]),
            record("unkeyed", vec![u64_value(unkeyed_findings)]),
            record("critical", vec![u64_value(critical_count)]),
        ]),
        record("finding-keys", vec![sequence(findings.values().map(finding_entry_value).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("summary-index-stable-keys"), string(keyed_status)]),
            record("check", vec![string("artifact-ref-binding"), string("pass")]),
        ])]),
    ]);
    (value, unkeyed_findings)
}

fn parse_summary_findings(summary: &GateFile, status: &OctetStatusArtifact) -> (BTreeMap<String, FindingEntry>, u64) {
    let mut findings = BTreeMap::new();
    let mut parsed_count = 0u64;
    let mut is_parsing_index = false;
    for line in summary.text.lines() {
        let trimmed = line.trim();
        if trimmed == "Index:" {
            is_parsing_index = true;
            continue;
        }
        if !is_parsing_index || trimmed.is_empty() {
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 4 || !parts[0].starts_with('F') {
            continue;
        }
        parsed_count = parsed_count.saturating_add(1);
        let lint = parts[1].to_string();
        let crate_name = parts[2].to_string();
        let location = parts[3].to_string();
        let key =
            finding_key(&lint, &crate_name, &location, &status.metadata.config_hash, &status.metadata.tool_version);
        findings
            .entry(key.clone())
            .and_modify(|entry: &mut FindingEntry| entry.count = entry.count.saturating_add(1))
            .or_insert(FindingEntry {
                key,
                lint,
                crate_name,
                location,
                count: 1,
            });
    }
    (findings, parsed_count)
}

fn finding_key(lint: &str, crate_name: &str, location: &str, config_hash: &str, tool_version: &str) -> String {
    b3_full_hash(&format!(
        "lint={lint}\ncrate={crate_name}\nlocation={location}\nconfig={config_hash}\ntool=cargo-octet@{tool_version}\n"
    ))
}

fn finding_entry_value(finding: &FindingEntry) -> IOValue {
    record("finding-key", vec![
        string(&finding.key),
        string(&finding.lint),
        string(&finding.crate_name),
        string(&finding.location),
        u64_value(finding.count),
    ])
}

fn finding_count_delta(
    current: &BTreeMap<String, FindingEntry>,
    baseline: &BTreeMap<String, FindingEntry>,
    kind: DeltaKind,
) -> Vec<FindingEntry> {
    let mut delta = Vec::new();
    match kind {
        DeltaKind::NewOrIncreased => {
            for (key, current_entry) in current {
                let baseline_count = baseline.get(key).map(|entry| entry.count).unwrap_or(0);
                if current_entry.count > baseline_count {
                    let mut entry = current_entry.clone();
                    entry.count -= baseline_count;
                    delta.push(entry);
                }
            }
        }
        DeltaKind::Removed => {
            for (key, baseline_entry) in baseline {
                let current_count = current.get(key).map(|entry| entry.count).unwrap_or(0);
                if baseline_entry.count > current_count {
                    let mut entry = baseline_entry.clone();
                    entry.count -= current_count;
                    delta.push(entry);
                }
            }
        }
    }
    delta
}

fn finding_intersection(
    current: &BTreeMap<String, FindingEntry>,
    baseline: &BTreeMap<String, FindingEntry>,
) -> Vec<FindingEntry> {
    let mut intersection = Vec::new();
    for (key, current_entry) in current {
        if let Some(baseline_entry) = baseline.get(key) {
            let count = current_entry.count.min(baseline_entry.count);
            if count > 0 {
                let mut entry = current_entry.clone();
                entry.count = count;
                if !push_finding_bounded(&mut intersection, entry) {
                    break;
                }
            }
        }
    }
    intersection
}

fn critical_keys(findings: &BTreeMap<String, FindingEntry>) -> Vec<String> {
    findings
        .values()
        .filter(|finding| is_critical_lint(&finding.lint))
        .map(|finding| finding.key.clone())
        .collect()
}

fn is_critical_lint(lint: &str) -> bool {
    CRITICAL_LINTS.iter().any(|critical| critical == &lint)
}

fn source_snapshot_ref(run: &CurrentOctetRun) -> Result<String> {
    canonical_hash(&record("octet-source-snapshot-v1", vec![
        record("status", vec![string(&run.status_ref)]),
        record("summary", vec![string(&run.summary_ref)]),
        record("object-corpus", vec![string(&run.object_corpus_ref)]),
    ]))
}

fn record_finding_entries(value: &Value<IOValue>, label: &str) -> Result<BTreeMap<String, FindingEntry>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    let items = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} sequence")))?;
    ensure_count_at_most(items.len(), MAX_OCTET_FINDING_ENTRIES, label)?;
    let mut findings = BTreeMap::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let fields = item
            .collect_simple_record("finding-key", Some(5))
            .ok_or_else(|| MoltenError::invalid_harness("expected finding-key record"))?;
        let key = required_string(&fields[0], "finding key")?;
        insert_bounded(
            &mut findings,
            key.clone(),
            FindingEntry {
                key,
                lint: required_string(&fields[1], "finding lint")?,
                crate_name: required_string(&fields[2], "finding crate")?,
                location: required_string(&fields[3], "finding location")?,
                count: required_u64(&fields[4], "finding count")?,
            },
            MAX_OCTET_FINDING_ENTRIES,
            label,
        )?;
    }
    Ok(findings)
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    required_string(&record[0], label)
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    required_u64(&record[0], label)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} record")))?;
    let items = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected {label} sequence")))?;
    ensure_count_at_most(items.len(), MAX_OCTET_STRING_SEQUENCE, label)?;
    let mut strings = Vec::with_capacity(items.len());
    for item in items.iter() {
        strings.push(required_string(item, label)?);
    }
    Ok(strings)
}

fn require_schema(value: &Value<IOValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} schema mismatch: got {actual}, expected {expected}")))
    }
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IOValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn finding_counts(status: Option<&OctetStatusArtifact>, lint_counts: &BTreeMap<String, u64>) -> FindingCounts {
    let mut counts = FindingCounts::default();
    if let Some(status) = status {
        counts.total = status.total_findings;
        counts.warnings = status.warning_findings;
        counts.errors = status.error_findings;
        counts.autofixable = status.autofixable_findings;
    }
    counts.critical = CRITICAL_LINTS.iter().map(|name| lint_counts.get(*name).copied().unwrap_or(0)).sum();
    counts.uncovered = if counts.total == 0 { 0 } else { counts.total };
    counts
}

fn counts_value(counts: &FindingCounts) -> IOValue {
    record("counts", vec![
        record("findings", vec![u64_value(counts.total)]),
        record("warnings", vec![u64_value(counts.warnings)]),
        record("errors", vec![u64_value(counts.errors)]),
        record("autofixable", vec![u64_value(counts.autofixable)]),
        record("critical", vec![u64_value(counts.critical)]),
        record("uncovered", vec![u64_value(counts.uncovered)]),
    ])
}

fn checks_value(checks: &[GateCheck]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|check| record("check", vec![string(check.name), string(check.status)])).collect(),
    )])
}

fn optional_ref(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
}

fn insert_bounded<K: Ord, V>(values: &mut BTreeMap<K, V>, key: K, value: V, maximum: usize, label: &str) -> Result<()> {
    if !values.contains_key(&key) {
        let total = values
            .len()
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
        ensure_count_at_most(total, maximum, label)?;
    }
    values.insert(key, value);
    Ok(())
}

fn push_finding_bounded(values: &mut impl crate::bounded::VecSink<FindingEntry>, value: FindingEntry) -> bool {
    if values.item_count() >= MAX_OCTET_FINDING_ENTRIES {
        return false;
    }
    values.push_item(value);
    true
}

fn push_token_bounded(
    values: &mut impl crate::bounded::VecSink<String>,
    value: String,
) -> std::result::Result<(), String> {
    if values.item_count() >= MAX_OCTET_COMMAND_TOKENS {
        return Err("octet command scope argument count exceeds command token bound".to_string());
    }
    values.push_item(value);
    Ok(())
}

fn push_check(checks: &mut impl crate::bounded::VecSink<GateCheck>, name: &'static str, pass: bool) {
    checks.push_item(GateCheck {
        name,
        status: if pass { "pass" } else { "fail" },
    });
}

fn push_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) {
    if diagnostics.item_count() < MAX_DIAGNOSTICS {
        diagnostics.push_item(diagnostic);
    }
}

fn bytes_ref(bytes: &[u8]) -> String {
    content_ref_from_bytes(bytes)
}

fn b3_ref_from_bytes(bytes: &[u8]) -> Result<String> {
    let reference = content_ref_from_bytes(bytes);
    Ok(format!("b3:{}", content_ref_hex(&reference)?))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use super::*;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    #[test]
    fn strict_profile_denies_warning_only_status() {
        let dir = temp_dir("warning-only");
        write_artifacts(&dir, warning_status_json(), warning_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("warning-only")));
        assert!(to_text(&evaluation.receipt_value).expect("receipt text").contains("octet-gate-receipt-v1"));
    }

    #[test]
    fn strict_profile_passes_clean_status_with_required_artifacts() {
        let dir = temp_dir("clean");
        write_artifacts(&dir, clean_status_json(), clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "pass");
        assert!(evaluation.diagnostics.is_empty());
        assert!(to_text(&evaluation.receipt_value).expect("receipt text").contains("fingerprint <some \"blake3:"));
    }

    #[test]
    fn source_gate_validation_accepts_clean_strict_receipt() {
        let gate = synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate fixture");
        let validation = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: test_ref("node-config"),
            gate_receipt_value: Some(gate),
            source_scope: Vec::new(),
        })
        .expect("validate source gate");

        assert_eq!(validation.decision, "pass");
        assert!(to_text(&validation.value).expect("validation text").contains("octet-source-gate-validation-v1"));
        assert_eq!(crate::ledger::artifact_kind(&validation.value), "octet-source-gate-validation");
    }

    #[test]
    fn source_gate_validation_denies_warning_or_missing_receipts() {
        let dir = temp_dir("source-gate-warning");
        write_artifacts(&dir, warning_status_json(), warning_summary(), object_corpus_json());
        let gate = evaluate_octet_gate(&input(&dir)).expect("warning gate");
        let denied = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "job-remote-admission".to_string(),
            subject_ref: test_ref("job"),
            gate_receipt_value: Some(gate.receipt_value),
            source_scope: Vec::new(),
        })
        .expect("validate warning gate");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("decision is deny")));

        let missing = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "upgrade-plan".to_string(),
            subject_ref: test_ref("plan"),
            gate_receipt_value: None,
            source_scope: Vec::new(),
        })
        .expect("validate missing gate");
        assert_eq!(missing.decision, "deny");
    }

    #[test]
    fn source_gate_validation_denies_stale_or_tampered_receipt() {
        let stale = octet_gate_receipt_value(OctetGateReceiptInput {
            decision: "pass",
            policy_ref: &test_ref("policy"),
            command_ref: Some("blake3:test-command"),
            status_ref: Some("blake3:test-status"),
            summary_ref: Some("blake3:test-summary"),
            structured_findings_ref: Some("blake3:test-findings"),
            object_corpus_ref: Some("blake3:test-object-corpus"),
            fingerprint_evidence_ref: Some("blake3:test-fingerprint"),
            config_hash: Some("b3:stale-config"),
            profile_hash: Some("b3:stale-profile"),
            toolchain: Some("toolchain"),
            counts: &FindingCounts::default(),
            diagnostics: &[],
            checks: &[
                GateCheck {
                    name: "profile-supported",
                    status: "pass",
                },
                GateCheck {
                    name: "strict-status-clean",
                    status: "pass",
                },
                GateCheck {
                    name: "no-critical-findings",
                    status: "pass",
                },
                GateCheck {
                    name: "object-corpus-critical-paths",
                    status: "pass",
                },
                GateCheck {
                    name: "object-corpus-fingerprint",
                    status: "pass",
                },
                GateCheck {
                    name: "fingerprint-evidence-bound",
                    status: "pass",
                },
            ],
        });
        let stale_validation = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: test_ref("node"),
            gate_receipt_value: Some(stale),
            source_scope: Vec::new(),
        })
        .expect("validate stale gate");
        assert_eq!(stale_validation.decision, "deny");
        assert!(stale_validation.diagnostics.iter().any(|diagnostic| diagnostic.contains("config hash")));

        let tampered = parse_text(
            &to_text(&synthetic_clean_octet_gate_receipt_for_tests().expect("clean fixture"))
                .expect("clean text")
                .replace("blake3:test-fingerprint", "not-a-ref"),
        )
        .expect("tampered parse");
        let tampered_validation = validate_octet_source_gate(&OctetSourceGateValidationInput {
            consumer: "node-startup".to_string(),
            subject_ref: test_ref("node"),
            gate_receipt_value: Some(tampered),
            source_scope: Vec::new(),
        })
        .expect("validate tampered gate");
        assert_eq!(tampered_validation.decision, "deny");
    }

    #[test]
    fn missing_status_denies_with_receipt() {
        let dir = temp_dir("missing-status");
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join(COMMAND_NAME), "cargo octet check --artifact-dir target/octet\n").expect("write command");
        fs::write(dir.join(SUMMARY_NAME), clean_summary()).expect("write summary");
        fs::write(dir.join(OBJECT_CORPUS_RECEIPT_NAME), object_corpus_json()).expect("write object corpus");

        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains(STATUS_NAME)));
    }

    #[test]
    fn malformed_status_denies_with_receipt() {
        let dir = temp_dir("malformed-status");
        write_artifacts(&dir, "{", clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("malformed status.json")));
    }

    #[test]
    fn unsupported_tool_version_denies_clean_status() {
        let dir = temp_dir("unsupported-tool-version");
        write_artifacts(&dir, unsupported_tool_version_status_json(), clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(
            evaluation
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("unsupported cargo-octet version"))
        );
    }

    #[test]
    fn missing_object_corpus_denies_clean_status() {
        let dir = temp_dir("missing-object-corpus");
        fs::create_dir_all(&dir).expect("create temp dir");
        fs::write(dir.join(COMMAND_NAME), "cargo octet check --artifact-dir target/octet\n").expect("write command");
        fs::write(dir.join(STATUS_NAME), clean_status_json()).expect("write status");
        fs::write(dir.join(SUMMARY_NAME), clean_summary()).expect("write summary");

        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains(OBJECT_CORPUS_RECEIPT_NAME)));
    }

    #[test]
    fn stale_status_metadata_denies_clean_status() {
        let dir = temp_dir("stale-status");
        write_artifacts(&dir, stale_status_json(), clean_summary(), object_corpus_json());
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale octet config hash")));
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale octet profile hash")));
    }

    #[test]
    fn malformed_object_corpus_denies() {
        let dir = temp_dir("bad-object-corpus");
        write_artifacts(&dir, clean_status_json(), clean_summary(), r#"{"schema":"wrong"}"#);
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("object corpus")));
    }

    #[test]
    fn object_corpus_without_fingerprint_or_critical_paths_denies() {
        let dir = temp_dir("incomplete-object-corpus");
        write_artifacts(
            &dir,
            clean_status_json(),
            clean_summary(),
            r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":1}"#,
        );
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("object_set_hash fingerprint")));
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("required critical paths")));
    }

    #[test]
    fn noncanonical_command_denies() {
        let dir = temp_dir("bad-command");
        write_artifacts(&dir, clean_status_json(), clean_summary(), object_corpus_json());
        fs::write(dir.join(COMMAND_NAME), "cargo check\n").expect("write bad command");
        let evaluation = evaluate_octet_gate(&input(&dir)).expect("evaluate octet gate");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("noncanonical")));
    }

    #[test]
    fn imports_raw_octet_artifacts_and_derived_evidence_to_ledger() {
        let dir = temp_dir("artifact-ledger-import");
        let ledger_root = dir.join("ledger");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let imported = import_octet_artifacts_to_ledger(&OctetArtifactLedgerInput {
            artifacts_dir: dir.clone(),
            ledger_root: ledger_root.clone(),
        })
        .expect("import octet artifacts");

        assert_eq!(imported.decision, "pass");
        let kinds = ledger::list_artifacts(&ledger_root)
            .expect("list ledger")
            .into_iter()
            .map(|entry| {
                let value = ledger::read_artifact(&ledger_root, &entry.artifact_ref).expect("read ledger artifact");
                ledger::artifact_kind(&value).to_string()
            })
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "octet-command-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-status-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-summary-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-object-corpus-artifact"));
        assert!(kinds.iter().any(|kind| kind == "octet-structured-findings"));
        assert!(kinds.iter().any(|kind| kind == "octet-fingerprint-evidence"));
    }

    #[test]
    fn gate_receipt_binds_structured_findings_ref_to_summary_index() {
        let dir = temp_dir("structured-findings-ref");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let first = evaluate_octet_gate(&input(&dir)).expect("first gate");
        let changed_summary = "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  function_length 1\n\nIndex:\n  F1 function_length molten src/changed.rs:99\n";
        write_artifacts(&dir, noncritical_status_json(1), changed_summary, object_corpus_json());
        let second = evaluate_octet_gate(&input(&dir)).expect("second gate");

        assert_ne!(receipt_findings_ref(&first.receipt_value), receipt_findings_ref(&second.receipt_value));
    }

    #[test]
    fn baseline_check_passes_identical_noncritical_warning() {
        let dir = temp_dir("baseline-pass");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "pass");
    }

    #[test]
    fn baseline_check_denies_new_warning() {
        let dir = temp_dir("baseline-new-warning");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        write_artifacts(&dir, noncritical_status_json(2), noncritical_summary_two(), object_corpus_json());
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("new or increased")));
    }

    #[test]
    fn baseline_check_allows_removed_warning() {
        let dir = temp_dir("baseline-removed-warning");
        write_artifacts(&dir, noncritical_status_json(2), noncritical_summary_two(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "pass");
        let receipt_text = to_text(&evaluation.receipt_value).expect("receipt text");
        assert!(receipt_text.contains("removed-findings"));
        assert!(receipt_text.contains("bool_naming"));
    }

    #[test]
    fn baseline_check_denies_expired_baseline() {
        let dir = temp_dir("baseline-expired");
        write_artifacts(&dir, noncritical_status_json(1), noncritical_summary_one(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "2026-01-01T00:00:00Z")).expect("write baseline");
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("expired")));
    }

    #[test]
    fn baseline_check_denies_unreviewed_critical_warning() {
        let dir = temp_dir("baseline-critical");
        write_artifacts(&dir, critical_status_json(), critical_summary(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        let evaluation =
            check_octet_warning_baseline(&baseline_check_input(&dir, &baseline.baseline_value, "2026-05-31T00:00:00Z"))
                .expect("check baseline");

        assert_eq!(evaluation.decision, "deny");
        assert!(evaluation.diagnostics.iter().any(|diagnostic| diagnostic.contains("unreviewed critical")));
    }

    #[test]
    fn baseline_check_accepts_exact_review_manifest_for_critical_warning() {
        let dir = temp_dir("baseline-critical-reviewed");
        write_artifacts(&dir, critical_status_json(), critical_summary(), object_corpus_json());
        let baseline =
            build_octet_warning_baseline(&baseline_input(&dir, "9999-01-01T00:00:00Z")).expect("write baseline");
        let parsed = parse_warning_baseline(&baseline.baseline_value).expect("parse baseline");
        let finding_key = parsed.findings.keys().next().expect("finding key").clone();
        let review = build_octet_review_manifest(&OctetReviewManifestInput {
            profile: QUARANTINE_PROFILE.to_string(),
            expires_at: "9999-01-01T00:00:00Z".to_string(),
            finding_keys: vec![finding_key],
            rationale: "temporary test review".to_string(),
        })
        .expect("build review");
        let evaluation = check_octet_warning_baseline(&baseline_check_input_with_reviews(
            &dir,
            &baseline.baseline_value,
            "2026-05-31T00:00:00Z",
            vec![review.review_value],
        ))
        .expect("check baseline");

        assert_eq!(evaluation.decision, "pass");
        let receipt_text = to_text(&evaluation.receipt_value).expect("receipt text");
        assert!(receipt_text.contains("review-refs"));
    }

    fn input(dir: &Path) -> OctetGateInput {
        OctetGateInput {
            artifacts_dir: dir.to_path_buf(),
            profile: STRICT_PROFILE.to_string(),
        }
    }

    fn receipt_findings_ref(value: &IOValue) -> String {
        let fields = value.collect_simple_record("octet-gate-receipt-v1", Some(15)).expect("gate receipt");
        let findings_value = value_to_iovalue(&fields[6]);
        let findings = findings_value.collect_simple_record("findings", Some(1)).expect("findings");
        let some = findings[0].collect_simple_record("some", Some(1)).expect("some findings ref");
        some[0].as_string().expect("findings ref string").into_owned()
    }

    fn baseline_input(dir: &Path, expires_at: &str) -> OctetWarningBaselineInput {
        OctetWarningBaselineInput {
            artifacts_dir: dir.to_path_buf(),
            created_at: "2026-05-31T00:00:00Z".to_string(),
            expires_at: expires_at.to_string(),
            target_next: None,
        }
    }

    fn baseline_check_input(dir: &Path, baseline_value: &IOValue, as_of: &str) -> OctetBaselineCheckInput {
        baseline_check_input_with_reviews(dir, baseline_value, as_of, Vec::new())
    }

    fn baseline_check_input_with_reviews(
        dir: &Path,
        baseline_value: &IOValue,
        as_of: &str,
        review_values: Vec<IOValue>,
    ) -> OctetBaselineCheckInput {
        OctetBaselineCheckInput {
            artifacts_dir: dir.to_path_buf(),
            baseline_value: baseline_value.clone(),
            profile: QUARANTINE_PROFILE.to_string(),
            as_of: as_of.to_string(),
            review_values,
        }
    }

    const COMMAND_TEXT: &str = "cargo octet check --artifact-dir target/octet";

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("octet-test-ref", vec![string(label)])).expect("test ref")
    }

    fn write_artifacts(dir: &Path, status: impl AsRef<str>, summary: &str, object_corpus: &str) {
        fs::create_dir_all(dir).expect("create temp dir");
        fs::write(dir.join(COMMAND_NAME), format!("{COMMAND_TEXT}\n")).expect("write command");
        fs::write(dir.join(STATUS_NAME), status.as_ref()).expect("write status");
        fs::write(dir.join(SUMMARY_NAME), summary).expect("write summary");
        fs::write(dir.join(OBJECT_CORPUS_RECEIPT_NAME), object_corpus).expect("write object corpus");
    }

    fn warning_status_json() -> String {
        status_json("warning-only", 3, 3, 0, 1, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn clean_status_json() -> String {
        status_json("clean", 0, 0, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn noncritical_status_json(total: u64) -> String {
        status_json("warning-only", total, total, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn critical_status_json() -> String {
        status_json("warning-only", 1, 1, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, current_metadata())
    }

    fn stale_status_json() -> String {
        status_json("clean", 0, 0, 0, 0, SUPPORTED_OCTET_TOOL_VERSION, ExpectedMetadata {
            config_hash: "b3:stale-config".to_string(),
            profile_hash: "b3:stale-profile".to_string(),
        })
    }

    fn unsupported_tool_version_status_json() -> String {
        status_json("clean", 0, 0, 0, 0, "9.9.9", current_metadata())
    }

    fn current_metadata() -> ExpectedMetadata {
        expected_metadata_for_command(COMMAND_TEXT).expect("compute current octet metadata")
    }

    fn status_json(
        status: &str,
        total_findings: u64,
        warning_findings: u64,
        error_findings: u64,
        autofixable_findings: u64,
        tool_version: &str,
        metadata: ExpectedMetadata,
    ) -> String {
        format!(
            r#"{{
  "status": "{status}",
  "exit_code": 0,
  "output_format": "human",
  "metadata": {{
    "tool_name": "cargo-octet",
    "tool_version": "{tool_version}",
    "rustc_version": "rustc 1.96.0-nightly",
    "toolchain": "nightly-2026-03-21-x86_64-unknown-linux-gnu",
    "profile_hash": "{}",
    "config_hash": "{}"
  }},
  "total_findings": {total_findings},
  "warning_findings": {warning_findings},
  "error_findings": {error_findings},
  "autofixable_findings": {autofixable_findings},
  "cargo_process_exit": {{"classification": "success", "code": 0}}
}}"#,
            metadata.profile_hash, metadata.config_hash
        )
    }

    fn warning_summary() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 3\nWarnings: 3\nErrors: 0\n\nBy lint:\n  no_unwrap 1\n  function_length 2\n\nIndex:\n"
    }

    fn clean_summary() -> &'static str {
        "--- octet summary ---\nStatus: clean\nFindings: 0\nWarnings: 0\nErrors: 0\n\nBy lint:\n\nIndex:\n"
    }

    fn noncritical_summary_one() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  function_length 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n"
    }

    fn noncritical_summary_two() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 2\nWarnings: 2\nErrors: 0\n\nBy lint:\n  function_length 1\n  bool_naming 1\n\nIndex:\n  F1 function_length molten src/example.rs:10\n  F2 bool_naming molten src/example.rs:20\n"
    }

    fn critical_summary() -> &'static str {
        "--- octet summary ---\nStatus: warning-only\nFindings: 1\nWarnings: 1\nErrors: 0\n\nBy lint:\n  no_unwrap 1\n\nIndex:\n  F1 no_unwrap molten src/example.rs:30\n"
    }

    fn object_corpus_json() -> &'static str {
        r#"{"schema":"octet.function-object-corpus-receipt.v1","schema_version":1,"object_count":3,"source_paths":["src/job/dag.rs","src/main.rs","src/node/runtime.rs"],"object_set_hash":"b3:test-object-set","pure_cache_blocked_count":3}"#
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-octet-gate-{label}-{}-{nanos}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        dir
    }
}
