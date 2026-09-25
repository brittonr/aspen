pub type IoValue = preserves::IOValue;

pub const FIXTURE_METADATA_KIND: &str = "cluster-harness-fixture-metadata";
pub const COMMAND_PLAN_KIND: &str = "cluster-harness-command-plan";
pub const LOCAL_PLAN_KIND: &str = "local-multiprocess-plan";
pub const LOCAL_EXECUTABLE_RUN_KIND: &str = "local-multiprocess-executable-run";
pub const CLUSTER_LIFECYCLE_KIND: &str = "cluster-lifecycle-run";
pub const DRIFT_SUMMARY_KIND: &str = "cluster-harness-drift-summary";
pub const CHILD_PROCESS_KIND: &str = "cluster-harness-child-process";
pub const CLEANUP_KIND: &str = "cluster-harness-cleanup";
pub const CLUSTER_RUN_KIND: &str = "cluster-harness-run";
pub const VERIFICATION_KIND: &str = "cluster-run-verification";
pub const DIAGNOSTIC_LOG_KIND: &str = "cluster-harness-diagnostic-log";

const FIXTURE_METADATA_SCHEMA: &str = "molten.testing.cluster-harness-fixture-metadata.v1";
const COMMAND_PLAN_SCHEMA: &str = "molten.testing.cluster-harness-command-plan.v1";
const DRIFT_SUMMARY_SCHEMA: &str = "molten.testing.cluster-harness-drift-summary.v1";
const CHILD_PROCESS_SCHEMA: &str = "molten.testing.cluster-harness-child-process.v1";
const CLEANUP_SCHEMA: &str = "molten.testing.cluster-harness-cleanup.v1";
const CLUSTER_RUN_SCHEMA: &str = "molten.testing.cluster-harness-run.v1";
const VERIFICATION_SCHEMA: &str = "molten.testing.cluster-run-verification.v1";
const PASS_DECISION: &str = "pass";
const DENY_DECISION: &str = "deny";
const NONE_VALUE: &str = "none";
const DIAGNOSTIC_ONLY: &str = "diagnostic-only";
const CLUSTER_RUN_RECORD_ARITY: u64 = 16;
const LOCAL_PLAN_RECORD_ARITY: u64 = 10;
const LOCAL_EXECUTABLE_RUN_RECORD_ARITY: u64 = 15;
const CLUSTER_LIFECYCLE_RECORD_ARITY: u64 = 12;
const CHILD_PROCESS_RECORD_ARITY: u64 = 10;
const CLEANUP_RECORD_ARITY: u64 = 9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHarnessChildProcessInput {
    pub node_id: String,
    pub phase: String,
    pub command_profile_ref: String,
    pub diagnostic_log_ref: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub orphaned: bool,
    pub succeeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHarnessCleanupInput {
    pub child_process_refs: Vec<String>,
    pub stopped_node_ids: Vec<String>,
    pub orphaned_processes: Vec<String>,
    pub removed_ticket_refs: Vec<String>,
    pub remaining_ticket_paths: Vec<String>,
    pub cleanup_succeeded: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHarnessParentInput {
    pub fixture_ref: String,
    pub command_plan_ref: String,
    pub local_plan_ref: String,
    pub local_run_ref: String,
    pub lifecycle_ref: String,
    pub drift_summary_ref: String,
    pub cleanup_ref: String,
    pub child_receipt_refs: Vec<String>,
    pub diagnostic_log_refs: Vec<String>,
    pub observed_artifact_kinds: Vec<String>,
    pub required_artifact_kinds: Vec<String>,
    pub unsupported_pass_claim: bool,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterHarnessParentReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterRunVerificationReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub verification_ref: String,
    pub value: IoValue,
}

// r[impl molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
pub fn fixture_metadata_value(
    fixture_ref: &str,
    node_ids: &[String],
    caveats: &[String],
) -> crate::error::Result<IoValue> {
    crate::preserves_rail::validate_content_ref(fixture_ref)?;
    validate_non_empty_strings("fixture node", node_ids)?;
    validate_non_empty_strings("fixture caveat", caveats)?;
    Ok(crate::preserves_rail::record("cluster-harness-fixture-metadata-v1", vec![
        crate::preserves_rail::string(FIXTURE_METADATA_SCHEMA),
        crate::preserves_rail::record("fixture", vec![crate::preserves_rail::string(fixture_ref)]),
        crate::preserves_rail::record("source-kind", vec![crate::preserves_rail::string("cluster-manifest-fixture")]),
        crate::preserves_rail::record("nodes", vec![strings_sequence(node_ids)]),
        crate::preserves_rail::record("caveats", vec![strings_sequence(caveats)]),
        crate::preserves_rail::checks_value(&[
            ("fixture-content-addressed", PASS_DECISION),
            ("fixture-is-source-of-truth", PASS_DECISION),
        ]),
    ]))
}

// r[impl molten.testing.fixture_driven_cluster_execution.fixture_source_of_truth]
pub fn command_plan_value(
    fixture_ref: &str,
    node_ids: &[String],
    child_timeout_ms: u64,
    expected_artifact_kinds: &[String],
) -> crate::error::Result<IoValue> {
    crate::preserves_rail::validate_content_ref(fixture_ref)?;
    if child_timeout_ms == 0 {
        return Err(crate::error::MoltenError::invalid_harness("cluster harness child timeout must be positive"));
    }
    validate_non_empty_strings("command plan node", node_ids)?;
    validate_non_empty_strings("expected artifact kind", expected_artifact_kinds)?;
    let phases = ["init", "start", "workflow", "status", "stop"]
        .into_iter()
        .map(crate::preserves_rail::string)
        .collect();
    Ok(crate::preserves_rail::record("cluster-harness-command-plan-v1", vec![
        crate::preserves_rail::string(COMMAND_PLAN_SCHEMA),
        crate::preserves_rail::record("fixture", vec![crate::preserves_rail::string(fixture_ref)]),
        crate::preserves_rail::record("nodes", vec![strings_sequence(node_ids)]),
        crate::preserves_rail::record("phases", vec![crate::preserves_rail::sequence(phases)]),
        crate::preserves_rail::record("child-timeout-ms", vec![crate::preserves_rail::u64_value(child_timeout_ms)]),
        crate::preserves_rail::record("expected-artifact-kinds", vec![strings_sequence(expected_artifact_kinds)]),
        crate::preserves_rail::checks_value(&[
            ("timeout-explicit", PASS_DECISION),
            ("init-start-status-stop-explicit", PASS_DECISION),
        ]),
    ]))
}

pub fn unavailable_cluster_lifecycle_value(
    fixture_ref: &str,
    node_ids: &[String],
    diagnostics: &[String],
    caveats: &[String],
) -> crate::error::Result<IoValue> {
    crate::preserves_rail::validate_content_ref(fixture_ref)?;
    validate_non_empty_strings("unavailable lifecycle node", node_ids)?;
    validate_non_empty_strings("unavailable lifecycle diagnostic", diagnostics)?;
    validate_non_empty_strings("unavailable lifecycle caveat", caveats)?;
    Ok(crate::preserves_rail::record("cluster-lifecycle-run-v1", vec![
        crate::preserves_rail::string("molten.testing.cluster-lifecycle-run.v1"),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(DENY_DECISION)]),
        crate::preserves_rail::record("workflow", vec![crate::preserves_rail::string("receipt-first-cluster-harness")]),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(fixture_ref)]),
        crate::preserves_rail::record("nodes", vec![strings_sequence(node_ids)]),
        crate::preserves_rail::record("phases", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("node-summaries", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("already-running", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("stop-order", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(diagnostics)]),
        crate::preserves_rail::record("caveats", vec![strings_sequence(caveats)]),
        crate::preserves_rail::checks_value(&[
            ("lifecycle-unavailable-is-not-pass", DENY_DECISION),
            ("stdout-not-evidence", PASS_DECISION),
        ]),
    ]))
}

