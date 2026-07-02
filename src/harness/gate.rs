type Cow<'a, T> = std::borrow::Cow<'a, T>;
type IoValue = preserves::IOValue;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type Record<T> = preserves::Record<T>;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const DETERMINISTIC_REPLAY_VERIFY_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA;
const EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA;
const HARNESS_GATE_RECEIPT_SCHEMA: &str = crate::preserves_rail::HARNESS_GATE_RECEIPT_SCHEMA;
const HARNESS_OBSERVATION_SCHEMA: &str = crate::preserves_rail::HARNESS_OBSERVATION_SCHEMA;
const HARNESS_REPORT_SCHEMA: &str = crate::preserves_rail::HARNESS_REPORT_SCHEMA;
const HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA: &str = crate::preserves_rail::HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const PASS_CHECKS: &[&str] = &[
    "report-schema",
    "effect-log",
    "budget",
    "explicit-budget-fixture",
    "no-default-resource-policy",
    "resource-policy-preflight",
    "nickel-resource-policy",
    "nickel-resource-export",
    "basalt-resource-receipt",
    "budget-usage-binding",
    "actor-registry",
    "explicit-actor-registry",
    "no-inferred-actors",
    "executor-boundary",
    "executor-preflight",
    "executor-kind-binding",
    "allowed-hostcall-binding",
    "no-unsupported-executor-fallback",
    "executor-conformance-suite-binding",
    "cross-kind-hostcall-conformance",
    "executor-execution-receipt-binding",
    "executor-output-ref-binding",
    "steel-executor-preflight",
    "steel-review-receipt-binding",
    "steel-vm-execution",
    "steel-resource-bounds",
    "adapter-executor-preflight",
    "remote-proxy-preflight",
    "wasm-executor-preflight",
    "wasm-inspection-receipt-binding",
    "wasm-execution-receipt-binding",
    "wasmtime-no-wasi",
    "wasm-fuel-memory-bounds",
    "wasm-abi-byte-bounds",
    "wasm-guest-memory-bounds",
    "wasm-preserves-abi-ready",
    "executor-hostcall-boundary",
    "hostcall-admission-binding",
    "hostcall-replay",
    "effect-handler-binding",
    "effect-handle-binding",
    "handle-not-authority",
    "hostcall-handle-replay",
    "no-ambient-executor-io",
    "admission-policy",
    "policy-preflight",
    "nickel-static-policy",
    "nickel-policy-source",
    "nickel-export-normalization",
    "basalt-policy-gate",
    "basalt-preflight-receipt",
    "basalt-receipt-binding",
    "steel-predicate-review",
    "explicit-capability-fixture",
    "no-implicit-authority",
    "capability-context",
    "capability-grants",
    "basalt-authority-receipt",
    "capability-proofset-binding",
    "grant-ref-binding",
    "deny-without-capability",
    "authority-ref-binding",
    "admission-decisions",
    "deny-rollback",
    "denied-effect-suppression",
    "runtime-predicate-receipts",
    "assertion-visibility-predicate",
    "turn-commit-rollback-predicate",
    "observe-delivery-predicate",
    "chain-continuity",
    "chain-anchor-descent",
    "chain-checkpoint-freshness",
    "chain-predicate-receipts",
    "turn-journal-chains",
    "turn-journal-input-binding",
    "turn-journal-admission-binding",
    "turn-journal-state-binding",
    "turn-journal-no-global-head",
    "deterministic-replay",
];

const REQUIRED_KINDS: &[&str] = &[
    "executor-preflights",
    "executor-execution-receipts",
    "runtime-predicate-receipts",
    "policy",
    "policy-gate",
    "policy-nickel-source",
    "policy-nickel-export",
    "policy-basalt-preflight",
    "budget",
    "budget-gate",
    "budget-nickel-source",
    "budget-nickel-export",
    "budget-basalt-preflight",
    "capabilities",
    "capability-gate",
    "capability-authority-preflight",
    "ucan-proofset",
];

