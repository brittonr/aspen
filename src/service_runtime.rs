use std::collections::BTreeMap;
use std::collections::BTreeSet;

use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::SERVICE_READINESS_ASSERTION_SCHEMA;
use crate::preserves_rail::SERVICE_REPLAY_IDENTITY_SCHEMA;
use crate::preserves_rail::SERVICE_RUNTIME_REPORT_SCHEMA;
use crate::preserves_rail::SERVICE_RUNTIME_SUITE_SCHEMA;
use crate::preserves_rail::SERVICE_TURN_CONTEXT_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::value_to_iovalue;
use crate::runtime::RuntimeEvent;
use crate::runtime::RuntimeState;
use crate::runtime::RuntimeStep;
use crate::runtime::RuntimeValue;
use crate::service_records;
use crate::service_records::ServiceDemand;
use crate::service_records::ServiceLifecycleReceiptInput;
use crate::service_records::ServiceManifest;
use crate::service_records::ServiceStatus;
use crate::service_records::ServiceStatusInput;

const MAX_SERVICE_RUNTIME_ITEMS: usize = 4096;
const MAX_SERVICE_RUNTIME_CHECKS: usize = 256;
const MAX_DEPENDENCY_PASSES: usize = 4096;

const _: () = assert!(MAX_SERVICE_RUNTIME_ITEMS <= 100_000);
const _: () = assert!(MAX_SERVICE_RUNTIME_CHECKS <= 10_000);
const _: () = assert!(MAX_DEPENDENCY_PASSES <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRuntimeEvidenceInput {
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub source_gate_refs: Vec<String>,
    pub scheduler_ref: Option<String>,
    pub effect_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRuntimeSuiteInput {
    pub manifests: Vec<IOValue>,
    pub demands: Vec<IOValue>,
    pub statuses: Vec<IOValue>,
    pub evidence: ServiceRuntimeEvidenceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRuntimeSuite {
    pub suite_ref: String,
    pub manifests: Vec<ServiceManifest>,
    pub demands: Vec<ServiceDemand>,
    pub statuses: Vec<ServiceStatus>,
    pub evidence: ServiceRuntimeEvidenceInput,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRuntimeRun {
    pub suite_ref: String,
    pub suite_value: IOValue,
    pub report_ref: String,
    pub lifecycle_receipts: Vec<IOValue>,
    pub statuses: Vec<IOValue>,
    pub readiness_assertions: Vec<IOValue>,
    pub replay_identities: Vec<IOValue>,
    pub turn_contexts: Vec<IOValue>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRuntimeReplay {
    pub expected_report_ref: String,
    pub actual_report_ref: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemandOutcome {
    lifecycle_receipt: IOValue,
    status: Option<IOValue>,
    readiness: Option<IOValue>,
    replay_identity: Option<IOValue>,
    turn_context: Option<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundedValues {
    label: &'static str,
    values: Vec<IOValue>,
}

impl BoundedValues {
    fn empty(label: &'static str) -> Self {
        Self {
            label,
            values: Vec::new(),
        }
    }

    fn from_values(label: &'static str, values: Vec<IOValue>) -> Self {
        Self { label, values }
    }

    fn push(&mut self, value: IOValue) -> Result<()> {
        let total = self
            .values
            .len()
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness(format!("{} count overflow", self.label)))?;
        ensure_count_at_most(total, self.label)?;
        self.values.push(value);
        Ok(())
    }

    fn as_slice(&self) -> &[IOValue] {
        &self.values
    }

    fn into_values(self) -> Vec<IOValue> {
        self.values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServiceRuntimeArtifacts {
    lifecycle_receipts: BoundedValues,
    statuses: BoundedValues,
    readiness_assertions: BoundedValues,
    replay_identities: BoundedValues,
    turn_contexts: BoundedValues,
}

impl ServiceRuntimeArtifacts {
    fn new(statuses: Vec<IOValue>) -> Self {
        Self {
            lifecycle_receipts: BoundedValues::empty("service lifecycle receipts"),
            statuses: BoundedValues::from_values("service statuses", statuses),
            readiness_assertions: BoundedValues::empty("service readiness assertions"),
            replay_identities: BoundedValues::empty("service replay identities"),
            turn_contexts: BoundedValues::empty("service turn contexts"),
        }
    }

    fn push_outcome(&mut self, outcome: DemandOutcome) -> Result<()> {
        self.lifecycle_receipts.push(outcome.lifecycle_receipt)?;
        if let Some(status) = outcome.status {
            self.statuses.push(status)?;
        }
        if let Some(readiness) = outcome.readiness {
            self.readiness_assertions.push(readiness)?;
        }
        if let Some(replay_identity) = outcome.replay_identity {
            self.replay_identities.push(replay_identity)?;
        }
        if let Some(turn_context) = outcome.turn_context {
            self.turn_contexts.push(turn_context)?;
        }
        Ok(())
    }
}

pub fn service_runtime_suite_value(input: &ServiceRuntimeSuiteInput) -> Result<IOValue> {
    validate_suite_input(input)?;
    Ok(record("service-runtime-suite-v1", vec![
        string(SERVICE_RUNTIME_SUITE_SCHEMA),
        record("manifests", vec![sequence(input.manifests.clone())]),
        record("demands", vec![sequence(input.demands.clone())]),
        record("statuses", vec![sequence(input.statuses.clone())]),
        evidence_value(&input.evidence),
        checks_value(&[
            "canonical-service-runtime-suite",
            "explicit-startup-evidence",
            "no-ambient-supervisor",
            "bounded-inputs",
        ]),
    ]))
}

pub fn parse_service_runtime_suite(value: &IOValue) -> Result<ServiceRuntimeSuite> {
    let fields = value
        .collect_simple_record("service-runtime-suite-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-runtime-suite-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_RUNTIME_SUITE_SCHEMA, "service runtime suite schema")?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "canonical-service-runtime-suite", "service runtime suite")?;
    let manifest_values = parse_iovalue_sequence(&fields[1], "manifests")?;
    let demand_values = parse_iovalue_sequence(&fields[2], "demands")?;
    let status_values = parse_iovalue_sequence(&fields[3], "statuses")?;
    let manifests = manifest_values.iter().map(service_records::parse_service_manifest).collect::<Result<Vec<_>>>()?;
    let demands = demand_values.iter().map(service_records::parse_service_demand).collect::<Result<Vec<_>>>()?;
    let statuses = status_values.iter().map(service_records::parse_service_status).collect::<Result<Vec<_>>>()?;
    let evidence = parse_evidence(&fields[4])?;
    validate_runtime_evidence(&evidence)?;
    Ok(ServiceRuntimeSuite {
        suite_ref: canonical_hash(value)?,
        manifests,
        demands,
        statuses,
        evidence,
        value: value.clone(),
    })
}

pub fn run_service_runtime_suite_value(value: &IOValue) -> Result<ServiceRuntimeRun> {
    let suite = parse_service_runtime_suite(value)?;
    run_service_runtime_suite(&suite)
}

pub fn run_service_runtime_suite(suite: &ServiceRuntimeSuite) -> Result<ServiceRuntimeRun> {
    let mut manifests = BTreeMap::new();
    for manifest in &suite.manifests {
        if manifests.insert(manifest.service_id.clone(), manifest.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate service manifest for {}",
                manifest.service_id
            )));
        }
    }
    let mut ready_statuses = ready_status_map(&suite.statuses)?;
    let initial_status_values = suite.statuses.iter().map(|status| status.value.clone()).collect::<Vec<_>>();
    let mut demands = suite.demands.clone();
    demands.sort_by(|left, right| {
        left.service_id.cmp(&right.service_id).then_with(|| left.demand_ref.cmp(&right.demand_ref))
    });
    let mut artifacts = ServiceRuntimeArtifacts::new(initial_status_values);
    let mut runtime = RuntimeState::new(1);
    let has_cycle = dependency_cycle_exists(&manifests)?;
    let mut pending = demands;
    let mut passes = 0usize;
    while !pending.is_empty() && !has_cycle {
        passes = passes
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("service dependency pass count overflow"))?;
        if passes > MAX_DEPENDENCY_PASSES {
            return Err(MoltenError::invalid_harness("service dependency evaluation exceeded pass bound"));
        }
        let mut next_pending = Vec::with_capacity(pending.len());
        let mut is_progress_made = false;
        for demand in pending {
            let Some(manifest) = manifests.get(&demand.service_id) else {
                artifacts.push_outcome(missing_manifest_outcome(&demand)?)?;
                continue;
            };
            if manifest_ref_mismatch(&demand, manifest) {
                artifacts.push_outcome(deny_outcome(
                    &demand,
                    Some(manifest),
                    "demand manifest ref does not match resolved manifest",
                )?)?;
                continue;
            }
            let dependency_refs = dependency_status_refs(manifest, &ready_statuses);
            if dependency_refs.len() == manifest.dependencies.len() {
                let admission_diagnostics = startup_admission_diagnostics(&suite.evidence);
                if admission_diagnostics.is_empty() {
                    let outcome = start_outcome(&mut runtime, &suite.evidence, &demand, manifest, dependency_refs)?;
                    if let Some(status) = outcome.status.as_ref() {
                        let parsed = service_records::parse_service_status(status)?;
                        ready_statuses.insert(parsed.service_id.clone(), parsed.status_ref);
                    }
                    artifacts.push_outcome(outcome)?;
                    is_progress_made = true;
                } else {
                    artifacts.push_outcome(deny_outcome(
                        &demand,
                        Some(manifest),
                        &admission_diagnostics.join("; "),
                    )?)?;
                }
            } else {
                next_pending.push(demand);
            }
        }
        if !is_progress_made {
            pending = next_pending;
            break;
        }
        pending = next_pending;
    }
    for demand in pending {
        let manifest = manifests.get(&demand.service_id);
        let diagnostic = if has_cycle {
            "dependency cycle detected"
        } else {
            "required service dependency is not ready"
        };
        let outcome = if has_cycle {
            dependency_deny_outcome(&demand, manifest, diagnostic)?
        } else {
            dependency_wait_outcome(&demand, manifest, diagnostic)?
        };
        artifacts.push_outcome(outcome)?;
    }
    let report_value = service_runtime_report_value(ReportValueInput {
        suite_value: &suite.value,
        lifecycle_receipts: artifacts.lifecycle_receipts.as_slice(),
        statuses: artifacts.statuses.as_slice(),
        readiness_assertions: artifacts.readiness_assertions.as_slice(),
        replay_identities: artifacts.replay_identities.as_slice(),
        turn_contexts: artifacts.turn_contexts.as_slice(),
    })?;
    let ServiceRuntimeArtifacts {
        lifecycle_receipts,
        statuses,
        readiness_assertions,
        replay_identities,
        turn_contexts,
    } = artifacts;
    Ok(ServiceRuntimeRun {
        suite_ref: suite.suite_ref.clone(),
        suite_value: suite.value.clone(),
        report_ref: canonical_hash(&report_value)?,
        lifecycle_receipts: lifecycle_receipts.into_values(),
        statuses: statuses.into_values(),
        readiness_assertions: readiness_assertions.into_values(),
        replay_identities: replay_identities.into_values(),
        turn_contexts: turn_contexts.into_values(),
        value: report_value,
    })
}

pub fn replay_service_runtime_report(value: &IOValue) -> Result<ServiceRuntimeReplay> {
    let report = parse_service_runtime_report(value)?;
    let rerun = run_service_runtime_suite_value(&report.suite_value)?;
    let expected_report_ref = canonical_hash(value)?;
    let decision = if expected_report_ref == rerun.report_ref {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    if decision == "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "service runtime replay divergence: expected {expected_report_ref}, got {}",
            rerun.report_ref
        )));
    }
    Ok(ServiceRuntimeReplay {
        expected_report_ref,
        actual_report_ref: rerun.report_ref,
        decision,
    })
}

pub fn parse_service_runtime_report(value: &IOValue) -> Result<ServiceRuntimeRun> {
    let fields = value
        .collect_simple_record("service-runtime-report-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-runtime-report-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_RUNTIME_REPORT_SCHEMA, "service runtime report schema")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-service-runtime-report", "service runtime report")?;
    let suite_value = record_iovalue(&fields[1], "suite")?;
    let suite_ref = canonical_hash(&suite_value)?;
    let lifecycle_receipts = parse_iovalue_sequence(&fields[2], "lifecycle")?;
    let statuses = parse_iovalue_sequence(&fields[3], "statuses")?;
    let readiness_assertions = parse_iovalue_sequence(&fields[4], "readiness")?;
    let replay_identities = parse_iovalue_sequence(&fields[5], "replay-identities")?;
    let turn_contexts = parse_iovalue_sequence(&fields[6], "turn-contexts")?;
    Ok(ServiceRuntimeRun {
        suite_ref,
        suite_value,
        report_ref: canonical_hash(value)?,
        lifecycle_receipts,
        statuses,
        readiness_assertions,
        replay_identities,
        turn_contexts,
        value: value.clone(),
    })
}

pub fn service_runtime_summary(value: &IOValue) -> Result<String> {
    if let Ok(report) = parse_service_runtime_report(value) {
        let pass_count = report
            .lifecycle_receipts
            .iter()
            .filter_map(|receipt| service_records::parse_service_lifecycle_receipt(receipt).ok())
            .filter(|receipt| receipt.decision == "pass")
            .count();
        return Ok(format!(
            "service runtime report ref={} suite={} lifecycle={} pass={} statuses={} readiness={}",
            report.report_ref,
            report.suite_ref,
            report.lifecycle_receipts.len(),
            pass_count,
            report.statuses.len(),
            report.readiness_assertions.len()
        ));
    }
    if let Ok(suite) = parse_service_runtime_suite(value) {
        return Ok(format!(
            "service runtime suite ref={} manifests={} demands={} statuses={}",
            suite.suite_ref,
            suite.manifests.len(),
            suite.demands.len(),
            suite.statuses.len()
        ));
    }
    service_records::service_summary(value)
}

pub fn two_service_suite_value() -> Result<IOValue> {
    let evidence = ServiceRuntimeEvidenceInput {
        authority_refs: vec![synthetic_ref("service-authority")?],
        policy_refs: vec![synthetic_ref("service-policy")?],
        resource_refs: vec![synthetic_ref("service-resource")?],
        effect_profile_refs: vec![synthetic_ref("service-effect")?],
        source_gate_refs: vec![synthetic_ref("service-source-gate")?],
        scheduler_ref: Some(synthetic_ref("service-scheduler")?),
        effect_log_refs: vec![synthetic_ref("service-effect-log")?],
    };
    let backend_manifest = service_records::service_manifest_value(&service_records::ServiceManifestInput {
        service_id: "svc:backend".to_string(),
        owner_authority_ref: evidence.authority_refs[0].clone(),
        target_ref: synthetic_ref("backend-target")?,
        dependencies: Vec::new(),
        provided_assertion_refs: vec![synthetic_ref("backend-ready-pattern")?],
        restart_policy_ref: synthetic_ref("backend-restart")?,
        policy_refs: evidence.policy_refs.clone(),
        resource_refs: evidence.resource_refs.clone(),
        effect_profile_refs: evidence.effect_profile_refs.clone(),
    })?;
    let frontend_manifest = service_records::service_manifest_value(&service_records::ServiceManifestInput {
        service_id: "svc:frontend".to_string(),
        owner_authority_ref: evidence.authority_refs[0].clone(),
        target_ref: synthetic_ref("frontend-target")?,
        dependencies: vec!["svc:backend".to_string()],
        provided_assertion_refs: vec![synthetic_ref("frontend-ready-pattern")?],
        restart_policy_ref: synthetic_ref("frontend-restart")?,
        policy_refs: evidence.policy_refs.clone(),
        resource_refs: evidence.resource_refs.clone(),
        effect_profile_refs: evidence.effect_profile_refs.clone(),
    })?;
    let backend_demand = service_records::service_demand_value(&service_records::ServiceDemandInput {
        demand_id: "demand:backend".to_string(),
        service_id: "svc:backend".to_string(),
        requester_ref: synthetic_ref("operator")?,
        manifest_ref: Some(canonical_hash(&backend_manifest)?),
        policy_refs: evidence.policy_refs.clone(),
    })?;
    let frontend_demand = service_records::service_demand_value(&service_records::ServiceDemandInput {
        demand_id: "demand:frontend".to_string(),
        service_id: "svc:frontend".to_string(),
        requester_ref: synthetic_ref("operator")?,
        manifest_ref: Some(canonical_hash(&frontend_manifest)?),
        policy_refs: evidence.policy_refs.clone(),
    })?;
    service_runtime_suite_value(&ServiceRuntimeSuiteInput {
        manifests: vec![backend_manifest, frontend_manifest],
        demands: vec![backend_demand, frontend_demand],
        statuses: Vec::new(),
        evidence,
    })
}

struct ReportValueInput<'a> {
    suite_value: &'a IOValue,
    lifecycle_receipts: &'a [IOValue],
    statuses: &'a [IOValue],
    readiness_assertions: &'a [IOValue],
    replay_identities: &'a [IOValue],
    turn_contexts: &'a [IOValue],
}

fn service_runtime_report_value(input: ReportValueInput<'_>) -> Result<IOValue> {
    Ok(record("service-runtime-report-v1", vec![
        string(SERVICE_RUNTIME_REPORT_SCHEMA),
        record("suite", vec![input.suite_value.clone()]),
        record("lifecycle", vec![sequence(input.lifecycle_receipts.to_vec())]),
        record("statuses", vec![sequence(input.statuses.to_vec())]),
        record("readiness", vec![sequence(input.readiness_assertions.to_vec())]),
        record("replay-identities", vec![sequence(input.replay_identities.to_vec())]),
        record("turn-contexts", vec![sequence(input.turn_contexts.to_vec())]),
        checks_value(&[
            "canonical-service-runtime-report",
            "replayable-suite-embedded",
            "no-text-evidence",
            "side-effects-recorded",
        ]),
    ]))
}

fn start_outcome(
    runtime: &mut RuntimeState,
    evidence: &ServiceRuntimeEvidenceInput,
    demand: &ServiceDemand,
    manifest: &ServiceManifest,
    dependency_status_refs: Vec<String>,
) -> Result<DemandOutcome> {
    let replay_identity = replay_identity_value(evidence, demand, manifest, &dependency_status_refs)?;
    let replay_identity_ref = canonical_hash(&replay_identity)?;
    let readiness = readiness_assertion_value(demand, manifest, &dependency_status_refs)?;
    let readiness_ref = canonical_hash(&readiness)?;
    let runtime_value = RuntimeValue::new(readiness.clone())?;
    let step = RuntimeStep::Assert {
        actor: manifest.service_id.clone(),
        value: runtime_value,
    };
    let events = runtime.apply_step(&step);
    let turn_context = turn_context_value(demand, manifest, &readiness_ref, &events)?;
    let status = service_records::service_status_value(&ServiceStatusInput {
        service_id: manifest.service_id.clone(),
        state: "ready".to_string(),
        manifest_ref: Some(manifest.manifest_ref.clone()),
        demand_refs: vec![demand.demand_ref.clone()],
        dependency_status_refs,
        readiness_assertion_refs: vec![readiness_ref],
        failure_refs: Vec::new(),
        restart_count: 0,
        monitor_refs: Vec::new(),
        replay_refs: vec![replay_identity_ref],
    })?;
    let status_ref = canonical_hash(&status)?;
    let lifecycle_receipt = service_records::service_lifecycle_receipt_value(&ServiceLifecycleReceiptInput {
        operation: "start".to_string(),
        decision: "pass".to_string(),
        service_id: manifest.service_id.clone(),
        manifest_ref: Some(manifest.manifest_ref.clone()),
        status_ref: Some(status_ref),
        authority_refs: evidence.authority_refs.clone(),
        resource_refs: evidence.resource_refs.clone(),
        effect_profile_refs: evidence.effect_profile_refs.clone(),
        supervision_refs: Vec::new(),
        diagnostics: Vec::new(),
    })?;
    Ok(DemandOutcome {
        lifecycle_receipt,
        status: Some(status),
        readiness: Some(readiness),
        replay_identity: Some(replay_identity),
        turn_context: Some(turn_context),
    })
}

fn missing_manifest_outcome(demand: &ServiceDemand) -> Result<DemandOutcome> {
    deny_outcome(demand, None, "service demand has no matching manifest")
}

fn deny_outcome(demand: &ServiceDemand, manifest: Option<&ServiceManifest>, diagnostic: &str) -> Result<DemandOutcome> {
    let lifecycle_receipt = service_records::service_lifecycle_receipt_value(&ServiceLifecycleReceiptInput {
        operation: "start".to_string(),
        decision: "deny".to_string(),
        service_id: demand.service_id.clone(),
        manifest_ref: manifest.map(|manifest| manifest.manifest_ref.clone()),
        status_ref: None,
        authority_refs: Vec::new(),
        resource_refs: Vec::new(),
        effect_profile_refs: Vec::new(),
        supervision_refs: Vec::new(),
        diagnostics: vec![diagnostic.to_string()],
    })?;
    Ok(DemandOutcome {
        lifecycle_receipt,
        status: None,
        readiness: None,
        replay_identity: None,
        turn_context: None,
    })
}

fn dependency_wait_outcome(
    demand: &ServiceDemand,
    manifest: Option<&ServiceManifest>,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    dependency_resolution_outcome(demand, manifest, "diagnostic", diagnostic)
}

fn dependency_deny_outcome(
    demand: &ServiceDemand,
    manifest: Option<&ServiceManifest>,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    dependency_resolution_outcome(demand, manifest, "deny", diagnostic)
}

fn dependency_resolution_outcome(
    demand: &ServiceDemand,
    manifest: Option<&ServiceManifest>,
    decision: &str,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    let lifecycle_receipt = service_records::service_lifecycle_receipt_value(&ServiceLifecycleReceiptInput {
        operation: "dependency-wait".to_string(),
        decision: decision.to_string(),
        service_id: demand.service_id.clone(),
        manifest_ref: manifest.map(|manifest| manifest.manifest_ref.clone()),
        status_ref: None,
        authority_refs: Vec::new(),
        resource_refs: Vec::new(),
        effect_profile_refs: Vec::new(),
        supervision_refs: Vec::new(),
        diagnostics: vec![diagnostic.to_string()],
    })?;
    Ok(DemandOutcome {
        lifecycle_receipt,
        status: None,
        readiness: None,
        replay_identity: None,
        turn_context: None,
    })
}

fn readiness_assertion_value(
    demand: &ServiceDemand,
    manifest: &ServiceManifest,
    dependency_status_refs: &[String],
) -> Result<IOValue> {
    Ok(record("service-readiness-v1", vec![
        string(SERVICE_READINESS_ASSERTION_SCHEMA),
        record("service-id", vec![string(&manifest.service_id)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("demand", vec![string(&demand.demand_ref)]),
        record("dependencies", vec![refs_sequence(dependency_status_refs)]),
        checks_value(&[
            "service-owned-assertion",
            "dependency-readiness-bound",
            "cleanup-identifiable",
        ]),
    ]))
}

fn replay_identity_value(
    evidence: &ServiceRuntimeEvidenceInput,
    demand: &ServiceDemand,
    manifest: &ServiceManifest,
    dependency_status_refs: &[String],
) -> Result<IOValue> {
    Ok(record("service-replay-identity-v1", vec![
        string(SERVICE_REPLAY_IDENTITY_SCHEMA),
        record("service-id", vec![string(&manifest.service_id)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("demand", vec![string(&demand.demand_ref)]),
        record("dependencies", vec![refs_sequence(dependency_status_refs)]),
        record("authority", vec![refs_sequence(&evidence.authority_refs)]),
        record("policy", vec![refs_sequence(&evidence.policy_refs)]),
        record("resource", vec![refs_sequence(&evidence.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&evidence.effect_profile_refs)]),
        record("source-gate", vec![refs_sequence(&evidence.source_gate_refs)]),
        record("scheduler", vec![optional_ref_value(evidence.scheduler_ref.as_deref())]),
        record("effect-log", vec![refs_sequence(&evidence.effect_log_refs)]),
        checks_value(&[
            "demand-bound",
            "dependency-bound",
            "authority-resource-effect-bound",
            "source-gate-bound",
        ]),
    ]))
}

fn turn_context_value(
    demand: &ServiceDemand,
    manifest: &ServiceManifest,
    readiness_ref: &str,
    events: &[RuntimeEvent],
) -> Result<IOValue> {
    let event_labels = events.iter().map(runtime_event_label).collect::<Vec<_>>();
    Ok(record("service-turn-context-v1", vec![
        string(SERVICE_TURN_CONTEXT_SCHEMA),
        record("service-id", vec![string(&manifest.service_id)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("demand", vec![string(&demand.demand_ref)]),
        record("readiness", vec![string(readiness_ref)]),
        record("runtime-events", vec![sequence(event_labels.into_iter().map(string).collect())]),
        checks_value(&["actor-scoped", "owned-assertion-committed", "turn-context-bound"]),
    ]))
}

fn runtime_event_label(event: &RuntimeEvent) -> &'static str {
    match event {
        RuntimeEvent::MessageDelivered { .. } => "message-delivered",
        RuntimeEvent::ObserveRegistered { .. } => "observe-registered",
        RuntimeEvent::AssertionObserved { .. } => "assertion-observed",
        RuntimeEvent::AssertionCommitted { .. } => "assertion-committed",
        RuntimeEvent::AssertionRetracted { .. } => "assertion-retracted",
        RuntimeEvent::AssertionRetractionObserved { .. } => "assertion-retraction-observed",
        RuntimeEvent::EffectRequest { .. } => "effect-request",
        RuntimeEvent::EffectResponse { .. } => "effect-response",
        RuntimeEvent::AdmissionDecision { .. } => "admission-decision",
        RuntimeEvent::TurnRolledBack { .. } => "turn-rolled-back",
    }
}

fn ready_status_map(statuses: &[ServiceStatus]) -> Result<BTreeMap<String, String>> {
    let mut ready = BTreeMap::new();
    for status in statuses {
        if status.state == "ready" && ready.insert(status.service_id.clone(), status.status_ref.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate ready service status for {}",
                status.service_id
            )));
        }
    }
    Ok(ready)
}

fn dependency_status_refs(manifest: &ServiceManifest, ready_statuses: &BTreeMap<String, String>) -> Vec<String> {
    manifest
        .dependencies
        .iter()
        .filter_map(|service_id| ready_statuses.get(service_id).cloned())
        .collect()
}

fn manifest_ref_mismatch(demand: &ServiceDemand, manifest: &ServiceManifest) -> bool {
    demand.manifest_ref.as_ref().is_some_and(|manifest_ref| manifest_ref != &manifest.manifest_ref)
}

fn dependency_cycle_exists(manifests: &BTreeMap<String, ServiceManifest>) -> Result<bool> {
    for service_id in manifests.keys() {
        let mut stack = manifests.get(service_id).map(|manifest| manifest.dependencies.clone()).unwrap_or_default();
        let mut seen = BTreeSet::new();
        while let Some(next) = stack.pop() {
            if &next == service_id {
                return Ok(true);
            }
            if !seen.insert(next.clone()) {
                continue;
            }
            if seen.len() > MAX_SERVICE_RUNTIME_ITEMS {
                return Err(MoltenError::invalid_harness("service dependency graph exceeds bound"));
            }
            if let Some(manifest) = manifests.get(&next) {
                for dependency in &manifest.dependencies {
                    stack.push(dependency.clone());
                }
            }
        }
    }
    Ok(false)
}

fn validate_suite_input(input: &ServiceRuntimeSuiteInput) -> Result<()> {
    ensure_count_at_most(input.manifests.len(), "service manifests")?;
    ensure_count_at_most(input.demands.len(), "service demands")?;
    ensure_count_at_most(input.statuses.len(), "service statuses")?;
    for manifest in &input.manifests {
        service_records::parse_service_manifest(manifest)?;
    }
    for demand in &input.demands {
        service_records::parse_service_demand(demand)?;
    }
    for status in &input.statuses {
        service_records::parse_service_status(status)?;
    }
    validate_runtime_evidence(&input.evidence)
}

fn validate_runtime_evidence(evidence: &ServiceRuntimeEvidenceInput) -> Result<()> {
    validate_refs(&evidence.authority_refs, "service runtime authority ref")?;
    validate_refs(&evidence.policy_refs, "service runtime policy ref")?;
    validate_refs(&evidence.resource_refs, "service runtime resource ref")?;
    validate_refs(&evidence.effect_profile_refs, "service runtime effect profile ref")?;
    validate_refs(&evidence.source_gate_refs, "service runtime source gate ref")?;
    validate_optional_ref(evidence.scheduler_ref.as_deref(), "service runtime scheduler ref")?;
    validate_refs(&evidence.effect_log_refs, "service runtime effect log ref")
}

fn startup_admission_diagnostics(evidence: &ServiceRuntimeEvidenceInput) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if evidence.authority_refs.is_empty() {
        diagnostics.push("missing startup authority evidence".to_string());
    }
    if evidence.policy_refs.is_empty() {
        diagnostics.push("missing startup policy evidence".to_string());
    }
    if evidence.resource_refs.is_empty() {
        diagnostics.push("missing startup resource evidence".to_string());
    }
    if evidence.effect_profile_refs.is_empty() {
        diagnostics.push("missing startup effect-handle evidence".to_string());
    }
    if evidence.source_gate_refs.is_empty() {
        diagnostics.push("missing strict source-gate evidence".to_string());
    }
    diagnostics
}

fn evidence_value(evidence: &ServiceRuntimeEvidenceInput) -> IOValue {
    record("evidence", vec![
        record("authority", vec![refs_sequence(&evidence.authority_refs)]),
        record("policy", vec![refs_sequence(&evidence.policy_refs)]),
        record("resource", vec![refs_sequence(&evidence.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&evidence.effect_profile_refs)]),
        record("source-gate", vec![refs_sequence(&evidence.source_gate_refs)]),
        record("scheduler", vec![optional_ref_value(evidence.scheduler_ref.as_deref())]),
        record("effect-log", vec![refs_sequence(&evidence.effect_log_refs)]),
    ])
}

fn parse_evidence(value: &Value<IOValue>) -> Result<ServiceRuntimeEvidenceInput> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("evidence", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <evidence ...>"))?;
    Ok(ServiceRuntimeEvidenceInput {
        authority_refs: parse_ref_sequence(&fields[0], "authority")?,
        policy_refs: parse_ref_sequence(&fields[1], "policy")?,
        resource_refs: parse_ref_sequence(&fields[2], "resource")?,
        effect_profile_refs: parse_ref_sequence(&fields[3], "effect-profile")?,
        source_gate_refs: parse_ref_sequence(&fields[4], "source-gate")?,
        scheduler_ref: record_optional_ref(&fields[5], "scheduler")?,
        effect_log_refs: parse_ref_sequence(&fields[6], "effect-log")?,
    })
}

fn parse_iovalue_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<IOValue>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    Ok(values.iter().map(value_to_iovalue).collect())
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    values.iter().map(|value| required_ref(value, label)).collect()
}

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn record_iovalue(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_ref_value(&fields[0])
}

fn parse_optional_ref_value(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional service runtime ref").map(Some);
    }
    required_ref(value, "optional service runtime ref").map(Some)
}