// r[impl molten.testing.receipt_first_cluster_harness.run_artifact_directory]
pub fn drift_summary_value(summary: &crate::drift_core::EvidenceSummary) -> crate::error::Result<IoValue> {
    if summary.workflow.trim().is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cluster drift summary workflow must be non-empty"));
    }
    let fields = summary
        .fields
        .iter()
        .map(|field| {
            crate::preserves_rail::record("field", vec![
                crate::preserves_rail::record("path", vec![crate::preserves_rail::string(&field.path)]),
                crate::preserves_rail::record("value", vec![crate::preserves_rail::string(&field.value)]),
                crate::preserves_rail::record("is-ref", vec![crate::preserves_rail::bool_value(field.is_ref)]),
            ])
        })
        .collect();
    let expected_equalities = vec![
        crate::preserves_rail::string("fixture-manifest-ref"),
        crate::preserves_rail::string("phase-decisions"),
        crate::preserves_rail::string("ordered-membership"),
        crate::preserves_rail::string("reverse-stop-order"),
    ];
    let allowed_variances = vec![
        crate::preserves_rail::string("node-config-and-identity-refs"),
        crate::preserves_rail::string("node-lifecycle-receipt-refs"),
        crate::preserves_rail::string("temporary-state-roots"),
        crate::preserves_rail::string("diagnostic-logs"),
    ];
    Ok(crate::preserves_rail::record("cluster-harness-drift-summary-v1", vec![
        crate::preserves_rail::string(DRIFT_SUMMARY_SCHEMA),
        crate::preserves_rail::record("workflow", vec![crate::preserves_rail::string(&summary.workflow)]),
        crate::preserves_rail::record("fields", vec![crate::preserves_rail::sequence(fields)]),
        crate::preserves_rail::record("expected-equalities", vec![crate::preserves_rail::sequence(
            expected_equalities,
        )]),
        crate::preserves_rail::record("allowed-variances", vec![crate::preserves_rail::sequence(allowed_variances)]),
        crate::preserves_rail::record("evidence-scope", vec![crate::preserves_rail::string(DIAGNOSTIC_ONLY)]),
        crate::preserves_rail::checks_value(&[
            ("canonical-receipt-summary", PASS_DECISION),
            ("logs-not-equality-evidence", PASS_DECISION),
        ]),
    ]))
}