const REDACTION_KINDS: &[&str] = &["redaction-policy", "redaction-gate"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub artifact_kind: String,
    pub artifact_ref: String,
    pub report_ref: String,
    pub suite_ref: String,
    pub initial_state_hash: String,
    pub final_state_hash: String,
    pub replay_actual_report_ref: String,
    pub deterministic_replay_verify_ref: String,
    pub deterministic_replay_verify_value: IoValue,
    pub executor_preflights_ref: String,
    pub executor_execution_receipts_ref: String,
    pub runtime_predicate_receipts_ref: String,
    pub policy_ref: String,
    pub policy_gate_ref: String,
    pub policy_nickel_source_ref: String,
    pub policy_nickel_export_ref: String,
    pub policy_basalt_preflight_ref: String,
    pub budget_ref: String,
    pub budget_gate_ref: String,
    pub budget_nickel_source_ref: String,
    pub budget_nickel_export_ref: String,
    pub budget_basalt_preflight_ref: String,
    pub capability_ref: String,
    pub capability_gate_ref: String,
    pub capability_authority_preflight_ref: String,
    pub capability_proofset_ref: String,
    pub redaction_policy_ref: Option<String>,
    pub redaction_gate_ref: Option<String>,
    pub observations: u64,
    pub actors: Vec<super::schema::ActorDecl>,
    pub budget: super::schema::BudgetEvidence,
    pub chain_evidence: ChainEvidence,
    pub turn_journals: TurnJournalEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainEvidence {
    pub link_ref: String,
    pub anchor_ref: String,
    pub verify_receipt_ref: String,
    pub checkpoint_ref: String,
    pub range_predicate_ref: String,
    pub predicate_receipt_refs: Vec<String>,
    pub link_value: IoValue,
    pub anchor_value: IoValue,
    pub verify_receipt_value: IoValue,
    pub checkpoint_value: IoValue,
    pub predicate_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnJournalEvidence {
    pub aggregate_ref: String,
    pub journals: Vec<TurnJournalChainEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnJournalChainEvidence {
    pub actor_id: String,
    pub link_refs: Vec<String>,
    pub payload_refs: Vec<String>,
    pub verify_receipt_ref: String,
    pub predicate_receipt_refs: Vec<String>,
    pub link_values: Vec<IoValue>,
    pub verify_receipt_value: IoValue,
    pub predicate_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub decision: String,
    pub artifact_kind: String,
    pub artifact_ref: String,
    pub report_ref: String,
    pub suite_ref: String,
    pub final_state_hash: String,
    pub checks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproVerifyReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub bundle_ref: String,
    pub report_ref: String,
    pub suite_ref: String,
    pub gate_receipt_ref: String,
    pub checks: Vec<String>,
}

pub fn check_value(value: &IoValue) -> Result<Check> {
    if value.collect_simple_record("harness-failure-v1", None).is_some() {
        let failure = super::schema::parse_failure(value)?;
        return Err(MoltenError::invalid_harness(format!(
            "harness failure artifact {} phase={} kind={} cannot satisfy pass evidence gate",
            failure.failure_ref, failure.phase, failure.kind
        )));
    }

    if value.collect_simple_record("harness-report-v1", None).is_some() {
        return check_report(value, "report".to_string(), None);
    }

    if value.collect_simple_record("harness-repro-bundle-v1", None).is_some() {
        let bundle = super::schema::parse_repro_bundle(value)?;
        return match bundle.kind {
            super::schema::HarnessReproBundleKind::Report => {
                if let Some(loss_classification) = bundle.loss_classification.as_deref()
                    && loss_classification != "gate-preserving"
                {
                    return Err(MoltenError::invalid_harness(format!(
                        "{} repro bundle {} is {loss_classification} and cannot satisfy pass evidence gates without an explicit gate-preserving policy",
                        bundle.export_profile.as_deref().unwrap_or("profiled"),
                        bundle.bundle_ref
                    )));
                }
                let report_value = bundle
                    .report_value
                    .clone()
                    .ok_or_else(|| MoltenError::invalid_harness("report repro bundle missing report value"))?;
                validate_sealed_report_bundle(&report_value, &bundle)?;
                let mut check = check_report(&report_value, "repro-bundle".to_string(), Some(bundle.bundle_ref))?;
                check.redaction_policy_ref = bundle.redaction_policy_ref;
                check.redaction_gate_ref = bundle.redaction_gate_ref;
                Ok(check)
            }
            super::schema::HarnessReproBundleKind::Failure => Err(MoltenError::invalid_harness(format!(
                "failure repro bundle {} wrapping {} cannot satisfy pass evidence gate",
                bundle.bundle_ref, bundle.artifact_ref
            ))),
        };
    }

    Err(MoltenError::invalid_harness(
        "expected harness report or report repro bundle as pass evidence; failure artifacts are diagnostics only",
    ))
}

pub fn sealed_repro_bundle_value_with_command(report_value: &IoValue, command: &[String]) -> Result<IoValue> {
    let report_check = check_report(report_value, "report".to_string(), None)?;
    let report_receipt_value = receipt_value(&report_check);
    super::schema::sealed_repro_bundle_value_with_command_and_receipt(report_value, command, &report_receipt_value)
}

pub fn repro_bundle_value_with_export_profile(
    report_value: &IoValue,
    command: &[String],
    profile: super::schema::ReproExportProfile,
) -> Result<IoValue> {
    match profile {
        super::schema::ReproExportProfile::DenySensitive => {
            sealed_repro_bundle_value_with_command(report_value, command)
        }
        super::schema::ReproExportProfile::RedactedDiagnostic | super::schema::ReproExportProfile::EncryptedPrivate => {
            super::schema::profiled_repro_bundle_value_with_command(report_value, command, profile)
        }
    }
}

pub fn repro_verify_receipt_value(bundle_value: &IoValue) -> Result<IoValue> {
    let bundle = super::schema::parse_repro_bundle(bundle_value)?;
    if bundle.kind == super::schema::HarnessReproBundleKind::Failure {
        return Err(MoltenError::invalid_harness(format!(
            "failure repro bundle {} wrapping {} is diagnostic-only and cannot be verified as pass evidence",
            bundle.bundle_ref, bundle.artifact_ref
        )));
    }
    if let Some(loss_classification) = bundle.loss_classification.as_deref()
        && loss_classification != "gate-preserving"
    {
        return Err(MoltenError::invalid_harness(format!(
            "{} repro bundle {} is {loss_classification} and cannot be verified as pass evidence",
            bundle.export_profile.as_deref().unwrap_or("profiled"),
            bundle.bundle_ref
        )));
    }
    let embedded_receipt_value = bundle.receipt_value.as_ref().ok_or_else(|| {
        MoltenError::invalid_harness("unsealed report repro bundle cannot satisfy sealed repro verification")
    })?;
    let embedded_receipt = parse_receipt(embedded_receipt_value)?;
    let check = check_value(bundle_value)?;
    if check.artifact_kind != "repro-bundle" || check.artifact_ref != bundle.bundle_ref {
        return Err(MoltenError::invalid_harness("repro verify gate check did not bind bundle artifact"));
    }
    if embedded_receipt.report_ref != check.report_ref || embedded_receipt.suite_ref != check.suite_ref {
        return Err(MoltenError::invalid_harness(
            "repro verify embedded receipt does not match recomputed bundle report refs",
        ));
    }
    Ok(record("repro-verify-receipt-v1", vec![
        string(HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        tool_value(),
        record("bundle", vec![string(&bundle.bundle_ref)]),
        record("artifact", vec![string(&bundle.artifact_ref)]),
        record("report", vec![string(&check.report_ref)]),
        record("suite", vec![string(&check.suite_ref)]),
        record("gate-receipt", vec![string(&embedded_receipt.receipt_ref)]),
        repro_verify_checks_value(),
    ]))
}

pub fn receipt_value(check: &Check) -> IoValue {
    let mut refs = vec![
        ("artifact", check.artifact_ref.as_str()),
        ("report", check.report_ref.as_str()),
        ("suite", check.suite_ref.as_str()),
        ("executor-preflights", check.executor_preflights_ref.as_str()),
        ("executor-execution-receipts", check.executor_execution_receipts_ref.as_str()),
        ("runtime-predicate-receipts", check.runtime_predicate_receipts_ref.as_str()),
        ("policy", check.policy_ref.as_str()),
        ("policy-gate", check.policy_gate_ref.as_str()),
        ("policy-nickel-source", check.policy_nickel_source_ref.as_str()),
        ("policy-nickel-export", check.policy_nickel_export_ref.as_str()),
        ("policy-basalt-preflight", check.policy_basalt_preflight_ref.as_str()),
        ("budget", check.budget_ref.as_str()),
        ("budget-gate", check.budget_gate_ref.as_str()),
        ("budget-nickel-source", check.budget_nickel_source_ref.as_str()),
        ("budget-nickel-export", check.budget_nickel_export_ref.as_str()),
        ("budget-basalt-preflight", check.budget_basalt_preflight_ref.as_str()),
        ("capabilities", check.capability_ref.as_str()),
        ("capability-gate", check.capability_gate_ref.as_str()),
        ("capability-authority-preflight", check.capability_authority_preflight_ref.as_str()),
        ("ucan-proofset", check.capability_proofset_ref.as_str()),
        ("chain-link", check.chain_evidence.link_ref.as_str()),
        ("chain-anchor", check.chain_evidence.anchor_ref.as_str()),
        ("chain-verify-receipt", check.chain_evidence.verify_receipt_ref.as_str()),
        ("chain-checkpoint", check.chain_evidence.checkpoint_ref.as_str()),
        ("chain-range-predicate", check.chain_evidence.range_predicate_ref.as_str()),
        ("turn-journals", check.turn_journals.aggregate_ref.as_str()),
        ("deterministic-replay-verify", check.deterministic_replay_verify_ref.as_str()),
    ];
    if let Some(redaction_policy_ref) = &check.redaction_policy_ref {
        refs.push(("redaction-policy", redaction_policy_ref.as_str()));
    }
    if let Some(redaction_gate_ref) = &check.redaction_gate_ref {
        refs.push(("redaction-gate", redaction_gate_ref.as_str()));
    }
    record("gate-receipt-v1", vec![
        string(HARNESS_GATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("artifact-kind", vec![string(&check.artifact_kind)]),
        record("artifact", vec![string(&check.artifact_ref)]),
        tool_value(),
        artifact_refs_value(&refs),
        validation_value(check),
        replay_value(check),
        checks_value(),
        chain_evidence_value(&check.chain_evidence),
        turn_journals_value(&check.turn_journals),
        string(&check.report_ref),
        string(&check.suite_ref),
        string(&check.final_state_hash),
    ])
}

pub fn parse_receipt(value: &IoValue) -> Result<Receipt> {
    let receipt = simple_record(value, "gate-receipt-v1", 14)?;
    let schema = required_string(&receipt[0], "gate receipt schema")?;
    if schema != HARNESS_GATE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported gate receipt schema {schema}; expected {HARNESS_GATE_RECEIPT_SCHEMA}"
        )));
    }

    let decision = required_record_string(&receipt[1], "decision", "gate receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate receipt decision {decision}")));
    }
    let artifact_kind = required_record_string(&receipt[2], "artifact-kind", "gate receipt artifact kind")?;
    if !matches!(artifact_kind.as_str(), "report" | "repro-bundle") {
        return Err(MoltenError::invalid_harness(format!("unsupported gate receipt artifact kind {artifact_kind}")));
    }
    let artifact_ref = required_record_hash(&receipt[3], "artifact", "gate receipt artifact ref")?;
    validate_tool_record(&receipt[4])?;
    let artifact_refs = parse_artifact_refs(&receipt[5])?;
    require_artifact_ref(&artifact_refs, "artifact", &artifact_ref)?;

    let validation = parse_validation(&receipt[6])?;
    let replay = parse_replay(&receipt[7])?;
    let checks = parse_checks(&receipt[8])?;
    require_all_checks(&checks)?;

    let chain_evidence = parse_chain_evidence(&receipt[9])?;
    let report_ref = required_hash(&receipt[11], "gate receipt report ref")?;
    let suite_ref = required_hash(&receipt[12], "gate receipt suite ref")?;
    let final_state_hash = required_hash(&receipt[13], "gate receipt final state hash")?;
    let turn_journals = parse_turn_journals(&receipt[10], &report_ref, &suite_ref)?;
    require_core_refs(&CoreRefs {
        validation: &validation,
        replay: &replay,
        report: &report_ref,
        suite: &suite_ref,
        final_state: &final_state_hash,
    })?;
    let chain_link = crate::evidence_chain::parse_chain_link(&chain_evidence.link_value)?;
    require_link_context(&chain_link, &report_ref, &suite_ref, &final_state_hash)?;
    require_artifact_ref(&artifact_refs, "report", &report_ref)?;
    require_artifact_ref(&artifact_refs, "suite", &suite_ref)?;
    require_kinds(&artifact_refs, REQUIRED_KINDS)?;
    require_artifact_ref(&artifact_refs, "chain-link", &chain_evidence.link_ref)?;
    require_artifact_ref(&artifact_refs, "chain-anchor", &chain_evidence.anchor_ref)?;
    require_artifact_ref(&artifact_refs, "chain-verify-receipt", &chain_evidence.verify_receipt_ref)?;
    require_artifact_ref(&artifact_refs, "chain-checkpoint", &chain_evidence.checkpoint_ref)?;
    require_artifact_ref(&artifact_refs, "chain-range-predicate", &chain_evidence.range_predicate_ref)?;
    require_artifact_ref(&artifact_refs, "turn-journals", &turn_journals.aggregate_ref)?;
    require_artifact_ref(&artifact_refs, "deterministic-replay-verify", &replay.verify_ref)?;
    if artifact_kind == "repro-bundle" {
        require_kinds(&artifact_refs, REDACTION_KINDS)?;
    }

    Ok(Receipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        artifact_kind,
        artifact_ref,
        report_ref,
        suite_ref,
        final_state_hash,
        checks,
    })
}

pub fn check_summary(check: &Check) -> String {
    format!(
        "gate check ok\nartifact_kind={}\nartifact={}\nreport={}\nsuite={}\nfinal_state={}",
        check.artifact_kind, check.artifact_ref, check.report_ref, check.suite_ref, check.final_state_hash
    )
}

pub fn receipt_summary(value: &IoValue) -> Result<String> {
    let receipt = parse_receipt(value)?;
    Ok(format!(
        "gate receipt {}\ndecision={}\nartifact_kind={}\nartifact={}\nreport={}\nsuite={}\nfinal_state={}\nchecks={}",
        receipt.receipt_ref,
        receipt.decision,
        receipt.artifact_kind,
        receipt.artifact_ref,
        receipt.report_ref,
        receipt.suite_ref,
        receipt.final_state_hash,
        receipt.checks.len()
    ))
}