fn checks_value(names: &[&str]) -> IOValue {
    record("checks", vec![sequence(
        names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect(),
    )])
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    ensure_count_at_most(values.len(), "service runtime checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected service runtime check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing passing {name} check")))
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")))
    }
}

fn refs_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), field)?;
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: Option<&str>, field: &str) -> Result<()> {
    if let Some(reference) = reference {
        require_ref(reference, field)
    } else {
        Ok(())
    }
}

fn required_ref(value: &Value<IOValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    if reference.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected blake3 ref for {field}, got {reference}")))
    }
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn ensure_count_at_most(actual: usize, label: &str) -> Result<()> {
    if actual <= MAX_SERVICE_RUNTIME_ITEMS {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{label} count {actual} exceeds bound {MAX_SERVICE_RUNTIME_ITEMS}"
        )))
    }
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("service-runtime-fixture-ref", vec![string(label)]))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::preserves_rail::parse_text;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("service-runtime-test-ref", vec![string(label)])).expect("test ref")
    }

    fn evidence() -> ServiceRuntimeEvidenceInput {
        ServiceRuntimeEvidenceInput {
            authority_refs: vec![test_ref("authority")],
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            effect_profile_refs: vec![test_ref("effect")],
            source_gate_refs: vec![test_ref("source-gate")],
            scheduler_ref: Some(test_ref("scheduler")),
            effect_log_refs: vec![test_ref("effect-log")],
        }
    }

    fn manifest(service_id: &str, dependencies: Vec<String>) -> IOValue {
        let evidence = evidence();
        service_records::service_manifest_value(&service_records::ServiceManifestInput {
            service_id: service_id.to_string(),
            owner_authority_ref: evidence.authority_refs[0].clone(),
            target_ref: test_ref(&format!("target-{service_id}")),
            dependencies,
            provided_assertion_refs: vec![test_ref(&format!("provided-{service_id}"))],
            restart_policy_ref: test_ref(&format!("restart-{service_id}")),
            policy_refs: evidence.policy_refs,
            resource_refs: evidence.resource_refs,
            effect_profile_refs: evidence.effect_profile_refs,
        })
        .expect("service manifest")
    }

    fn demand(service_id: &str, manifest_value: &IOValue) -> IOValue {
        service_records::service_demand_value(&service_records::ServiceDemandInput {
            demand_id: format!("demand:{service_id}"),
            service_id: service_id.to_string(),
            requester_ref: test_ref("requester"),
            manifest_ref: Some(canonical_hash(manifest_value).expect("manifest ref")),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("service demand")
    }

    #[test]
    fn two_service_demand_starts_dependency_then_frontend() {
        let suite_value = two_service_suite_value().expect("two service suite");
        let run = run_service_runtime_suite_value(&suite_value).expect("run services");
        assert_eq!(run.readiness_assertions.len(), 2);
        let receipts = run
            .lifecycle_receipts
            .iter()
            .map(service_records::parse_service_lifecycle_receipt)
            .collect::<Result<Vec<_>>>()
            .expect("parse receipts");
        assert_eq!(receipts.iter().filter(|receipt| receipt.decision == "pass").count(), 2);
        replay_service_runtime_report(&run.value).expect("replay service runtime report");
    }

    #[test]
    fn missing_authority_denies_before_side_effects() {
        let run = run_with_missing_evidence(|evidence| evidence.authority_refs.clear());
        assert!(run.readiness_assertions.is_empty());
        let receipt =
            service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("deny receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority")));
    }

    #[test]
    fn missing_source_gate_denies_before_side_effects() {
        let run = run_with_missing_evidence(|evidence| evidence.source_gate_refs.clear());
        assert!(run.readiness_assertions.is_empty());
        let receipt =
            service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("deny receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("source-gate")));
    }

    fn run_with_missing_evidence(update: impl FnOnce(&mut ServiceRuntimeEvidenceInput)) -> ServiceRuntimeRun {
        let backend = manifest("svc:backend", Vec::new());
        let demand = demand("svc:backend", &backend);
        let mut evidence = evidence();
        update(&mut evidence);
        let suite_value = record("service-runtime-suite-v1", vec![
            string(SERVICE_RUNTIME_SUITE_SCHEMA),
            record("manifests", vec![sequence(vec![backend])]),
            record("demands", vec![sequence(vec![demand])]),
            record("statuses", vec![sequence(Vec::new())]),
            evidence_value(&evidence),
            checks_value(&["canonical-service-runtime-suite", "explicit-startup-evidence"]),
        ]);
        run_service_runtime_suite_value(&suite_value).expect("missing evidence emits deny report")
    }

    #[test]
    fn unmet_dependency_waits_without_readiness() {
        let frontend = manifest("svc:frontend", vec!["svc:backend".to_string()]);
        let demand = demand("svc:frontend", &frontend);
        let suite = service_runtime_suite_value(&ServiceRuntimeSuiteInput {
            manifests: vec![frontend],
            demands: vec![demand],
            statuses: Vec::new(),
            evidence: evidence(),
        })
        .expect("suite");
        let run = run_service_runtime_suite_value(&suite).expect("run services");
        assert!(run.readiness_assertions.is_empty());
        let receipt = service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("receipt");
        assert_eq!(receipt.operation, "dependency-wait");
        assert_eq!(receipt.decision, "diagnostic");
    }

    #[test]
    fn dependency_cycle_denies_without_readiness() {
        let frontend = manifest("svc:frontend", vec!["svc:backend".to_string()]);
        let backend = manifest("svc:backend", vec!["svc:frontend".to_string()]);
        let frontend_demand = demand("svc:frontend", &frontend);
        let backend_demand = demand("svc:backend", &backend);
        let suite = service_runtime_suite_value(&ServiceRuntimeSuiteInput {
            manifests: vec![frontend, backend],
            demands: vec![frontend_demand, backend_demand],
            statuses: Vec::new(),
            evidence: evidence(),
        })
        .expect("suite");
        let run = run_service_runtime_suite_value(&suite).expect("run services");
        assert!(run.readiness_assertions.is_empty());
        let receipts = run
            .lifecycle_receipts
            .iter()
            .map(service_records::parse_service_lifecycle_receipt)
            .collect::<Result<Vec<_>>>()
            .expect("receipts");
        assert_eq!(receipts.len(), 2);
        assert!(receipts.iter().all(|receipt| receipt.decision == "deny"));
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("cycle")))
        );
    }

    #[test]
    fn malformed_manifest_denies_before_execution() {
        let malformed = parse_text("<service-manifest-v1 \"molten.service.manifest.v1\" <service-id \"svc:x\">>")
            .expect("malformed manifest shape parses");
        let suite = ServiceRuntimeSuiteInput {
            manifests: vec![malformed],
            demands: Vec::new(),
            statuses: Vec::new(),
            evidence: evidence(),
        };
        assert!(service_runtime_suite_value(&suite).is_err());
    }

    #[test]
    fn replay_detects_changed_dependency_identity() {
        let suite_value = two_service_suite_value().expect("two service suite");
        let run = run_service_runtime_suite_value(&suite_value).expect("run services");
        let mut report = parse_service_runtime_report(&run.value).expect("parse report");
        report.statuses.pop();
        let tampered = service_runtime_report_value(ReportValueInput {
            suite_value: &suite_value,
            lifecycle_receipts: &report.lifecycle_receipts,
            statuses: &report.statuses,
            readiness_assertions: &report.readiness_assertions,
            replay_identities: &report.replay_identities,
            turn_contexts: &report.turn_contexts,
        })
        .expect("tampered report");
        assert!(replay_service_runtime_report(&tampered).is_err());
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_demand_identity_replay_and_no_side_effects_on_wait(tc: TestCase) {
        let dependency_count = tc.draw(generators::integers::<u64>().min_value(0).max_value(3));
        let dependency_count_usize = usize::try_from(dependency_count).expect("bounded dependency count");
        let dependencies = (0..dependency_count_usize).map(|index| format!("svc:dep-{index}")).collect::<Vec<_>>();
        let service = manifest("svc:generated", dependencies);
        let demand = demand("svc:generated", &service);
        let suite = service_runtime_suite_value(&ServiceRuntimeSuiteInput {
            manifests: vec![service],
            demands: vec![demand],
            statuses: Vec::new(),
            evidence: evidence(),
        })
        .expect("generated suite");
        let run = run_service_runtime_suite_value(&suite).expect("generated run");
        let replay = replay_service_runtime_report(&run.value).expect("generated replay");
        assert_eq!(replay.decision, "pass");
        if dependency_count_usize == 0 {
            assert_eq!(run.readiness_assertions.len(), 1);
        } else {
            assert!(run.readiness_assertions.is_empty());
            let receipt =
                service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("receipt");
            assert_eq!(receipt.operation, "dependency-wait");
        }
    }
}