// r[impl molten.testing.local_multiprocess_cluster_tier.middle_tier]
// r[impl molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
pub fn child_process_value(input: &ClusterHarnessChildProcessInput) -> crate::error::Result<IoValue> {
    if input.node_id.trim().is_empty() || input.phase.trim().is_empty() {
        return Err(crate::error::MoltenError::invalid_harness("cluster child process requires node and phase"));
    }
    crate::preserves_rail::validate_content_ref(&input.command_profile_ref)?;
    crate::preserves_rail::validate_content_ref(&input.diagnostic_log_ref)?;
    let decision = if input.succeeded && !input.timed_out && !input.orphaned {
        PASS_DECISION
    } else {
        DENY_DECISION
    };
    let exit_code = input.exit_code.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |code| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(code.to_string())]),
    );
    Ok(crate::preserves_rail::record("cluster-harness-child-process-v1", vec![
        crate::preserves_rail::string(CHILD_PROCESS_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.node_id)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(&input.phase)]),
        crate::preserves_rail::record("command-profile", vec![crate::preserves_rail::string(
            &input.command_profile_ref,
        )]),
        crate::preserves_rail::record("diagnostic-log", vec![crate::preserves_rail::string(&input.diagnostic_log_ref)]),
        crate::preserves_rail::record("exit-code", vec![exit_code]),
        crate::preserves_rail::record("timed-out", vec![crate::preserves_rail::bool_value(input.timed_out)]),
        crate::preserves_rail::record("orphaned", vec![crate::preserves_rail::bool_value(input.orphaned)]),
        crate::preserves_rail::checks_value(&[
            ("child-exited-successfully", status(input.succeeded)),
            ("child-within-timeout", status(!input.timed_out)),
            ("child-reaped", status(!input.orphaned)),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]))
}

// r[impl molten.testing.local_multiprocess_cluster_tier.cleanup_negatives]
pub fn cleanup_value(input: &ClusterHarnessCleanupInput) -> crate::error::Result<IoValue> {
    validate_refs("cleanup child process", &input.child_process_refs)?;
    validate_refs("cleanup removed ticket", &input.removed_ticket_refs)?;
    validate_non_empty_strings("cleanup caveat", &input.caveats)?;
    let decision =
        if input.cleanup_succeeded && input.orphaned_processes.is_empty() && input.remaining_ticket_paths.is_empty() {
            PASS_DECISION
        } else {
            DENY_DECISION
        };
    Ok(crate::preserves_rail::record("cluster-harness-cleanup-v1", vec![
        crate::preserves_rail::string(CLEANUP_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision)]),
        crate::preserves_rail::record("child-processes", vec![refs_sequence(&input.child_process_refs)]),
        crate::preserves_rail::record("stopped-nodes", vec![strings_sequence(&input.stopped_node_ids)]),
        crate::preserves_rail::record("orphaned-processes", vec![strings_sequence(&input.orphaned_processes)]),
        crate::preserves_rail::record("removed-tickets", vec![refs_sequence(&input.removed_ticket_refs)]),
        crate::preserves_rail::record("remaining-ticket-paths", vec![strings_sequence(&input.remaining_ticket_paths)]),
        crate::preserves_rail::record("caveats", vec![strings_sequence(&input.caveats)]),
        crate::preserves_rail::checks_value(&[
            ("no-orphaned-processes", status(input.orphaned_processes.is_empty())),
            ("no-stale-ticket-files", status(input.remaining_ticket_paths.is_empty())),
            ("cleanup-completed", status(input.cleanup_succeeded)),
        ]),
    ]))
}