pub fn parse_repro_verify_receipt(value: &IoValue) -> Result<ReproVerifyReceipt> {
    let receipt = simple_record(value, "repro-verify-receipt-v1", 9)?;
    let schema = required_string(&receipt[0], "repro verify receipt schema")?;
    if schema != HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported repro verify receipt schema {schema}; expected {HARNESS_REPRO_VERIFY_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "repro verify receipt decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported repro verify receipt decision {decision}")));
    }
    validate_tool_record(&receipt[2])?;
    let bundle_ref = required_record_hash(&receipt[3], "bundle", "repro verify bundle ref")?;
    let artifact_ref = required_record_hash(&receipt[4], "artifact", "repro verify artifact ref")?;
    let report_ref = required_record_hash(&receipt[5], "report", "repro verify report ref")?;
    if artifact_ref != report_ref {
        return Err(MoltenError::invalid_harness("repro verify receipt artifact ref does not match report ref"));
    }
    let suite_ref = required_record_hash(&receipt[6], "suite", "repro verify suite ref")?;
    let gate_receipt_ref = required_record_hash(&receipt[7], "gate-receipt", "repro verify gate receipt ref")?;
    let checks = parse_checks(&receipt[8])?;
    require_check(&checks, "sealed-bundle")?;
    require_check(&checks, "embedded-report")?;
    require_check(&checks, "embedded-gate-receipt")?;
    require_check(&checks, "report-validation")?;
    require_check(&checks, "deterministic-replay")?;
    require_check(&checks, "gate-receipt-recomputed")?;
    Ok(ReproVerifyReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        bundle_ref,
        report_ref,
        suite_ref,
        gate_receipt_ref,
        checks,
    })
}

pub fn repro_verify_receipt_summary(value: &IoValue) -> Result<String> {
    let receipt = parse_repro_verify_receipt(value)?;
    Ok(format!(
        "repro verify receipt {}\ndecision={}\nbundle={}\nreport={}\nsuite={}\ngate_receipt={}\nchecks={}",
        receipt.receipt_ref,
        receipt.decision,
        receipt.bundle_ref,
        receipt.report_ref,
        receipt.suite_ref,
        receipt.gate_receipt_ref,
        receipt.checks.len()
    ))
}

fn validate_sealed_report_bundle(report_value: &IoValue, bundle: &super::schema::HarnessReproBundle) -> Result<()> {
    if bundle.redaction_policy_ref.is_none() || bundle.redaction_gate_ref.is_none() {
        return Err(MoltenError::invalid_harness("sealed report repro bundle missing redaction preflight evidence"));
    }
    let embedded_receipt_value = bundle
        .receipt_value
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("sealed report repro bundle missing embedded gate receipt"))?;
    let embedded_receipt_ref = bundle
        .gate_receipt_ref
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("sealed report repro bundle missing gate receipt ref"))?;
    let receipt = parse_receipt(embedded_receipt_value)?;
    if &receipt.receipt_ref != embedded_receipt_ref {
        return Err(MoltenError::invalid_harness(
            "sealed repro bundle gate receipt ref does not match embedded receipt",
        ));
    }
    if receipt.artifact_kind != "report" {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle must embed a report gate receipt, got {}",
            receipt.artifact_kind
        )));
    }
    if receipt.artifact_ref != bundle.artifact_ref || receipt.report_ref != bundle.artifact_ref {
        return Err(MoltenError::invalid_harness(
            "sealed repro bundle gate receipt does not bind the embedded report ref",
        ));
    }
    let expected_report_check = check_report(report_value, "report".to_string(), None)?;
    let expected_receipt_value = receipt_value(&expected_report_check);
    let expected_receipt_ref = canonical_hash(&expected_receipt_value)?;
    let actual_receipt_ref = canonical_hash(embedded_receipt_value)?;
    if actual_receipt_ref != expected_receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "sealed repro bundle embedded gate receipt does not match report: receipt hashes to {actual_receipt_ref}, expected {expected_receipt_ref}"
        )));
    }
    Ok(())
}

struct EvidenceRefs {
    executor_preflights_ref: String,
    executor_execution_receipts_ref: String,
    runtime_predicate_receipts_ref: String,
    policy_ref: String,
    policy_gate_ref: String,
    policy_nickel_source_ref: String,
    policy_nickel_export_ref: String,
    policy_basalt_preflight_ref: String,
    budget_ref: String,
    budget_gate_ref: String,
    budget_nickel_source_ref: String,
    budget_nickel_export_ref: String,
    budget_basalt_preflight_ref: String,
    capability_ref: String,
    capability_gate_ref: String,
    capability_authority_preflight_ref: String,
    capability_proofset_ref: String,
}

fn check_report(value: &IoValue, artifact_kind: String, artifact_ref: Option<String>) -> Result<Check> {
    let validation = super::replay::validate_report_value(value)?;
    let replay = super::replay::replay_report_value(value)?;
    let report = super::schema::parse_report(value)?;
    if validation.report_ref != replay.expected_report_ref || validation.report_ref != replay.actual_report_ref {
        return Err(MoltenError::invalid_harness("gate replay report refs do not match validation report ref"));
    }
    if validation.final_state_hash != replay.final_state_hash {
        return Err(MoltenError::invalid_harness("gate replay final state does not match validation final state"));
    }
    let refs = evidence_refs(&report)?;
    let deterministic_replay_verify_value =
        harness_replay_verify_value(&replay.expected_report_ref, &replay.actual_report_ref, &replay.final_state_hash);
    let deterministic_replay_verify_ref = canonical_hash(&deterministic_replay_verify_value)?;
    let chain_evidence = build_gate_chain_evidence(
        &validation.report_ref,
        &validation.suite_ref,
        &report.final_state_hash,
        &report.profile,
    )?;
    let turn_journals = build_turn_journals(&report)?;
    Ok(Check {
        artifact_kind,
        artifact_ref: artifact_ref.unwrap_or_else(|| validation.report_ref.clone()),
        report_ref: validation.report_ref,
        suite_ref: validation.suite_ref,
        initial_state_hash: report.initial_state_hash,
        final_state_hash: report.final_state_hash,
        replay_actual_report_ref: replay.actual_report_ref,
        deterministic_replay_verify_ref,
        deterministic_replay_verify_value,
        executor_preflights_ref: refs.executor_preflights_ref,
        executor_execution_receipts_ref: refs.executor_execution_receipts_ref,
        runtime_predicate_receipts_ref: refs.runtime_predicate_receipts_ref,
        policy_ref: refs.policy_ref,
        policy_gate_ref: refs.policy_gate_ref,
        policy_nickel_source_ref: refs.policy_nickel_source_ref,
        policy_nickel_export_ref: refs.policy_nickel_export_ref,
        policy_basalt_preflight_ref: refs.policy_basalt_preflight_ref,
        budget_ref: refs.budget_ref,
        budget_gate_ref: refs.budget_gate_ref,
        budget_nickel_source_ref: refs.budget_nickel_source_ref,
        budget_nickel_export_ref: refs.budget_nickel_export_ref,
        budget_basalt_preflight_ref: refs.budget_basalt_preflight_ref,
        capability_ref: refs.capability_ref,
        capability_gate_ref: refs.capability_gate_ref,
        capability_authority_preflight_ref: refs.capability_authority_preflight_ref,
        capability_proofset_ref: refs.capability_proofset_ref,
        redaction_policy_ref: None,
        redaction_gate_ref: None,
        observations: validation.observations as u64,
        actors: report.actors,
        budget: report.budget,
        chain_evidence,
        turn_journals,
    })
}

fn evidence_refs(report: &super::schema::HarnessReport) -> Result<EvidenceRefs> {
    let policy = report
        .policy_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing policy gate evidence"))?;
    let budget = report
        .budget_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing budget gate evidence"))?;
    let capability = report
        .capability_gate
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing capability gate evidence"))?;
    let preflights = report
        .executor_preflights
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("missing executor preflight evidence"))?;
    Ok(EvidenceRefs {
        executor_preflights_ref: canonical_hash(&preflights.value)?,
        executor_execution_receipts_ref: executor_execution_receipts_ref(&report.observations)?,
        runtime_predicate_receipts_ref: runtime_predicate_receipts_ref(&report.observations)?,
        policy_ref: policy.policy_ref.clone(),
        policy_gate_ref: canonical_hash(&policy.value)?,
        policy_nickel_source_ref: policy.nickel_source_ref.clone(),
        policy_nickel_export_ref: policy.nickel_export_ref.clone(),
        policy_basalt_preflight_ref: policy.basalt_preflight_ref.clone(),
        budget_ref: budget.budget_ref.clone(),
        budget_gate_ref: canonical_hash(&budget.value)?,
        budget_nickel_source_ref: budget.nickel_source_ref.clone(),
        budget_nickel_export_ref: budget.nickel_export_ref.clone(),
        budget_basalt_preflight_ref: budget.basalt_preflight_ref.clone(),
        capability_ref: capability.capability_ref.clone(),
        capability_gate_ref: canonical_hash(&capability.value)?,
        capability_authority_preflight_ref: capability.authority_preflight_ref.clone(),
        capability_proofset_ref: capability.proofset_ref.clone(),
    })
}

