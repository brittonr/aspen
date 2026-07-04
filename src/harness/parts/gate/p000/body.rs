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
    "ucan-verification-receipt-binding",
    "basalt-enforcement-receipt-binding",
    "grant-ref-binding",
    "derived-grant-ref-binding",
    "request-ref-binding",
    "fixture-authority-evidence-only",
    "authority-replay-evidence-only",
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
    "ucan-verification-receipts",
    "derived-grants",
    "basalt-enforcement-receipts",
    "authority-requests",
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
    pub capability_ucan_verification_receipts_ref: String,
    pub capability_derived_grants_ref: String,
    pub capability_authority_receipts_ref: String,
    pub capability_request_refs_ref: String,
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
            super::schema::ReproBundleKind::Report => {
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
            super::schema::ReproBundleKind::Failure => Err(MoltenError::invalid_harness(format!(
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