fn executor_execution_receipts_ref(observations: &[super::schema::HarnessObservation]) -> Result<String> {
    let receipts = observations
        .iter()
        .flat_map(|observation| observation.events.iter())
        .filter(|event| {
            matches!(
                super::schema::event_boundary(event),
                super::schema::EventBoundary::SteelExecution | super::schema::EventBoundary::WasmExecution
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    canonical_hash(&record("executor-execution-receipts", vec![sequence(receipts)]))
}

fn runtime_predicate_receipts_ref(observations: &[super::schema::HarnessObservation]) -> Result<String> {
    let receipts = observations
        .iter()
        .flat_map(|observation| observation.events.iter())
        .filter(|event| super::schema::event_boundary(event) == super::schema::EventBoundary::RuntimePredicate)
        .cloned()
        .collect::<Vec<_>>();
    canonical_hash(&record("runtime-predicate-receipts", vec![sequence(receipts)]))
}

fn tool_value() -> IoValue {
    record("tool", vec![string("molten"), string(env!("CARGO_PKG_VERSION"))])
}

fn artifact_refs_value(refs: &[(&str, &str)]) -> IoValue {
    record("artifact-refs", vec![sequence(
        refs.iter()
            .map(|(kind, artifact_ref)| record("artifact-ref", vec![string(*kind), string(*artifact_ref)]))
            .collect(),
    )])
}

fn validation_value(check: &Check) -> IoValue {
    record("validation", vec![
        record("status", vec![string("pass")]),
        record("report", vec![string(&check.report_ref)]),
        record("suite", vec![string(&check.suite_ref)]),
        record("final-state", vec![string(&check.final_state_hash)]),
        record("observations", vec![u64_value(check.observations)]),
        super::schema::actor_registry_value(&check.actors),
        super::schema::budget_value(&check.budget.limits, &check.budget.usage),
    ])
}

fn harness_replay_verify_value(expected_report_ref: &str, actual_report_ref: &str, final_state_hash: &str) -> IoValue {
    record("deterministic-replay-verify-v1", vec![
        string(DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
        string("pass"),
        record("expected-report-ref", vec![string(expected_report_ref)]),
        record("actual-report-ref", vec![string(actual_report_ref)]),
        record("final-state-ref", vec![string(final_state_hash)]),
        record("divergence", vec![string("none")]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("report-replayed"), string("pass")]),
            record("check", vec![string("final-state-bound"), string("pass")]),
            record("check", vec![string("no-divergence"), string("pass")]),
        ])]),
    ])
}

fn replay_value(check: &Check) -> IoValue {
    record("replay", vec![
        record("status", vec![string("pass")]),
        record("expected-report", vec![string(&check.report_ref)]),
        record("actual-report", vec![string(&check.replay_actual_report_ref)]),
        record("final-state", vec![string(&check.final_state_hash)]),
        record("verify-ref", vec![string(&check.deterministic_replay_verify_ref)]),
        check.deterministic_replay_verify_value.clone(),
    ])
}

struct PassLink {
    chain: crate::evidence_chain::ChainScope,
    producer: crate::evidence_chain::ChainProducer,
    link_ref: String,
    link_value: IoValue,
    payload_refs: Vec<String>,
    subject_refs: Vec<String>,
    context_refs: Vec<String>,
}

struct PassPredicates {
    values: Vec<IoValue>,
    refs: Vec<String>,
    range_ref: String,
}

struct PassArtifacts {
    anchor_ref: String,
    anchor_value: IoValue,
    verify_ref: String,
    verify_value: IoValue,
    checkpoint_ref: String,
    checkpoint_value: IoValue,
}

struct Pred<'a> {
    predicate: &'a str,
    subject_refs: &'a [String],
    input_refs: &'a [String],
    context_refs: &'a [String],
    checks: &'a [crate::evidence_chain::ChainCheck],
}

fn build_gate_chain_evidence(
    report_ref: &str,
    suite_ref: &str,
    final_state_hash: &str,
    profile: &str,
) -> Result<ChainEvidence> {
    let link = pass_link(report_ref, suite_ref, final_state_hash, profile)?;
    let predicates = pass_predicates(&link)?;
    let artifacts = pass_artifacts(&link, &predicates, suite_ref)?;
    Ok(ChainEvidence {
        link_ref: link.link_ref,
        anchor_ref: artifacts.anchor_ref,
        verify_receipt_ref: artifacts.verify_ref,
        checkpoint_ref: artifacts.checkpoint_ref,
        range_predicate_ref: predicates.range_ref,
        predicate_receipt_refs: predicates.refs,
        link_value: link.link_value,
        anchor_value: artifacts.anchor_value,
        verify_receipt_value: artifacts.verify_value,
        checkpoint_value: artifacts.checkpoint_value,
        predicate_values: predicates.values,
    })
}

fn pass_link(report_ref: &str, suite_ref: &str, final_state_hash: &str, profile: &str) -> Result<PassLink> {
    let chain = crate::evidence_chain::ChainScope::new("harness-pass-evidence", report_ref, profile);
    let producer_key_ref = canonical_hash(&record("gate-chain-producer-key", vec![string("molten")]))?;
    let producer = crate::evidence_chain::ChainProducer::new("molten-gate", producer_key_ref);
    let trellis_input_ref = canonical_hash(&record("gate-chain-input", vec![
        string(report_ref),
        string(suite_ref),
        string(final_state_hash),
    ]))?;
    let link_value = crate::evidence_chain::chain_link_value(&crate::evidence_chain::ChainLinkInput::genesis(
        chain.clone(),
        crate::evidence_chain::ChainPayload::new("harness-report", report_ref, HARNESS_REPORT_SCHEMA),
        vec![
            crate::evidence_chain::ChainContextRef::new("suite", suite_ref),
            crate::evidence_chain::ChainContextRef::new("final-state", final_state_hash),
        ],
        producer.clone(),
        trellis_input_ref,
    ));
    let link = crate::evidence_chain::parse_chain_link(&link_value)?;
    let link_ref = link.link_ref.clone();
    let scope_context_ref = canonical_hash(&record("gate-chain-scope", vec![
        string(&chain.scope),
        string(&chain.id),
        string(&chain.epoch),
    ]))?;
    Ok(PassLink {
        chain,
        producer,
        link_ref: link_ref.clone(),
        link_value,
        payload_refs: vec![report_ref.to_string()],
        subject_refs: vec![link_ref],
        context_refs: vec![scope_context_ref, suite_ref.to_string(), final_state_hash.to_string()],
    })
}

fn pass_predicate(input: Pred<'_>) -> IoValue {
    crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
        predicate: input.predicate,
        decision: "pass",
        subject_refs: input.subject_refs,
        input_refs: input.input_refs,
        context_refs: input.context_refs,
        checks: input.checks,
    })
}

fn pass_predicates(link: &PassLink) -> Result<PassPredicates> {
    let genesis_checks = vec![
        crate::evidence_chain::ChainCheck::pass("trellis-bounded-predicate"),
        crate::evidence_chain::ChainCheck::pass("predicate-decision-binding"),
    ];
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    let checkpoint_checks = vec![
        crate::evidence_chain::ChainCheck::pass("checkpoint-range-coverage"),
        crate::evidence_chain::ChainCheck::pass("verified-range"),
    ];
    let values = vec![
        pass_predicate(Pred {
            predicate: crate::evidence_chain::GENESIS_VALID_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.payload_refs,
            context_refs: &link.context_refs,
            checks: &genesis_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.payload_refs,
            context_refs: &link.context_refs,
            checks: &segment_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.subject_refs,
            context_refs: &link.context_refs,
            checks: &fork_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.subject_refs,
            context_refs: &link.context_refs,
            checks: &anchor_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.payload_refs,
            context_refs: &link.context_refs,
            checks: &checkpoint_checks,
        }),
    ];
    let (refs, range_ref) = pass_predicate_refs(&values)?;
    Ok(PassPredicates {
        values,
        refs,
        range_ref,
    })
}

fn pass_predicate_refs(values: &[IoValue]) -> Result<(Vec<String>, String)> {
    let mut refs = Vec::with_capacity(values.len());
    let mut range_ref = None;
    for value in values {
        let parsed = crate::evidence_chain::parse_chain_predicate_receipt(value)?;
        if parsed.predicate == crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE {
            range_ref = Some(parsed.receipt_ref.clone());
        }
        refs.push(parsed.receipt_ref);
    }
    Ok((
        refs,
        range_ref.ok_or_else(|| {
            MoltenError::invalid_harness("gate chain evidence did not build checkpoint range predicate")
        })?,
    ))
}

fn pass_verify_value(link: &PassLink, predicates: &PassPredicates) -> IoValue {
    let diagnostics = Vec::new();
    let receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain: &link.chain,
        anchor_ref: Some(&link.link_ref),
        expected_head: Some(&link.link_ref),
        discovered_heads: std::slice::from_ref(&link.link_ref),
        verified_links: std::slice::from_ref(&link.link_ref),
        payload_refs: &link.payload_refs,
        diagnostics: &diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt,
            predicate_receipt_refs: &predicates.refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}

fn pass_artifacts(link: &PassLink, predicates: &PassPredicates, suite_ref: &str) -> Result<PassArtifacts> {
    let policy_refs = vec![suite_ref.to_string()];
    let anchor_value =
        crate::evidence_chain::chain_anchor_value(&link.chain, &link.link_ref, &policy_refs, &link.producer);
    let anchor_ref = canonical_hash(&anchor_value)?;
    let verify_value = pass_verify_value(link, predicates);
    let verify_ref = canonical_hash(&verify_value)?;
    let checkpoint_value =
        crate::evidence_chain::chain_checkpoint_value(&crate::evidence_chain::ChainCheckpointInput {
            chain: link.chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: link.link_ref.clone(),
            head_ref: link.link_ref.clone(),
            verify_receipt_ref: verify_ref.clone(),
            range_predicate_ref: predicates.range_ref.clone(),
            policy_refs,
            membership_refs: vec![suite_ref.to_string()],
            producer: link.producer.clone(),
            checks: vec![
                crate::evidence_chain::ChainCheck::pass("raft-control-plane-command"),
                crate::evidence_chain::ChainCheck::pass("verified-range"),
                crate::evidence_chain::ChainCheck::pass("checkpoint-freshness"),
            ],
        });
    let checkpoint_ref = canonical_hash(&checkpoint_value)?;
    Ok(PassArtifacts {
        anchor_ref,
        anchor_value,
        verify_ref,
        verify_value,
        checkpoint_ref,
        checkpoint_value,
    })
}

fn chain_evidence_value(evidence: &ChainEvidence) -> IoValue {
    record("chain-evidence", vec![
        record("profile", vec![string("local-pass-evidence-chain")]),
        record("link", vec![evidence.link_value.clone()]),
        record("anchor", vec![evidence.anchor_value.clone()]),
        record("verify-receipt", vec![evidence.verify_receipt_value.clone()]),
        record("checkpoint", vec![evidence.checkpoint_value.clone()]),
        record("predicates", vec![sequence(evidence.predicate_values.clone())]),
        record("checks", vec![sequence(
            [
                "chain-continuity",
                "chain-anchor-descent",
                "chain-checkpoint-freshness",
                "chain-predicate-receipts",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

struct EvidenceParts {
    link_value: IoValue,
    anchor_value: IoValue,
    verify_receipt_value: IoValue,
    checkpoint_value: IoValue,
    predicate_values: Vec<IoValue>,
}

struct ParsedPredicates {
    receipts: Vec<crate::evidence_chain::ChainPredicateReceipt>,
    refs: Vec<String>,
}

fn parse_chain_evidence(value: &Value<IoValue>) -> Result<ChainEvidence> {
    let parts = evidence_parts(value)?;
    let link = crate::evidence_chain::parse_chain_link(&parts.link_value)?;
    let link_ref = link.link_ref.clone();
    let anchor = crate::evidence_chain::parse_chain_anchor(&parts.anchor_value)?;
    if anchor.link_ref != link_ref || anchor.chain != link.chain {
        return Err(MoltenError::invalid_harness("gate chain anchor does not bind the gate chain link"));
    }
    let checkpoint = crate::evidence_chain::parse_chain_checkpoint(&parts.checkpoint_value)?;
    if checkpoint.chain != link.chain || checkpoint.anchor_link_ref != link_ref || checkpoint.head_ref != link_ref {
        return Err(MoltenError::invalid_harness("gate chain checkpoint does not bind the anchored chain head"));
    }
    let verify_receipt_ref = canonical_hash(&parts.verify_receipt_value)?;
    if checkpoint.verify_receipt_ref != verify_receipt_ref {
        return Err(MoltenError::invalid_harness("gate chain checkpoint does not bind the embedded verify receipt"));
    }

    let predicates = parsed_predicates(&parts.predicate_values)?;
    require_predicates(&predicates, &checkpoint.range_predicate_ref, &link_ref, &link.payload.artifact_ref)?;
    validate_gate_chain_verify_receipt(
        &parts.verify_receipt_value,
        &link,
        &checkpoint.range_predicate_ref,
        &predicates.refs,
    )?;

    Ok(ChainEvidence {
        link_ref,
        anchor_ref: anchor.anchor_ref,
        verify_receipt_ref,
        checkpoint_ref: checkpoint.checkpoint_ref,
        range_predicate_ref: checkpoint.range_predicate_ref,
        predicate_receipt_refs: predicates.refs,
        link_value: parts.link_value,
        anchor_value: parts.anchor_value,
        verify_receipt_value: parts.verify_receipt_value,
        checkpoint_value: parts.checkpoint_value,
        predicate_values: parts.predicate_values,
    })
}

fn evidence_parts(value: &Value<IoValue>) -> Result<EvidenceParts> {
    let value = value_to_iovalue(value);
    let evidence = simple_record(&value, "chain-evidence", 7)?;
    let profile = required_record_string(&evidence[0], "profile", "chain evidence profile")?;
    if profile != "local-pass-evidence-chain" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate chain evidence profile {profile}")));
    }
    let checks = parse_checks(&evidence[6])?;
    require_check(&checks, "chain-continuity")?;
    require_check(&checks, "chain-anchor-descent")?;
    require_check(&checks, "chain-checkpoint-freshness")?;
    require_check(&checks, "chain-predicate-receipts")?;
    Ok(EvidenceParts {
        link_value: required_record_value(&evidence[1], "link")?,
        anchor_value: required_record_value(&evidence[2], "anchor")?,
        verify_receipt_value: required_record_value(&evidence[3], "verify-receipt")?,
        checkpoint_value: required_record_value(&evidence[4], "checkpoint")?,
        predicate_values: required_record_values(&evidence[5], "predicates")?,
    })
}

fn parsed_predicates(values: &[IoValue]) -> Result<ParsedPredicates> {
    let mut receipts = Vec::with_capacity(values.len());
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        let parsed = crate::evidence_chain::parse_chain_predicate_receipt(value)?;
        refs.push(parsed.receipt_ref.clone());
        receipts.push(parsed);
    }
    Ok(ParsedPredicates { receipts, refs })
}

fn require_predicates(predicates: &ParsedPredicates, range_ref: &str, link_ref: &str, payload_ref: &str) -> Result<()> {
    let range_predicate = require_chain_predicate(
        &predicates.receipts,
        range_ref,
        crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
    )?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::GENESIS_VALID_PREDICATE)?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE)?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE)?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE)?;
    if range_predicate.subject_refs != vec![link_ref.to_string()] {
        return Err(MoltenError::invalid_harness("gate chain range predicate subjects do not match anchored link"));
    }
    if range_predicate.input_refs != vec![payload_ref.to_string()] {
        return Err(MoltenError::invalid_harness("gate chain range predicate inputs do not match report payload ref"));
    }
    Ok(())
}

fn validate_gate_chain_verify_receipt(
    value: &IoValue,
    link: &crate::evidence_chain::ChainLink,
    range_predicate_ref: &str,
    predicate_receipt_refs: &[String],
) -> Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("gate chain evidence missing chain verify receipt"))?;
    let schema = required_string(&receipt[0], "chain verify receipt schema")?;
    if schema != EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chain verify receipt schema {schema}; expected {EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "chain verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "gate chain verify receipt decision must be pass, got {decision}"
        )));
    }
    let anchor_ref = required_record_optional_hash(&receipt[3], "anchor", "chain verify anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("gate chain verify receipt missing anchor"))?;
    let expected_head = required_record_optional_hash(&receipt[4], "expected-head", "chain verify expected head")?
        .ok_or_else(|| MoltenError::invalid_harness("gate chain verify receipt missing expected head"))?;
    if anchor_ref != link.link_ref || expected_head != link.link_ref {
        return Err(MoltenError::invalid_harness("gate chain verify receipt does not bind the anchored head"));
    }
    let discovered_heads = required_record_hash_sequence(&receipt[5], "discovered-heads")?;
    let verified_links = required_record_hash_sequence(&receipt[6], "verified-links")?;
    let payload_refs = required_record_hash_sequence(&receipt[7], "payloads")?;
    let predicate_refs = required_record_hash_sequence(&receipt[8], "predicates")?;
    if discovered_heads != vec![link.link_ref.clone()] || verified_links != vec![link.link_ref.clone()] {
        return Err(MoltenError::invalid_harness(
            "gate chain verify receipt must cover exactly the anchored report link",
        ));
    }
    if payload_refs != vec![link.payload.artifact_ref.clone()] {
        return Err(MoltenError::invalid_harness(
            "gate chain verify receipt payload refs do not bind the report payload",
        ));
    }
    if predicate_refs != predicate_receipt_refs {
        return Err(MoltenError::invalid_harness(
            "gate chain verify receipt predicate refs do not match embedded predicate receipts",
        ));
    }
    if !predicate_refs.iter().any(|predicate_ref| predicate_ref == range_predicate_ref) {
        return Err(MoltenError::invalid_harness("gate chain verify receipt does not bind checkpoint range predicate"));
    }
    Ok(())
}

fn require_chain_predicate<'a>(
    predicates: &'a [crate::evidence_chain::ChainPredicateReceipt],
    expected_ref: &str,
    expected_kind: &str,
) -> Result<&'a crate::evidence_chain::ChainPredicateReceipt> {
    let predicate = predicates
        .iter()
        .find(|predicate| predicate.receipt_ref == expected_ref)
        .ok_or_else(|| MoltenError::invalid_harness("gate chain evidence missing checkpoint range predicate"))?;
    if predicate.predicate != expected_kind || predicate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "gate chain predicate {expected_ref} must be a passing {expected_kind} receipt"
        )));
    }
    Ok(predicate)
}

fn require_chain_predicate_kind(
    predicates: &[crate::evidence_chain::ChainPredicateReceipt],
    expected_kind: &str,
) -> Result<()> {
    if predicates
        .iter()
        .any(|predicate| predicate.predicate == expected_kind && predicate.decision == "pass")
    {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "gate chain evidence missing passing {expected_kind} predicate receipt"
        )))
    }
}

#[derive(Debug, Clone)]
struct TurnJournalBuilder {
    actor_id: String,
    chain: crate::evidence_chain::ChainScope,
    links: Vec<crate::evidence_chain::ChainLink>,
    link_values: Vec<IoValue>,
    payload_refs: Vec<String>,
}

struct LinkEnds<'a> {
    anchor_ref: &'a String,
    head_ref: &'a String,
}

fn build_turn_journals(report: &super::schema::HarnessReport) -> Result<TurnJournalEvidence> {
    let suite = super::schema::parse_suite(&report.suite_value)?;
    if suite.steps.len() != report.observations.len() {
        return Err(MoltenError::invalid_harness("turn journal evidence requires one observation per suite step"));
    }
    let mut builders: OrderedMap<String, TurnJournalBuilder> = OrderedMap::new();
    for (position, observation) in report.observations.iter().enumerate() {
        append_turn_journal_observation(&mut builders, report, observation, &suite.steps[position])?;
    }

    let mut journals = Vec::with_capacity(builders.len());
    for builder in builders.into_values() {
        journals.push(build_turn_journal_chain(builder, report)?);
    }
    let mut evidence = TurnJournalEvidence {
        aggregate_ref: String::new(),
        journals,
    };
    evidence.aggregate_ref = canonical_hash(&turn_journals_value(&evidence))?;
    Ok(evidence)
}

fn append_turn_journal_observation(
    builders: &mut OrderedMap<String, TurnJournalBuilder>,
    report: &super::schema::HarnessReport,
    observation: &super::schema::HarnessObservation,
    step: &super::core::CoreStep,
) -> Result<()> {
    let actor_id = step.primary_actor().to_string();
    let computed_step_ref = canonical_hash(&super::schema::step_value(step))?;
    if observation.step_ref != computed_step_ref {
        return Err(MoltenError::invalid_harness(format!(
            "turn journal observation {} step ref does not match embedded suite step",
            observation.index
        )));
    }
    let builder = builders.entry(actor_id.clone()).or_insert_with(|| TurnJournalBuilder {
        actor_id: actor_id.clone(),
        chain: crate::evidence_chain::ChainScope::new(
            "harness-turn-journal",
            actor_id.clone(),
            report.report_ref.clone(),
        ),
        links: Vec::new(),
        link_values: Vec::new(),
        payload_refs: Vec::new(),
    });
    let observation_ref = observation.observation_ref.clone();
    let payload = crate::evidence_chain::ChainPayload::new(
        "turn-observation",
        observation_ref.clone(),
        HARNESS_OBSERVATION_SCHEMA,
    );
    let context_refs = turn_journal_context_refs(report, observation, &actor_id)?;
    let trellis_input_ref = turn_journal_trellis_input_ref(observation, &actor_id)?;
    let producer = turn_journal_producer()?;
    let input = if let Some(previous) = builder.links.last() {
        crate::evidence_chain::ChainLinkInput::append(previous, payload, context_refs, producer, trellis_input_ref)
    } else {
        crate::evidence_chain::ChainLinkInput::genesis(
            builder.chain.clone(),
            payload,
            context_refs,
            producer,
            trellis_input_ref,
        )
    };
    let link_value = crate::evidence_chain::chain_link_value(&input);
    let link = crate::evidence_chain::parse_chain_link(&link_value)?;
    builder.payload_refs.push(observation_ref);
    builder.link_values.push(link_value);
    builder.links.push(link);
    Ok(())
}

fn turn_journal_trellis_input_ref(observation: &super::schema::HarnessObservation, actor_id: &str) -> Result<String> {
    canonical_hash(&record("turn-journal-input", vec![
        string(actor_id),
        u64_value(observation.index),
        record("observation", vec![string(&observation.observation_ref)]),
        string(&observation.step_ref),
        string(&observation.before_state_hash),
        string(&observation.after_state_hash),
        record("event-refs", vec![sequence(observation.event_refs.iter().map(string).collect())]),
    ]))
}

fn build_turn_journal_chain(
    builder: TurnJournalBuilder,
    report: &super::schema::HarnessReport,
) -> Result<TurnJournalChainEvidence> {
    let link_refs = builder.links.iter().map(|link| link.link_ref.clone()).collect::<Vec<_>>();
    let ends = link_ends(&link_refs)?;
    let context_refs = actor_refs(report, &builder.actor_id)?;
    let predicate_values = predicate_values(&link_refs, &builder.payload_refs, &context_refs, &ends);
    let predicate_receipt_refs = predicate_refs(&predicate_values)?;
    let verify_receipt_value =
        verify_value(&builder.chain, &link_refs, &builder.payload_refs, &ends, &predicate_receipt_refs);
    let verify_receipt_ref = canonical_hash(&verify_receipt_value)?;
    Ok(TurnJournalChainEvidence {
        actor_id: builder.actor_id,
        link_refs,
        payload_refs: builder.payload_refs,
        verify_receipt_ref,
        predicate_receipt_refs,
        link_values: builder.link_values,
        verify_receipt_value,
        predicate_values,
    })
}

fn link_ends(link_refs: &[String]) -> Result<LinkEnds<'_>> {
    let Some(anchor_ref) = link_refs.first() else {
        return Err(MoltenError::invalid_harness("turn journal chain must contain at least one link"));
    };
    let Some(head_ref) = link_refs.last() else {
        return Err(MoltenError::invalid_harness("turn journal chain must contain a head link"));
    };
    Ok(LinkEnds { anchor_ref, head_ref })
}

fn actor_refs(report: &super::schema::HarnessReport, actor_id: &str) -> Result<Vec<String>> {
    Ok(vec![
        report.report_ref.clone(),
        report.suite_ref.clone(),
        canonical_hash(&record("turn-journal-actor", vec![string(actor_id)]))?,
    ])
}

fn predicate_values(
    link_refs: &[String],
    payload_refs: &[String],
    context_refs: &[String],
    ends: &LinkEnds<'_>,
) -> Vec<IoValue> {
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_subject_refs = vec![ends.anchor_ref.clone(), ends.head_ref.clone()];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    vec![
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            decision: "pass",
            subject_refs: link_refs,
            input_refs: payload_refs,
            context_refs,
            checks: &segment_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            decision: "pass",
            subject_refs: std::slice::from_ref(ends.head_ref),
            input_refs: link_refs,
            context_refs,
            checks: &fork_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: "pass",
            subject_refs: &anchor_subject_refs,
            input_refs: link_refs,
            context_refs,
            checks: &anchor_checks,
        }),
    ]
}

fn predicate_refs(values: &[IoValue]) -> Result<Vec<String>> {
    Ok(values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|receipt| receipt.receipt_ref)
        .collect())
}

fn verify_value(
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    payload_refs: &[String],
    ends: &LinkEnds<'_>,
    predicate_receipt_refs: &[String],
) -> IoValue {
    let verify_diagnostics = Vec::new();
    let verify_receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain,
        anchor_ref: Some(ends.anchor_ref.as_str()),
        expected_head: Some(ends.head_ref.as_str()),
        discovered_heads: std::slice::from_ref(ends.head_ref),
        verified_links: link_refs,
        payload_refs,
        diagnostics: &verify_diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt: verify_receipt,
            predicate_receipt_refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}

fn turn_journal_context_refs(
    report: &super::schema::HarnessReport,
    observation: &super::schema::HarnessObservation,
    actor_id: &str,
) -> Result<Vec<crate::evidence_chain::ChainContextRef>> {
    let mut refs = vec![
        crate::evidence_chain::ChainContextRef::new("report", report.report_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("suite", report.suite_ref.clone()),
        crate::evidence_chain::ChainContextRef::new(
            "actor",
            canonical_hash(&record("turn-journal-actor", vec![string(actor_id)]))?,
        ),
        crate::evidence_chain::ChainContextRef::new("observation", observation.observation_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("step", observation.step_ref.clone()),
        crate::evidence_chain::ChainContextRef::new("before-state", observation.before_state_hash.clone()),
        crate::evidence_chain::ChainContextRef::new("after-state", observation.after_state_hash.clone()),
    ];
    for (event, event_ref) in observation.events.iter().zip(observation.event_refs.iter()) {
        let computed_event_ref = canonical_hash(event)?;
        if computed_event_ref != *event_ref {
            return Err(MoltenError::invalid_harness(
                "turn journal observation event refs do not match canonical events",
            ));
        }
        let label = match super::schema::event_boundary(event) {
            super::schema::EventBoundary::PolicyDecision => "admission",
            super::schema::EventBoundary::EffectRequest | super::schema::EventBoundary::EffectResponse => "effect-log",
            _ => "trace",
        };
        refs.push(crate::evidence_chain::ChainContextRef::new(label, event_ref.clone()));
    }
    Ok(refs)
}

fn turn_journal_producer() -> Result<crate::evidence_chain::ChainProducer> {
    Ok(crate::evidence_chain::ChainProducer::new(
        "molten-turn-journal",
        canonical_hash(&record("turn-journal-producer-key", vec![string("molten")]))?,
    ))
}

fn turn_journals_value(evidence: &TurnJournalEvidence) -> IoValue {
    record("turn-journals", vec![
        record("profile", vec![string("per-actor-local-turn-journal")]),
        record("journals", vec![sequence(evidence.journals.iter().map(turn_journal_value).collect())]),
        record("checks", vec![sequence(
            [
                "turn-journal-chains",
                "turn-journal-input-binding",
                "turn-journal-admission-binding",
                "turn-journal-state-binding",
                "turn-journal-no-global-head",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

fn turn_journal_value(journal: &TurnJournalChainEvidence) -> IoValue {
    record("turn-journal", vec![
        record("actor", vec![string(&journal.actor_id)]),
        record("links", vec![sequence(journal.link_values.clone())]),
        record("verify-receipt", vec![journal.verify_receipt_value.clone()]),
        record("predicates", vec![sequence(journal.predicate_values.clone())]),
        record("checks", vec![sequence(
            [
                "turn-journal-chains",
                "turn-journal-input-binding",
                "turn-journal-admission-binding",
                "turn-journal-state-binding",
                "turn-journal-no-global-head",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

fn parse_turn_journals(value: &Value<IoValue>, report_ref: &str, suite_ref: &str) -> Result<TurnJournalEvidence> {
    let value = value_to_iovalue(value);
    let journals_record = simple_record(&value, "turn-journals", 3)?;
    let profile = required_record_string(&journals_record[0], "profile", "turn journal profile")?;
    if profile != "per-actor-local-turn-journal" {
        return Err(MoltenError::invalid_harness(format!("unsupported turn journal profile {profile}")));
    }
    let journal_values = required_record_values(&journals_record[1], "journals")?;
    let checks = parse_checks(&journals_record[2])?;
    require_turn_journal_checks(&checks)?;
    let journals = parse_turn_journal_set(&journal_values, report_ref, suite_ref)?;
    Ok(TurnJournalEvidence {
        aggregate_ref: canonical_hash(&value)?,
        journals,
    })
}

fn require_turn_journal_checks(checks: &[String]) -> Result<()> {
    require_check(checks, "turn-journal-chains")?;
    require_check(checks, "turn-journal-input-binding")?;
    require_check(checks, "turn-journal-admission-binding")?;
    require_check(checks, "turn-journal-state-binding")?;
    require_check(checks, "turn-journal-no-global-head")?;
    Ok(())
}

fn parse_turn_journal_set(
    journal_values: &[IoValue],
    report_ref: &str,
    suite_ref: &str,
) -> Result<Vec<TurnJournalChainEvidence>> {
    let mut journals = Vec::with_capacity(journal_values.len());
    let mut actor_ids = OrderedMap::new();
    for journal_value in journal_values {
        let journal = parse_turn_journal(journal_value, report_ref, suite_ref)?;
        if actor_ids.insert(journal.actor_id.clone(), ()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate turn journal for actor {}", journal.actor_id)));
        }
        journals.push(journal);
    }
    if journals.is_empty() {
        return Err(MoltenError::invalid_harness("turn journal evidence must contain at least one actor journal"));
    }
    Ok(journals)
}

fn parse_turn_journal(value: &IoValue, report_ref: &str, suite_ref: &str) -> Result<TurnJournalChainEvidence> {
    let journal_record = simple_record(value, "turn-journal", 5)?;
    let actor_id = required_record_string(&journal_record[0], "actor", "turn journal actor")?;
    let link_values = required_record_values(&journal_record[1], "links")?;
    let verify_receipt_value = required_record_value(&journal_record[2], "verify-receipt")?;
    let predicate_values = required_record_values(&journal_record[3], "predicates")?;
    let checks = parse_checks(&journal_record[4])?;
    require_turn_journal_checks(&checks)?;
    let parsed_links = parse_turn_journal_links(&link_values, &actor_id, report_ref, suite_ref)?;
    let predicate_receipts = parse_turn_journal_predicates(&predicate_values)?;
    let predicate_receipt_refs =
        predicate_receipts.iter().map(|receipt| receipt.receipt_ref.clone()).collect::<Vec<_>>();
    validate_turn_journal_verify_receipt(
        &verify_receipt_value,
        &parsed_links.links[0].chain,
        &parsed_links.link_refs,
        &parsed_links.payload_refs,
        &predicate_receipt_refs,
    )?;
    Ok(TurnJournalChainEvidence {
        actor_id,
        link_refs: parsed_links.link_refs,
        payload_refs: parsed_links.payload_refs,
        verify_receipt_ref: canonical_hash(&verify_receipt_value)?,
        predicate_receipt_refs,
        link_values,
        verify_receipt_value,
        predicate_values,
    })
}

struct ParsedTurnJournalLinks {
    links: Vec<crate::evidence_chain::ChainLink>,
    link_refs: Vec<String>,
    payload_refs: Vec<String>,
}

fn parse_turn_journal_links(
    link_values: &[IoValue],
    actor_id: &str,
    report_ref: &str,
    suite_ref: &str,
) -> Result<ParsedTurnJournalLinks> {
    if link_values.is_empty() {
        return Err(MoltenError::invalid_harness("turn journal must contain at least one link"));
    }
    let mut links = Vec::with_capacity(link_values.len());
    let mut link_refs = Vec::with_capacity(link_values.len());
    let mut payload_refs = Vec::with_capacity(link_values.len());
    for (position, link_value) in link_values.iter().enumerate() {
        let link = crate::evidence_chain::parse_chain_link(link_value)?;
        validate_turn_journal_link(TurnJournalLinkValidation {
            link: &link,
            position,
            link_refs: &link_refs,
            actor_id,
            report_ref,
            suite_ref,
        })?;
        payload_refs.push(link.payload.artifact_ref.clone());
        link_refs.push(link.link_ref.clone());
        links.push(link);
    }
    Ok(ParsedTurnJournalLinks {
        links,
        link_refs,
        payload_refs,
    })
}

struct TurnJournalLinkValidation<'a> {
    link: &'a crate::evidence_chain::ChainLink,
    position: usize,
    link_refs: &'a [String],
    actor_id: &'a str,
    report_ref: &'a str,
    suite_ref: &'a str,
}

fn validate_turn_journal_link(input: TurnJournalLinkValidation<'_>) -> Result<()> {
    if input.link.chain.scope != "harness-turn-journal"
        || input.link.chain.id != input.actor_id
        || input.link.chain.epoch != input.report_ref
    {
        return Err(MoltenError::invalid_harness(
            "turn journal link scope must be per actor and per report, not global",
        ));
    }
    if input.link.sequence != input.position as u64 {
        return Err(MoltenError::invalid_harness("turn journal link sequence is not contiguous"));
    }
    validate_turn_journal_previous_ref(input.link, input.position, input.link_refs)?;
    require_context_ref(&input.link.context_refs, "report", input.report_ref)?;
    require_context_ref(&input.link.context_refs, "suite", input.suite_ref)?;
    require_context_ref_kind(&input.link.context_refs, "step")?;
    require_context_ref_kind(&input.link.context_refs, "before-state")?;
    require_context_ref_kind(&input.link.context_refs, "after-state")?;
    require_context_ref_kind(&input.link.context_refs, "admission")?;
    require_context_ref_kind(&input.link.context_refs, "trace")?;
    Ok(())
}

fn validate_turn_journal_previous_ref(
    link: &crate::evidence_chain::ChainLink,
    position: usize,
    link_refs: &[String],
) -> Result<()> {
    if position == 0 {
        if link.previous_link_ref.is_some() {
            return Err(MoltenError::invalid_harness("turn journal genesis link must not name a previous link"));
        }
        return Ok(());
    }
    if link.previous_link_ref.as_deref() != link_refs.get(position - 1).map(String::as_str) {
        return Err(MoltenError::invalid_harness("turn journal link does not bind previous actor-local turn"));
    }
    Ok(())
}

fn parse_turn_journal_predicates(
    predicate_values: &[IoValue],
) -> Result<Vec<crate::evidence_chain::ChainPredicateReceipt>> {
    let receipts = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?;
    require_chain_predicate_kind(&receipts, crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE)?;
    require_chain_predicate_kind(&receipts, crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE)?;
    require_chain_predicate_kind(&receipts, crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE)?;
    Ok(receipts)
}

fn validate_turn_journal_verify_receipt(
    value: &IoValue,
    chain: &crate::evidence_chain::ChainScope,
    link_refs: &[String],
    payload_refs: &[String],
    predicate_receipt_refs: &[String],
) -> Result<()> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("turn journal missing chain verify receipt"))?;
    let schema = required_string(&receipt[0], "turn journal verify receipt schema")?;
    if schema != EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported turn journal verify receipt schema {schema}; expected {EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA}"
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "turn journal verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "turn journal verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = required_chain_scope(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(MoltenError::invalid_harness("turn journal verify receipt chain scope mismatch"));
    }
    let anchor = required_record_optional_hash(&receipt[3], "anchor", "turn journal anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("turn journal verify receipt missing anchor"))?;
    let expected_head = required_record_optional_hash(&receipt[4], "expected-head", "turn journal expected head")?
        .ok_or_else(|| MoltenError::invalid_harness("turn journal verify receipt missing expected head"))?;
    if Some(&anchor) != link_refs.first() || Some(&expected_head) != link_refs.last() {
        return Err(MoltenError::invalid_harness("turn journal verify receipt does not bind actor-local anchor/head"));
    }
    if required_record_hash_sequence(&receipt[5], "discovered-heads")? != vec![expected_head] {
        return Err(MoltenError::invalid_harness("turn journal verify receipt discovered head mismatch"));
    }
    if required_record_hash_sequence(&receipt[6], "verified-links")? != link_refs {
        return Err(MoltenError::invalid_harness("turn journal verify receipt link range mismatch"));
    }
    if required_record_hash_sequence(&receipt[7], "payloads")? != payload_refs {
        return Err(MoltenError::invalid_harness("turn journal verify receipt payload refs mismatch"));
    }
    if required_record_hash_sequence(&receipt[8], "predicates")? != predicate_receipt_refs {
        return Err(MoltenError::invalid_harness("turn journal verify receipt predicate refs mismatch"));
    }
    Ok(())
}

fn require_context_ref(
    context_refs: &[crate::evidence_chain::ChainContextRef],
    label: &str,
    expected: &str,
) -> Result<()> {
    if context_refs.iter().any(|context| context.label == label && context.artifact_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("turn journal link missing {label} context ref {expected}")))
    }
}

fn require_context_ref_kind(context_refs: &[crate::evidence_chain::ChainContextRef], label: &str) -> Result<()> {
    if context_refs.iter().any(|context| context.label == label) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("turn journal link missing {label} context ref")))
    }
}

fn repro_verify_checks_value() -> IoValue {
    record("checks", vec![sequence(
        [
            "sealed-bundle",
            "embedded-report",
            "embedded-gate-receipt",
            "report-validation",
            "deterministic-replay",
            "gate-receipt-recomputed",
        ]
        .iter()
        .map(|name| record("check", vec![string(*name), string("pass")]))
        .collect(),
    )])
}

fn checks_value() -> IoValue {
    record("checks", vec![sequence(
        PASS_CHECKS.iter().map(|name| record("check", vec![string(*name), string("pass")])).collect(),
    )])
}

struct CoreRefs<'a> {
    validation: &'a ValidationReceipt,
    replay: &'a ReplayReceipt,
    report: &'a str,
    suite: &'a str,
    final_state: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationReceipt {
    report_ref: String,
    suite_ref: String,
    final_state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayReceipt {
    expected_report_ref: String,
    actual_report_ref: String,
    final_state_hash: String,
    verify_ref: String,
}

fn parse_validation(value: &Value<IoValue>) -> Result<ValidationReceipt> {
    let value = value_to_iovalue(value);
    let validation = simple_record(&value, "validation", 7)?;
    let status = required_record_string(&validation[0], "status", "gate validation status")?;
    if status != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate validation status {status}")));
    }
    let report_ref = required_record_hash(&validation[1], "report", "gate validation report ref")?;
    let suite_ref = required_record_hash(&validation[2], "suite", "gate validation suite ref")?;
    let final_state_hash = required_record_hash(&validation[3], "final-state", "gate validation final state hash")?;
    let observations = required_record_u64(&validation[4], "observations", "gate validation observations")?;
    super::schema::parse_actor_registry(&value_to_iovalue(&validation[5]))?;
    let budget = super::schema::parse_budget(&value_to_iovalue(&validation[6]))?;
    if observations != budget.usage.steps {
        return Err(MoltenError::invalid_harness(
            "gate receipt validation observation count does not match budget step usage",
        ));
    }
    Ok(ValidationReceipt {
        report_ref,
        suite_ref,
        final_state_hash,
    })
}

fn parse_replay(value: &Value<IoValue>) -> Result<ReplayReceipt> {
    let value = value_to_iovalue(value);
    let replay = simple_record(&value, "replay", 6)?;
    let status = required_record_string(&replay[0], "status", "gate replay status")?;
    if status != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate replay status {status}")));
    }
    let expected_report_ref = required_record_hash(&replay[1], "expected-report", "gate replay expected report ref")?;
    let actual_report_ref = required_record_hash(&replay[2], "actual-report", "gate replay actual report ref")?;
    let final_state_hash = required_record_hash(&replay[3], "final-state", "gate replay final state hash")?;
    let verify_ref = required_record_hash(&replay[4], "verify-ref", "gate replay verify ref")?;
    let verify_value = value_to_iovalue(&replay[5]);
    validate_harness_replay_verify_value(
        &verify_value,
        &verify_ref,
        &expected_report_ref,
        &actual_report_ref,
        &final_state_hash,
    )?;
    Ok(ReplayReceipt {
        expected_report_ref,
        actual_report_ref,
        final_state_hash,
        verify_ref,
    })
}

fn validate_harness_replay_verify_value(
    value: &IoValue,
    expected_verify_ref: &str,
    expected_report_ref: &str,
    actual_report_ref: &str,
    final_state_hash: &str,
) -> Result<()> {
    let actual_verify_ref = canonical_hash(value)?;
    if actual_verify_ref != expected_verify_ref {
        return Err(MoltenError::invalid_harness("gate replay verify ref does not match embedded value"));
    }
    let receipt = simple_record(value, "deterministic-replay-verify-v1", 7)?;
    let schema = required_string(&receipt[0], "deterministic replay verify schema")?;
    if schema != DETERMINISTIC_REPLAY_VERIFY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic replay verify schema {schema}; expected {DETERMINISTIC_REPLAY_VERIFY_SCHEMA}"
        )));
    }
    let decision = required_string(&receipt[1], "deterministic replay verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic replay verify decision {decision}"
        )));
    }
    let verify_expected_report =
        required_record_hash(&receipt[2], "expected-report-ref", "deterministic replay expected report")?;
    let verify_actual_report =
        required_record_hash(&receipt[3], "actual-report-ref", "deterministic replay actual report")?;
    let verify_final_state = required_record_hash(&receipt[4], "final-state-ref", "deterministic replay final state")?;
    let divergence = required_record_string(&receipt[5], "divergence", "deterministic replay divergence")?;
    if divergence != "none" {
        return Err(MoltenError::invalid_harness(format!(
            "deterministic replay verify divergence must be none, got {divergence}"
        )));
    }
    let checks = parse_checks(&receipt[6])?;
    require_check(&checks, "report-replayed")?;
    require_check(&checks, "final-state-bound")?;
    require_check(&checks, "no-divergence")?;
    if verify_expected_report != expected_report_ref
        || verify_actual_report != actual_report_ref
        || verify_final_state != final_state_hash
    {
        return Err(MoltenError::invalid_harness("deterministic replay verify refs do not match gate replay refs"));
    }
    Ok(())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks_record = simple_record(&value, "checks", 1)?;
    let check_values = required_sequence(&checks_record[0], "gate checks")?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check_value in check_values.iter() {
        let check_value = value_to_iovalue(check_value);
        let check = simple_record(&check_value, "check", 2)?;
        let name = required_string(&check[0], "gate check name")?;
        let status = required_string(&check[1], "gate check status")?;
        if status != "pass" {
            return Err(MoltenError::invalid_harness(format!("gate check {name} status is {status}")));
        }
        checks.push(name);
    }
    Ok(checks)
}

fn require_check(checks: &[String], expected: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("gate receipt missing {expected} check")))
    }
}

fn require_all_checks(checks: &[String]) -> Result<()> {
    for expected in PASS_CHECKS.iter().copied() {
        require_check(checks, expected)?;
    }
    Ok(())
}

fn require_core_refs(input: &CoreRefs<'_>) -> Result<()> {
    if input.report != input.validation.report_ref
        || input.report != input.replay.expected_report_ref
        || input.report != input.replay.actual_report_ref
    {
        return Err(MoltenError::invalid_harness("gate receipt report refs are inconsistent"));
    }
    if input.suite != input.validation.suite_ref {
        return Err(MoltenError::invalid_harness("gate receipt suite refs are inconsistent"));
    }
    if input.final_state != input.validation.final_state_hash || input.final_state != input.replay.final_state_hash {
        return Err(MoltenError::invalid_harness("gate receipt final state refs are inconsistent"));
    }
    Ok(())
}

fn require_link_context(
    link: &crate::evidence_chain::ChainLink,
    report_ref: &str,
    suite_ref: &str,
    final_state_hash: &str,
) -> Result<()> {
    if link.payload.artifact_ref != report_ref {
        return Err(MoltenError::invalid_harness("gate chain evidence payload does not bind the gate report ref"));
    }
    if !link
        .context_refs
        .iter()
        .any(|context| context.label == "suite" && context.artifact_ref == suite_ref)
    {
        return Err(MoltenError::invalid_harness("gate chain evidence context does not bind the gate suite ref"));
    }
    if !link
        .context_refs
        .iter()
        .any(|context| context.label == "final-state" && context.artifact_ref == final_state_hash)
    {
        return Err(MoltenError::invalid_harness("gate chain evidence context does not bind the gate final state ref"));
    }
    Ok(())
}

fn validate_tool_record(value: &Value<IoValue>) -> Result<()> {
    let value = value_to_iovalue(value);
    let tool = simple_record(&value, "tool", 2)?;
    let name = required_string(&tool[0], "gate receipt tool name")?;
    if name != "molten" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate receipt tool {name}")));
    }
    let version = required_string(&tool[1], "gate receipt tool version")?;
    if version.is_empty() {
        return Err(MoltenError::invalid_harness("gate receipt tool version must not be empty"));
    }
    Ok(())
}

fn parse_artifact_refs(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let artifact_refs = simple_record(&value, "artifact-refs", 1)?;
    let ref_values = required_sequence(&artifact_refs[0], "gate receipt artifact refs")?;
    let mut refs = Vec::with_capacity(ref_values.len());
    for ref_value in ref_values.iter() {
        let ref_value = value_to_iovalue(ref_value);
        let artifact_ref = simple_record(&ref_value, "artifact-ref", 2)?;
        refs.push((
            required_string(&artifact_ref[0], "artifact ref kind")?,
            required_hash(&artifact_ref[1], "artifact ref value")?,
        ));
    }
    Ok(refs)
}

fn require_artifact_ref(refs: &[(String, String)], kind: &str, expected: &str) -> Result<()> {
    if refs.iter().any(|(actual_kind, actual_ref)| actual_kind == kind && actual_ref == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("gate receipt artifact refs missing {kind} ref {expected}")))
    }
}

fn require_kinds(refs: &[(String, String)], expected: &[&str]) -> Result<()> {
    for kind in expected.iter().copied() {
        require_artifact_kind(refs, kind)?;
    }
    Ok(())
}

fn require_artifact_kind(refs: &[(String, String)], kind: &str) -> Result<()> {
    if refs.iter().any(|(actual_kind, _)| actual_kind == kind) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("gate receipt artifact refs missing {kind} ref")))
    }
}

fn required_record_string(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], field)
}

fn required_record_hash(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_hash(&record[0], field)
}

fn required_record_optional_hash(value: &Value<IoValue>, label: &str, field: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        required_hash(&some[0], field).map(Some)
    } else {
        Err(MoltenError::invalid_harness(format!("expected <none> or <some ref> for {field}")))
    }
}

fn required_record_hash_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], label)?;
    values.iter().map(|value| required_hash(value, label)).collect()
}

fn required_record_value(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    Ok(value_to_iovalue(&record[0]))
}

fn required_record_values(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let values = required_sequence(&record[0], label)?;
    Ok(values.iter().map(value_to_iovalue).collect())
}

fn required_record_u64(value: &Value<IoValue>, label: &str, field: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_u64(&record[0], field)
}

fn required_chain_scope(value: &Value<IoValue>) -> Result<crate::evidence_chain::ChainScope> {
    let value = value_to_iovalue(value);
    let chain = simple_record(&value, "chain", 3)?;
    Ok(crate::evidence_chain::ChainScope::new(
        required_record_string(&chain[0], "scope", "chain scope")?,
        required_record_string(&chain[1], "id", "chain id")?,
        required_record_string(&chain[2], "epoch", "chain epoch")?,
    ))
}

fn simple_record<'a>(value: &'a IoValue, label: &str, arity: usize) -> Result<Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_hash(value: &Value<IoValue>, field: &str) -> Result<String> {
    let hash = required_string(value, field)?;
    validate_content_ref(&hash).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {hash}: {error}"))
    })?;
    Ok(hash)
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}
