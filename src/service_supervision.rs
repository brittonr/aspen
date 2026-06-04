use std::collections::BTreeSet;

use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::SERVICE_FAILURE_MARKER_SCHEMA;
use crate::preserves_rail::SERVICE_MONITOR_NOTIFICATION_SCHEMA;
use crate::preserves_rail::SERVICE_OWNED_STATE_SCHEMA;
use crate::preserves_rail::SERVICE_RETENTION_INPUT_SCHEMA;
use crate::preserves_rail::SERVICE_RETRACTION_SCHEMA;
use crate::preserves_rail::SERVICE_SUPERVISION_REPORT_SCHEMA;
use crate::preserves_rail::SERVICE_SUPERVISION_SUITE_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;
use crate::service_records;
use crate::service_records::ServiceCleanupReceiptInput;
use crate::service_records::ServiceDemandInput;
use crate::service_records::ServiceLifecycleReceiptInput;
use crate::service_records::ServiceLink;
use crate::service_records::ServiceManifest;
use crate::service_records::ServiceMonitor;
use crate::service_records::ServiceRestartDecisionInput;
use crate::service_records::ServiceRestartPolicy;
use crate::service_records::ServiceStatusInput;

const MAX_SUPERVISION_ITEMS: usize = 4096;

const _: () = assert!(MAX_SUPERVISION_ITEMS <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionEvidenceInput {
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub retention_policy_refs: Vec<String>,
    pub prior_lifecycle_refs: Vec<String>,
    pub effect_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOwnedStateInput {
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub foreign_ref_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOwnedState {
    pub state_ref: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub foreign_ref_claims: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionSuiteInput {
    pub manifest: IOValue,
    pub links: Vec<IOValue>,
    pub monitors: Vec<IOValue>,
    pub restart_policy: IOValue,
    pub owned_state: IOValue,
    pub restart_attempt: u64,
    pub logical_step: u64,
    pub evidence: ServiceSupervisionEvidenceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionSuite {
    pub suite_ref: String,
    pub manifest: ServiceManifest,
    pub links: Vec<ServiceLink>,
    pub monitors: Vec<ServiceMonitor>,
    pub restart_policy: ServiceRestartPolicy,
    pub owned_state: ServiceOwnedState,
    pub restart_attempt: u64,
    pub logical_step: u64,
    pub evidence: ServiceSupervisionEvidenceInput,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionRun {
    pub suite_ref: String,
    pub suite_value: IOValue,
    pub report_ref: String,
    pub failure_markers: Vec<IOValue>,
    pub statuses: Vec<IOValue>,
    pub lifecycle_receipts: Vec<IOValue>,
    pub monitor_notifications: Vec<IOValue>,
    pub restart_decisions: Vec<IOValue>,
    pub scheduled_demands: Vec<IOValue>,
    pub cleanup_receipts: Vec<IOValue>,
    pub retractions: Vec<IOValue>,
    pub retention_inputs: Vec<IOValue>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionReplay {
    pub expected_report_ref: String,
    pub actual_report_ref: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestartEvaluation {
    decision: String,
    attempt: u64,
    backoff_slot: u64,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupEvaluation {
    cleanup_receipt: Option<IOValue>,
    retractions: Vec<IOValue>,
    retention_input: Option<IOValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CleanupTarget {
    kind: String,
    target_ref: String,
}

pub fn service_owned_state_value(input: &ServiceOwnedStateInput) -> Result<IOValue> {
    validate_owned_state_input(input)?;
    Ok(record("service-owned-state-v1", vec![
        string(SERVICE_OWNED_STATE_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("owned-assertions", vec![refs_sequence(&input.owned_assertion_refs)]),
        record("observers", vec![refs_sequence(&input.observer_refs)]),
        record("live-refs", vec![refs_sequence(&input.live_ref_refs)]),
        record("exposed-refs", vec![refs_sequence(&input.exposed_ref_refs)]),
        record("pending-effects", vec![refs_sequence(&input.pending_effect_refs)]),
        record("foreign-claims", vec![refs_sequence(&input.foreign_ref_claims)]),
        checks_value(&["service-owned-state", "cleanup-index", "foreign-claims-explicit"]),
    ]))
}

pub fn parse_service_owned_state(value: &IOValue) -> Result<ServiceOwnedState> {
    let fields = value
        .collect_simple_record("service-owned-state-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-owned-state-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_OWNED_STATE_SCHEMA, "service owned-state schema")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "cleanup-index", "service owned state")?;
    let owned_state = ServiceOwnedState {
        state_ref: canonical_hash(value)?,
        service_id: record_string(&fields[1], "service-id")?,
        manifest_ref: record_optional_ref(&fields[2], "manifest")?,
        owned_assertion_refs: parse_ref_sequence(&fields[3], "owned-assertions")?,
        observer_refs: parse_ref_sequence(&fields[4], "observers")?,
        live_ref_refs: parse_ref_sequence(&fields[5], "live-refs")?,
        exposed_ref_refs: parse_ref_sequence(&fields[6], "exposed-refs")?,
        pending_effect_refs: parse_ref_sequence(&fields[7], "pending-effects")?,
        foreign_ref_claims: parse_ref_sequence(&fields[8], "foreign-claims")?,
        value: value.clone(),
    };
    validate_owned_state_parsed(&owned_state)?;
    Ok(owned_state)
}

pub fn service_supervision_suite_value(input: &ServiceSupervisionSuiteInput) -> Result<IOValue> {
    validate_suite_input(input)?;
    Ok(record("service-supervision-suite-v1", vec![
        string(SERVICE_SUPERVISION_SUITE_SCHEMA),
        record("manifest", vec![input.manifest.clone()]),
        record("links", vec![sequence(input.links.clone())]),
        record("monitors", vec![sequence(input.monitors.clone())]),
        record("restart-policy", vec![input.restart_policy.clone()]),
        record("owned-state", vec![input.owned_state.clone()]),
        record("restart-attempt", vec![u64_value(input.restart_attempt)]),
        record("logical-step", vec![u64_value(input.logical_step)]),
        evidence_value(&input.evidence),
        checks_value(&[
            "canonical-service-supervision-suite",
            "logical-supervision-only",
            "bounded-restart-cleanup",
        ]),
    ]))
}

pub fn parse_service_supervision_suite(value: &IOValue) -> Result<ServiceSupervisionSuite> {
    let fields = value
        .collect_simple_record("service-supervision-suite-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervision-suite-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_SUPERVISION_SUITE_SCHEMA, "service supervision suite schema")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "logical-supervision-only", "service supervision suite")?;
    let manifest_value = record_iovalue(&fields[1], "manifest")?;
    let restart_policy_value = record_iovalue(&fields[4], "restart-policy")?;
    let owned_state_value = record_iovalue(&fields[5], "owned-state")?;
    let suite = ServiceSupervisionSuite {
        suite_ref: canonical_hash(value)?,
        manifest: service_records::parse_service_manifest(&manifest_value)?,
        links: parse_link_sequence(&fields[2])?,
        monitors: parse_monitor_sequence(&fields[3])?,
        restart_policy: service_records::parse_service_restart_policy(&restart_policy_value)?,
        owned_state: parse_service_owned_state(&owned_state_value)?,
        restart_attempt: record_u64(&fields[6], "restart-attempt")?,
        logical_step: record_u64(&fields[7], "logical-step")?,
        evidence: parse_evidence(&fields[8])?,
        value: value.clone(),
    };
    validate_suite_parsed(&suite)?;
    Ok(suite)
}

pub fn run_service_supervision_suite_value(value: &IOValue) -> Result<ServiceSupervisionRun> {
    let suite = parse_service_supervision_suite(value)?;
    run_service_supervision_suite(&suite)
}

pub fn run_service_supervision_suite(suite: &ServiceSupervisionSuite) -> Result<ServiceSupervisionRun> {
    let mut monitors = suite
        .monitors
        .iter()
        .filter(|monitor| monitor.service_id == suite.manifest.service_id)
        .cloned()
        .collect::<Vec<_>>();
    ensure_count_at_most(monitors.len(), "service monitors")?;
    monitors.sort_by(|left, right| {
        left.service_id
            .cmp(&right.service_id)
            .then_with(|| left.monitor_ref.cmp(&right.monitor_ref))
            .then_with(|| left.observer_ref.cmp(&right.observer_ref))
    });

    let failure_marker = failure_marker_value(suite)?;
    let failure_ref = canonical_hash(&failure_marker)?;
    let monitor_refs = monitors.iter().map(|monitor| monitor.monitor_ref.clone()).collect::<Vec<_>>();
    let failure_status = failure_status_value(suite, &failure_ref, &monitor_refs)?;
    let failure_status_ref = canonical_hash(&failure_status)?;
    let monitor_notifications = monitor_notification_values(&monitors, suite, &failure_ref, &failure_status_ref)?;
    let notification_refs = refs_for_values(&monitor_notifications)?;
    let supervision_refs = supervision_refs(suite, &monitor_refs, &notification_refs)?;
    let failure_lifecycle = failure_lifecycle_receipt(suite, &failure_status_ref, &supervision_refs)?;
    let failure_lifecycle_ref = canonical_hash(&failure_lifecycle)?;
    let restart_evaluation = evaluate_restart(suite)?;
    let restart_decision = restart_decision_value(suite, &restart_evaluation, &failure_lifecycle_ref)?;
    let restart_decision_ref = canonical_hash(&restart_decision)?;
    let scheduled_demands = scheduled_demands(suite, &restart_evaluation)?;
    let cleanup_evaluation = evaluate_cleanup(suite, &restart_evaluation, &restart_decision_ref)?;
    let final_statuses =
        final_status_values(suite, &failure_ref, &restart_evaluation, &restart_decision_ref, &monitor_refs)?;
    let failure_markers = vec![failure_marker];
    let statuses = status_values(&failure_status, &final_statuses)?;
    let lifecycle_receipts = vec![failure_lifecycle];
    let restart_decisions = vec![restart_decision];
    let cleanup_receipts = optional_value_vec(cleanup_evaluation.cleanup_receipt);
    let retractions = cleanup_evaluation.retractions;
    let retention_inputs = optional_value_vec(cleanup_evaluation.retention_input);
    let report_value = service_supervision_report_value(ReportValueInput {
        suite_value: &suite.value,
        failure_markers: &failure_markers,
        statuses: &statuses,
        lifecycle_receipts: &lifecycle_receipts,
        monitor_notifications: &monitor_notifications,
        restart_decisions: &restart_decisions,
        scheduled_demands: &scheduled_demands,
        cleanup_receipts: &cleanup_receipts,
        retractions: &retractions,
        retention_inputs: &retention_inputs,
    })?;
    Ok(ServiceSupervisionRun {
        suite_ref: suite.suite_ref.clone(),
        suite_value: suite.value.clone(),
        report_ref: canonical_hash(&report_value)?,
        failure_markers,
        statuses,
        lifecycle_receipts,
        monitor_notifications,
        restart_decisions,
        scheduled_demands,
        cleanup_receipts,
        retractions,
        retention_inputs,
        value: report_value,
    })
}

pub fn replay_service_supervision_report(value: &IOValue) -> Result<ServiceSupervisionReplay> {
    let report = parse_service_supervision_report(value)?;
    let rerun = run_service_supervision_suite_value(&report.suite_value)?;
    let expected_report_ref = canonical_hash(value)?;
    let decision = if expected_report_ref == rerun.report_ref {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    if decision == "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "service supervision replay divergence: expected {expected_report_ref}, got {}",
            rerun.report_ref
        )));
    }
    Ok(ServiceSupervisionReplay {
        expected_report_ref,
        actual_report_ref: rerun.report_ref,
        decision,
    })
}

pub fn parse_service_supervision_report(value: &IOValue) -> Result<ServiceSupervisionRun> {
    let fields = value
        .collect_simple_record("service-supervision-report-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervision-report-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_SUPERVISION_REPORT_SCHEMA, "service supervision report schema")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-service-supervision-report", "service supervision report")?;
    let suite_value = record_iovalue(&fields[1], "suite")?;
    let suite_ref = canonical_hash(&suite_value)?;
    Ok(ServiceSupervisionRun {
        suite_ref,
        suite_value,
        report_ref: canonical_hash(value)?,
        failure_markers: parse_iovalue_sequence(&fields[2], "failures")?,
        statuses: parse_iovalue_sequence(&fields[3], "statuses")?,
        lifecycle_receipts: parse_iovalue_sequence(&fields[4], "lifecycle")?,
        monitor_notifications: parse_iovalue_sequence(&fields[5], "monitor-notifications")?,
        restart_decisions: parse_iovalue_sequence(&fields[6], "restart-decisions")?,
        scheduled_demands: parse_iovalue_sequence(&fields[7], "scheduled-demands")?,
        cleanup_receipts: parse_iovalue_sequence(&fields[8], "cleanup")?,
        retractions: parse_iovalue_sequence(&fields[9], "retractions")?,
        retention_inputs: parse_iovalue_sequence(&fields[10], "retention")?,
        value: value.clone(),
    })
}

pub fn service_supervision_summary(value: &IOValue) -> Result<String> {
    let report = parse_service_supervision_report(value)?;
    let restart_decision = report
        .restart_decisions
        .first()
        .map(service_records::parse_service_restart_decision)
        .transpose()?
        .map_or_else(|| "none".to_string(), |decision| decision.decision);
    Ok(format!(
        "service supervision report ref={} suite={} monitors={} restart={} cleanup={} retractions={}",
        report.report_ref,
        report.suite_ref,
        report.monitor_notifications.len(),
        restart_decision,
        report.cleanup_receipts.len(),
        report.retractions.len()
    ))
}

pub fn service_supervision_report_value(input: ReportValueInput<'_>) -> Result<IOValue> {
    validate_report_input(&input)?;
    Ok(record("service-supervision-report-v1", vec![
        string(SERVICE_SUPERVISION_REPORT_SCHEMA),
        record("suite", vec![input.suite_value.clone()]),
        record("failures", vec![sequence(input.failure_markers.to_vec())]),
        record("statuses", vec![sequence(input.statuses.to_vec())]),
        record("lifecycle", vec![sequence(input.lifecycle_receipts.to_vec())]),
        record("monitor-notifications", vec![sequence(input.monitor_notifications.to_vec())]),
        record("restart-decisions", vec![sequence(input.restart_decisions.to_vec())]),
        record("scheduled-demands", vec![sequence(input.scheduled_demands.to_vec())]),
        record("cleanup", vec![sequence(input.cleanup_receipts.to_vec())]),
        record("retractions", vec![sequence(input.retractions.to_vec())]),
        record("retention", vec![sequence(input.retention_inputs.to_vec())]),
        checks_value(&[
            "canonical-service-supervision-report",
            "monitor-order-bound",
            "cleanup-retention-bound",
        ]),
    ]))
}

#[derive(Debug, Clone, Copy)]
pub struct ReportValueInput<'a> {
    pub suite_value: &'a IOValue,
    pub failure_markers: &'a [IOValue],
    pub statuses: &'a [IOValue],
    pub lifecycle_receipts: &'a [IOValue],
    pub monitor_notifications: &'a [IOValue],
    pub restart_decisions: &'a [IOValue],
    pub scheduled_demands: &'a [IOValue],
    pub cleanup_receipts: &'a [IOValue],
    pub retractions: &'a [IOValue],
    pub retention_inputs: &'a [IOValue],
}

pub fn supervision_fixture_suite_value() -> Result<IOValue> {
    let authority_ref = synthetic_ref("authority")?;
    let resource_ref = synthetic_ref("resource")?;
    let retention_ref = synthetic_ref("retention")?;
    let effect_ref = synthetic_ref("effect-log")?;
    let policy_ref = synthetic_ref("policy")?;
    let restart_policy = service_records::service_restart_policy_value(&service_records::ServiceRestartPolicyInput {
        policy_id: "restart:web".to_string(),
        max_attempts: 2,
        window_steps: 8,
        backoff_steps: 0,
        resource_refs: vec![resource_ref.clone()],
    })?;
    let restart_policy_ref = canonical_hash(&restart_policy)?;
    let manifest = service_records::service_manifest_value(&service_records::ServiceManifestInput {
        service_id: "svc:web".to_string(),
        owner_authority_ref: authority_ref.clone(),
        target_ref: synthetic_ref("target")?,
        dependencies: Vec::new(),
        provided_assertion_refs: vec![synthetic_ref("ready-pattern")?],
        restart_policy_ref,
        policy_refs: vec![policy_ref.clone()],
        resource_refs: vec![resource_ref.clone()],
        effect_profile_refs: vec![effect_ref.clone()],
    })?;
    let manifest_ref = canonical_hash(&manifest)?;
    let first_monitor = service_records::service_monitor_value(&service_records::ServiceMonitorInput {
        monitor_id: "monitor:web:b".to_string(),
        service_id: "svc:web".to_string(),
        observer_ref: synthetic_ref("observer-b")?,
        notification_policy: "failure".to_string(),
        policy_refs: vec![policy_ref.clone()],
    })?;
    let second_monitor = service_records::service_monitor_value(&service_records::ServiceMonitorInput {
        monitor_id: "monitor:web:a".to_string(),
        service_id: "svc:web".to_string(),
        observer_ref: synthetic_ref("observer-a")?,
        notification_policy: "failure".to_string(),
        policy_refs: vec![policy_ref.clone()],
    })?;
    let link = service_records::service_link_value(&service_records::ServiceLinkInput {
        supervisor_id: "supervisor:web".to_string(),
        parent_service_id: "svc:web".to_string(),
        child_service_id: "svc:web".to_string(),
        propagation: "restart".to_string(),
        policy_refs: vec![policy_ref],
    })?;
    let owned_state = service_owned_state_value(&ServiceOwnedStateInput {
        service_id: "svc:web".to_string(),
        manifest_ref: Some(manifest_ref),
        owned_assertion_refs: vec![synthetic_ref("readiness")?],
        observer_refs: vec![synthetic_ref("observer-registration")?],
        live_ref_refs: vec![synthetic_ref("live-ref")?],
        exposed_ref_refs: vec![synthetic_ref("exposed-ref")?],
        pending_effect_refs: vec![synthetic_ref("pending-effect")?],
        foreign_ref_claims: Vec::new(),
    })?;
    service_supervision_suite_value(&ServiceSupervisionSuiteInput {
        manifest,
        links: vec![link],
        monitors: vec![first_monitor, second_monitor],
        restart_policy,
        owned_state,
        restart_attempt: 0,
        logical_step: 0,
        evidence: ServiceSupervisionEvidenceInput {
            authority_refs: vec![authority_ref],
            resource_refs: vec![resource_ref],
            revocation_refs: Vec::new(),
            retention_policy_refs: vec![retention_ref],
            prior_lifecycle_refs: vec![synthetic_ref("prior-lifecycle")?],
            effect_log_refs: vec![effect_ref],
        },
    })
}

fn failure_marker_value(suite: &ServiceSupervisionSuite) -> Result<IOValue> {
    Ok(record("service-failure-v1", vec![
        string(SERVICE_FAILURE_MARKER_SCHEMA),
        record("service-id", vec![string(&suite.manifest.service_id)]),
        record("manifest", vec![string(&suite.manifest.manifest_ref)]),
        record("prior-lifecycle", vec![refs_sequence(&suite.evidence.prior_lifecycle_refs)]),
        record("effect-log", vec![refs_sequence(&suite.evidence.effect_log_refs)]),
        checks_value(&["canonical-service-failure", "logical-supervision", "replay-bound"]),
    ]))
}

fn failure_status_value(
    suite: &ServiceSupervisionSuite,
    failure_ref: &str,
    monitor_refs: &[String],
) -> Result<IOValue> {
    service_records::service_status_value(&ServiceStatusInput {
        service_id: suite.manifest.service_id.clone(),
        state: "failed".to_string(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        demand_refs: Vec::new(),
        dependency_status_refs: Vec::new(),
        readiness_assertion_refs: Vec::new(),
        failure_refs: vec![failure_ref.to_string()],
        restart_count: suite.restart_attempt,
        monitor_refs: monitor_refs.to_vec(),
        replay_refs: suite.evidence.prior_lifecycle_refs.clone(),
    })
}

fn final_status_values(
    suite: &ServiceSupervisionSuite,
    failure_ref: &str,
    restart: &RestartEvaluation,
    restart_decision_ref: &str,
    monitor_refs: &[String],
) -> Result<Vec<IOValue>> {
    if restart.decision != "deny" {
        return Ok(Vec::new());
    }
    let state = if suite.evidence.revocation_refs.is_empty() {
        "failed"
    } else {
        "stopped"
    };
    let status = service_records::service_status_value(&ServiceStatusInput {
        service_id: suite.manifest.service_id.clone(),
        state: state.to_string(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        demand_refs: Vec::new(),
        dependency_status_refs: Vec::new(),
        readiness_assertion_refs: Vec::new(),
        failure_refs: vec![failure_ref.to_string(), restart_decision_ref.to_string()],
        restart_count: restart.attempt,
        monitor_refs: monitor_refs.to_vec(),
        replay_refs: suite.evidence.prior_lifecycle_refs.clone(),
    })?;
    Ok(vec![status])
}

fn monitor_notification_values(
    monitors: &[ServiceMonitor],
    suite: &ServiceSupervisionSuite,
    failure_ref: &str,
    failure_status_ref: &str,
) -> Result<Vec<IOValue>> {
    let mut notifications = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let notification = record("service-monitor-notification-v1", vec![
            string(SERVICE_MONITOR_NOTIFICATION_SCHEMA),
            record("service-id", vec![string(&suite.manifest.service_id)]),
            record("monitor", vec![string(&monitor.monitor_ref)]),
            record("observer", vec![string(&monitor.observer_ref)]),
            record("failure", vec![string(failure_ref)]),
            record("status", vec![string(failure_status_ref)]),
            checks_value(&["monitor-order-bound", "logical-notification", "failure-bound"]),
        ]);
        notifications.push(notification);
    }
    Ok(notifications)
}

fn failure_lifecycle_receipt(
    suite: &ServiceSupervisionSuite,
    failure_status_ref: &str,
    supervision_refs: &[String],
) -> Result<IOValue> {
    service_records::service_lifecycle_receipt_value(&ServiceLifecycleReceiptInput {
        operation: "fail".to_string(),
        decision: "pass".to_string(),
        service_id: suite.manifest.service_id.clone(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        status_ref: Some(failure_status_ref.to_string()),
        authority_refs: suite.evidence.authority_refs.clone(),
        resource_refs: suite.evidence.resource_refs.clone(),
        effect_profile_refs: suite.evidence.effect_log_refs.clone(),
        supervision_refs: supervision_refs.to_vec(),
        diagnostics: Vec::new(),
    })
}

fn evaluate_restart(suite: &ServiceSupervisionSuite) -> Result<RestartEvaluation> {
    let is_authority_present = !suite.evidence.authority_refs.is_empty();
    let is_resource_present = !suite.evidence.resource_refs.is_empty();
    let is_revoked = !suite.evidence.revocation_refs.is_empty();
    let backoff_slot = suite
        .restart_attempt
        .checked_mul(suite.restart_policy.backoff_steps)
        .ok_or_else(|| MoltenError::invalid_harness("service restart backoff overflow"))?;
    let attempt = if suite.restart_attempt >= suite.restart_policy.max_attempts {
        suite.restart_attempt
    } else {
        suite
            .restart_attempt
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("service restart attempt overflow"))?
    };
    if is_revoked {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "service owner authority revoked".to_string(),
        ]));
    }
    if !is_authority_present {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "missing restart authority evidence".to_string(),
        ]));
    }
    if !is_resource_present {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "missing restart resource evidence".to_string(),
        ]));
    }
    if suite.restart_attempt >= suite.restart_policy.max_attempts {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "restart attempt budget exhausted".to_string(),
        ]));
    }
    if suite.logical_step < backoff_slot {
        return Ok(restart_evaluation("backoff", attempt, backoff_slot, vec![
            "logical backoff slot has not elapsed".to_string(),
        ]));
    }
    Ok(restart_evaluation("pass", attempt, backoff_slot, Vec::new()))
}

fn restart_evaluation(decision: &str, attempt: u64, backoff_slot: u64, diagnostics: Vec<String>) -> RestartEvaluation {
    RestartEvaluation {
        decision: decision.to_string(),
        attempt,
        backoff_slot,
        diagnostics,
    }
}

fn restart_decision_value(
    suite: &ServiceSupervisionSuite,
    restart: &RestartEvaluation,
    failure_lifecycle_ref: &str,
) -> Result<IOValue> {
    let mut prior_lifecycle_refs = suite.evidence.prior_lifecycle_refs.clone();
    prior_lifecycle_refs.push(failure_lifecycle_ref.to_string());
    service_records::service_restart_decision_value(&ServiceRestartDecisionInput {
        decision: restart.decision.clone(),
        service_id: suite.manifest.service_id.clone(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        policy_ref: suite.restart_policy.policy_ref.clone(),
        attempt: restart.attempt,
        max_attempts: suite.restart_policy.max_attempts,
        window_step: suite.logical_step,
        backoff_slot: restart.backoff_slot,
        prior_lifecycle_refs,
        authority_refs: suite.evidence.authority_refs.clone(),
        resource_refs: suite.evidence.resource_refs.clone(),
        diagnostics: restart.diagnostics.clone(),
    })
}

fn scheduled_demands(suite: &ServiceSupervisionSuite, restart: &RestartEvaluation) -> Result<Vec<IOValue>> {
    if restart.decision != "pass" {
        return Ok(Vec::new());
    }
    let requester_ref = suite
        .evidence
        .authority_refs
        .first()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("restart pass requires authority ref"))?;
    let demand = service_records::service_demand_value(&ServiceDemandInput {
        demand_id: format!("restart:{}:{}", suite.manifest.service_id, restart.attempt),
        service_id: suite.manifest.service_id.clone(),
        requester_ref,
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        policy_refs: suite.manifest.policy_refs.clone(),
    })?;
    Ok(vec![demand])
}

fn evaluate_cleanup(
    suite: &ServiceSupervisionSuite,
    restart: &RestartEvaluation,
    restart_decision_ref: &str,
) -> Result<CleanupEvaluation> {
    let is_cleanup_required = restart.decision == "deny" || !suite.evidence.revocation_refs.is_empty();
    if !is_cleanup_required {
        return Ok(CleanupEvaluation {
            cleanup_receipt: None,
            retractions: Vec::new(),
            retention_input: None,
        });
    }
    let is_foreign_claim_present = !suite.owned_state.foreign_ref_claims.is_empty();
    if is_foreign_claim_present {
        let cleanup_receipt = service_records::service_cleanup_receipt_value(&ServiceCleanupReceiptInput {
            decision: "deny".to_string(),
            service_id: suite.manifest.service_id.clone(),
            manifest_ref: Some(suite.manifest.manifest_ref.clone()),
            authority_refs: suite.evidence.authority_refs.clone(),
            owned_assertion_refs: suite.owned_state.owned_assertion_refs.clone(),
            observer_refs: suite.owned_state.observer_refs.clone(),
            live_ref_refs: suite.owned_state.live_ref_refs.clone(),
            exposed_ref_refs: suite.owned_state.exposed_ref_refs.clone(),
            pending_effect_refs: suite.owned_state.pending_effect_refs.clone(),
            retraction_refs: Vec::new(),
            revocation_refs: suite.evidence.revocation_refs.clone(),
            retention_refs: suite.evidence.retention_policy_refs.clone(),
            diagnostics: vec!["foreign service-owned state cannot be proven".to_string()],
        })?;
        let retention_input = retention_input_value(suite, &canonical_hash(&cleanup_receipt)?, restart_decision_ref)?;
        return Ok(CleanupEvaluation {
            cleanup_receipt: Some(cleanup_receipt),
            retractions: Vec::new(),
            retention_input: Some(retention_input),
        });
    }
    let targets = cleanup_targets(&suite.owned_state)?;
    let mut retractions = Vec::with_capacity(targets.len());
    let mut retraction_refs = Vec::with_capacity(targets.len());
    for target in targets {
        let retraction = retraction_value(suite, &target)?;
        retraction_refs.push(canonical_hash(&retraction)?);
        retractions.push(retraction);
    }
    let cleanup_receipt = service_records::service_cleanup_receipt_value(&ServiceCleanupReceiptInput {
        decision: "pass".to_string(),
        service_id: suite.manifest.service_id.clone(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        authority_refs: suite.evidence.authority_refs.clone(),
        owned_assertion_refs: suite.owned_state.owned_assertion_refs.clone(),
        observer_refs: suite.owned_state.observer_refs.clone(),
        live_ref_refs: suite.owned_state.live_ref_refs.clone(),
        exposed_ref_refs: suite.owned_state.exposed_ref_refs.clone(),
        pending_effect_refs: suite.owned_state.pending_effect_refs.clone(),
        retraction_refs,
        revocation_refs: suite.evidence.revocation_refs.clone(),
        retention_refs: suite.evidence.retention_policy_refs.clone(),
        diagnostics: Vec::new(),
    })?;
    let retention_input = retention_input_value(suite, &canonical_hash(&cleanup_receipt)?, restart_decision_ref)?;
    Ok(CleanupEvaluation {
        cleanup_receipt: Some(cleanup_receipt),
        retractions,
        retention_input: Some(retention_input),
    })
}

fn cleanup_targets(owned_state: &ServiceOwnedState) -> Result<Vec<CleanupTarget>> {
    let total = owned_state
        .owned_assertion_refs
        .len()
        .checked_add(owned_state.observer_refs.len())
        .and_then(|total| total.checked_add(owned_state.live_ref_refs.len()))
        .and_then(|total| total.checked_add(owned_state.exposed_ref_refs.len()))
        .and_then(|total| total.checked_add(owned_state.pending_effect_refs.len()))
        .ok_or_else(|| MoltenError::invalid_harness("service cleanup target count overflow"))?;
    ensure_count_at_most(total, "service cleanup targets")?;
    let mut targets = BTreeSet::new();
    insert_targets(&mut targets, "owned-assertion", &owned_state.owned_assertion_refs);
    insert_targets(&mut targets, "observer", &owned_state.observer_refs);
    insert_targets(&mut targets, "live-ref", &owned_state.live_ref_refs);
    insert_targets(&mut targets, "exposed-ref", &owned_state.exposed_ref_refs);
    insert_targets(&mut targets, "pending-effect", &owned_state.pending_effect_refs);
    Ok(targets.into_iter().collect())
}

fn insert_targets(targets: &mut BTreeSet<CleanupTarget>, kind: &str, refs: &[String]) {
    for target_ref in refs {
        targets.insert(CleanupTarget {
            kind: kind.to_string(),
            target_ref: target_ref.clone(),
        });
    }
}

fn retraction_value(suite: &ServiceSupervisionSuite, target: &CleanupTarget) -> Result<IOValue> {
    Ok(record("service-retraction-v1", vec![
        string(SERVICE_RETRACTION_SCHEMA),
        record("service-id", vec![string(&suite.manifest.service_id)]),
        record("manifest", vec![string(&suite.manifest.manifest_ref)]),
        record("kind", vec![string(&target.kind)]),
        record("target", vec![string(&target.target_ref)]),
        record("authority", vec![refs_sequence(&suite.evidence.authority_refs)]),
        record("revocations", vec![refs_sequence(&suite.evidence.revocation_refs)]),
        checks_value(&["service-owned-retraction", "no-foreign-delete", "retention-still-gates"]),
    ]))
}

fn retention_input_value(
    suite: &ServiceSupervisionSuite,
    cleanup_receipt_ref: &str,
    restart_decision_ref: &str,
) -> Result<IOValue> {
    Ok(record("service-retention-input-v1", vec![
        string(SERVICE_RETENTION_INPUT_SCHEMA),
        record("service-id", vec![string(&suite.manifest.service_id)]),
        record("cleanup", vec![string(cleanup_receipt_ref)]),
        record("restart-decision", vec![string(restart_decision_ref)]),
        record("retention-policy", vec![refs_sequence(&suite.evidence.retention_policy_refs)]),
        checks_value(&[
            "cleanup-is-input-evidence",
            "retention-policy-still-decides",
            "no-physical-delete",
        ]),
    ]))
}

fn status_values(failure_status: &IOValue, final_statuses: &[IOValue]) -> Result<Vec<IOValue>> {
    let total = final_statuses
        .len()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("service status count overflow"))?;
    let mut statuses = Vec::with_capacity(total);
    statuses.push(failure_status.clone());
    statuses.extend_from_slice(final_statuses);
    Ok(statuses)
}

fn refs_for_values(values: &[IOValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(canonical_hash(value)?);
    }
    Ok(refs)
}

fn supervision_refs(
    suite: &ServiceSupervisionSuite,
    monitor_refs: &[String],
    notification_refs: &[String],
) -> Result<Vec<String>> {
    let total = suite
        .links
        .len()
        .checked_add(monitor_refs.len())
        .and_then(|total| total.checked_add(notification_refs.len()))
        .ok_or_else(|| MoltenError::invalid_harness("service supervision ref count overflow"))?;
    let mut refs = Vec::with_capacity(total);
    refs.extend(suite.links.iter().map(|link| link.link_ref.clone()));
    refs.extend_from_slice(monitor_refs);
    refs.extend_from_slice(notification_refs);
    Ok(refs)
}

fn optional_value_vec(value: Option<IOValue>) -> Vec<IOValue> {
    value.map_or_else(Vec::new, |value| vec![value])
}

fn validate_suite_input(input: &ServiceSupervisionSuiteInput) -> Result<()> {
    ensure_count_at_most(input.links.len(), "service supervision links")?;
    ensure_count_at_most(input.monitors.len(), "service supervision monitors")?;
    service_records::parse_service_manifest(&input.manifest)?;
    service_records::parse_service_restart_policy(&input.restart_policy)?;
    parse_service_owned_state(&input.owned_state)?;
    for link in &input.links {
        service_records::parse_service_link(link)?;
    }
    for monitor in &input.monitors {
        service_records::parse_service_monitor(monitor)?;
    }
    validate_evidence(&input.evidence)
}

fn validate_suite_parsed(suite: &ServiceSupervisionSuite) -> Result<()> {
    if suite.owned_state.service_id != suite.manifest.service_id {
        return Err(MoltenError::invalid_harness("owned state service id must match manifest service id"));
    }
    if suite.owned_state.manifest_ref.as_deref() != Some(suite.manifest.manifest_ref.as_str()) {
        return Err(MoltenError::invalid_harness("owned state manifest ref must match manifest"));
    }
    validate_evidence(&suite.evidence)
}

fn validate_owned_state_input(input: &ServiceOwnedStateInput) -> Result<()> {
    validate_service_id(&input.service_id, "service owned-state service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service owned-state manifest ref")?;
    validate_refs(&input.owned_assertion_refs, "service owned assertion ref")?;
    validate_refs(&input.observer_refs, "service observer ref")?;
    validate_refs(&input.live_ref_refs, "service live ref")?;
    validate_refs(&input.exposed_ref_refs, "service exposed ref")?;
    validate_refs(&input.pending_effect_refs, "service pending effect ref")?;
    validate_refs(&input.foreign_ref_claims, "service foreign claim ref")
}

fn validate_owned_state_parsed(owned_state: &ServiceOwnedState) -> Result<()> {
    validate_service_id(&owned_state.service_id, "service owned-state service id")?;
    validate_optional_ref(owned_state.manifest_ref.as_deref(), "service owned-state manifest ref")?;
    validate_refs(&owned_state.foreign_ref_claims, "service foreign claim ref")
}

fn validate_evidence(evidence: &ServiceSupervisionEvidenceInput) -> Result<()> {
    validate_refs(&evidence.authority_refs, "service supervision authority ref")?;
    validate_refs(&evidence.resource_refs, "service supervision resource ref")?;
    validate_refs(&evidence.revocation_refs, "service supervision revocation ref")?;
    validate_refs(&evidence.retention_policy_refs, "service supervision retention ref")?;
    validate_refs(&evidence.prior_lifecycle_refs, "service supervision lifecycle ref")?;
    validate_refs(&evidence.effect_log_refs, "service supervision effect log ref")
}

fn validate_report_input(input: &ReportValueInput<'_>) -> Result<()> {
    ensure_count_at_most(input.failure_markers.len(), "service supervision failures")?;
    ensure_count_at_most(input.statuses.len(), "service supervision statuses")?;
    ensure_count_at_most(input.lifecycle_receipts.len(), "service supervision lifecycle receipts")?;
    ensure_count_at_most(input.monitor_notifications.len(), "service monitor notifications")?;
    ensure_count_at_most(input.restart_decisions.len(), "service restart decisions")?;
    ensure_count_at_most(input.scheduled_demands.len(), "service scheduled demands")?;
    ensure_count_at_most(input.cleanup_receipts.len(), "service cleanup receipts")?;
    ensure_count_at_most(input.retractions.len(), "service retractions")?;
    ensure_count_at_most(input.retention_inputs.len(), "service retention inputs")
}

fn ensure_count_at_most(actual: usize, label: &str) -> Result<()> {
    if actual <= MAX_SUPERVISION_ITEMS {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{label} count {actual} exceeds bound {MAX_SUPERVISION_ITEMS}"
        )))
    }
}

fn evidence_value(input: &ServiceSupervisionEvidenceInput) -> IOValue {
    record("evidence", vec![
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("revocations", vec![refs_sequence(&input.revocation_refs)]),
        record("retention", vec![refs_sequence(&input.retention_policy_refs)]),
        record("prior-lifecycle", vec![refs_sequence(&input.prior_lifecycle_refs)]),
        record("effect-log", vec![refs_sequence(&input.effect_log_refs)]),
    ])
}

fn parse_evidence(value: &Value<IOValue>) -> Result<ServiceSupervisionEvidenceInput> {
    let fields = value
        .collect_simple_record("evidence", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected service supervision evidence"))?;
    Ok(ServiceSupervisionEvidenceInput {
        authority_refs: parse_ref_sequence(&fields[0], "authority")?,
        resource_refs: parse_ref_sequence(&fields[1], "resource")?,
        revocation_refs: parse_ref_sequence(&fields[2], "revocations")?,
        retention_policy_refs: parse_ref_sequence(&fields[3], "retention")?,
        prior_lifecycle_refs: parse_ref_sequence(&fields[4], "prior-lifecycle")?,
        effect_log_refs: parse_ref_sequence(&fields[5], "effect-log")?,
    })
}

fn parse_link_sequence(value: &Value<IOValue>) -> Result<Vec<ServiceLink>> {
    parse_iovalue_sequence(value, "links")?.iter().map(service_records::parse_service_link).collect()
}

fn parse_monitor_sequence(value: &Value<IOValue>) -> Result<Vec<ServiceMonitor>> {
    parse_iovalue_sequence(value, "monitors")?
        .iter()
        .map(service_records::parse_service_monitor)
        .collect()
}

fn parse_iovalue_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<IOValue>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    Ok(values.iter().map(value_to_iovalue).collect())
}

fn record_iovalue(value: &Value<IOValue>, label: &str) -> Result<IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_u64(value: &Value<IOValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} N>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} STRING>")))?;
    required_string(&fields[0], label)
}

fn record_optional_ref(value: &Value<IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} OPTION>")))?;
    if fields[0].collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = fields[0]
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    let reference = required_string(&some[0], label)?;
    require_ref(&reference, label)?;
    Ok(Some(reference))
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    let refs = values.iter().map(|value| required_ref(value, label)).collect::<Result<Vec<_>>>()?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn field_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<Value<IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} [...]>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let checks = field_sequence(value, "checks")?;
    ensure_count_at_most(checks.len(), "checks")?;
    let mut parsed = Vec::with_capacity(checks.len());
    for check in checks {
        let check = value_to_iovalue(&check);
        let check_fields = check
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected <check NAME STATUS>"))?;
        parsed.push((
            required_string(&check_fields[0], "check name")?,
            required_string(&check_fields[1], "check status")?,
        ));
    }
    Ok(parsed)
}

fn require_schema(value: &Value<IOValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {expected} for {label}, got {actual}")))
    }
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("missing passing check {name} for {label}")))
}

fn refs_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn checks_value(values: &[&str]) -> IOValue {
    record("checks", vec![sequence(
        values.iter().map(|value| record("check", vec![string(value), string("pass")])).collect(),
    )])
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: Option<&str>, label: &str) -> Result<()> {
    if let Some(reference) = reference {
        require_ref(reference, label)
    } else {
        Ok(())
    }
}

fn validate_service_id(value: &str, label: &str) -> Result<()> {
    if value.starts_with("svc:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected svc: service id for {label}, got {value}")))
    }
}

fn required_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = required_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    if reference.starts_with("blake3:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected blake3 ref for {label}, got {reference}")))
    }
}

fn required_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn synthetic_ref(label: &str) -> Result<String> {
    canonical_hash(&record("service-supervision-fixture-ref", vec![string(label)]))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::catalog;
    use crate::catalog::CatalogListInput;
    use crate::catalog::CatalogVisibilityInput;
    use crate::catalog_mcp;
    use crate::ledger;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("service-supervision-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-{label}-{id}"));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn suite_with_attempt(attempt: u64) -> IOValue {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.restart_attempt = attempt;
        service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state: suite.owned_state.value,
            restart_attempt: attempt,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("suite with attempt")
    }

    #[test]
    fn failure_notifies_monitors_and_restart_passes() {
        let suite_value = supervision_fixture_suite_value().expect("fixture suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        assert_eq!(run.monitor_notifications.len(), 2);
        assert_eq!(run.scheduled_demands.len(), 1);
        let decision = service_records::parse_service_restart_decision(&run.restart_decisions[0]).expect("decision");
        assert_eq!(decision.decision, "pass");
        let lifecycle =
            service_records::parse_service_lifecycle_receipt(&run.lifecycle_receipts[0]).expect("lifecycle");
        assert_eq!(lifecycle.operation, "fail");
        assert_eq!(lifecycle.supervision_refs.len(), 5);
        replay_service_supervision_report(&run.value).expect("replay supervision report");
    }

    #[test]
    fn restart_budget_exhausted_cleans_and_publishes_final_status() {
        let suite_value = suite_with_attempt(2);
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        assert!(run.scheduled_demands.is_empty());
        assert_eq!(run.cleanup_receipts.len(), 1);
        assert_eq!(run.retractions.len(), 5);
        assert_eq!(run.statuses.len(), 2);
        let decision = service_records::parse_service_restart_decision(&run.restart_decisions[0]).expect("decision");
        assert_eq!(decision.decision, "deny");
        assert!(decision.diagnostics.iter().any(|diagnostic| diagnostic.contains("budget")));
    }

    #[test]
    fn revocation_retracts_owned_state_and_binds_retention() {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.evidence.revocation_refs = vec![test_ref("revocation")];
        let suite_value = service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state: suite.owned_state.value,
            restart_attempt: 0,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("revoked suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run revoked supervision");
        assert_eq!(run.cleanup_receipts.len(), 1);
        assert_eq!(run.retention_inputs.len(), 1);
        let cleanup = service_records::parse_service_cleanup_receipt(&run.cleanup_receipts[0]).expect("cleanup");
        assert_eq!(cleanup.decision, "pass");
        assert_eq!(cleanup.revocation_refs.len(), 1);
        assert_eq!(cleanup.retraction_refs.len(), 5);
    }

    #[test]
    fn foreign_state_is_not_deleted() {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.restart_attempt = 2;
        let mut owned = suite.owned_state.clone();
        owned.foreign_ref_claims = vec![test_ref("foreign")];
        let owned_state = service_owned_state_value(&ServiceOwnedStateInput {
            service_id: owned.service_id,
            manifest_ref: owned.manifest_ref,
            owned_assertion_refs: owned.owned_assertion_refs,
            observer_refs: owned.observer_refs,
            live_ref_refs: owned.live_ref_refs,
            exposed_ref_refs: owned.exposed_ref_refs,
            pending_effect_refs: owned.pending_effect_refs,
            foreign_ref_claims: owned.foreign_ref_claims,
        })
        .expect("owned state with foreign claim");
        let suite_value = service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state,
            restart_attempt: 2,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("foreign suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run foreign cleanup");
        assert!(run.retractions.is_empty());
        let cleanup = service_records::parse_service_cleanup_receipt(&run.cleanup_receipts[0]).expect("cleanup");
        assert_eq!(cleanup.decision, "deny");
        assert!(cleanup.diagnostics.iter().any(|diagnostic| diagnostic.contains("foreign")));
    }

    #[test]
    fn resource_denial_prevents_restart_and_cleans_owned_state() {
        let mut suite = parse_service_supervision_suite(&supervision_fixture_suite_value().expect("fixture suite"))
            .expect("parse fixture suite");
        suite.evidence.resource_refs.clear();
        let suite_value = service_supervision_suite_value(&ServiceSupervisionSuiteInput {
            manifest: suite.manifest.value,
            links: suite.links.into_iter().map(|link| link.value).collect(),
            monitors: suite.monitors.into_iter().map(|monitor| monitor.value).collect(),
            restart_policy: suite.restart_policy.value,
            owned_state: suite.owned_state.value,
            restart_attempt: 0,
            logical_step: 0,
            evidence: suite.evidence,
        })
        .expect("resource denied suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run resource denied supervision");
        assert!(run.scheduled_demands.is_empty());
        assert_eq!(run.cleanup_receipts.len(), 1);
        let decision = service_records::parse_service_restart_decision(&run.restart_decisions[0]).expect("decision");
        assert_eq!(decision.decision, "deny");
        assert!(decision.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource")));
    }

    #[test]
    fn replay_detects_monitor_restart_and_cleanup_divergence() {
        let suite_value = suite_with_attempt(2);
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        let mut monitor_report = parse_service_supervision_report(&run.value).expect("parse report");
        monitor_report.monitor_notifications.reverse();
        assert!(replay_service_supervision_report(&report_from_parts(&suite_value, &monitor_report)).is_err());

        let mut restart_report = parse_service_supervision_report(&run.value).expect("parse report");
        let decision =
            service_records::parse_service_restart_decision(&restart_report.restart_decisions[0]).expect("decision");
        restart_report.restart_decisions[0] =
            service_records::service_restart_decision_value(&ServiceRestartDecisionInput {
                decision: decision.decision,
                service_id: decision.service_id,
                manifest_ref: decision.manifest_ref,
                policy_ref: decision.policy_ref,
                attempt: decision.attempt,
                max_attempts: decision.max_attempts,
                window_step: decision.window_step,
                backoff_slot: decision.backoff_slot,
                prior_lifecycle_refs: decision.prior_lifecycle_refs,
                authority_refs: decision.authority_refs,
                resource_refs: decision.resource_refs,
                diagnostics: vec!["tampered restart diagnostic".to_string()],
            })
            .expect("tampered restart decision");
        assert!(replay_service_supervision_report(&report_from_parts(&suite_value, &restart_report)).is_err());

        let mut cleanup_report = parse_service_supervision_report(&run.value).expect("parse report");
        cleanup_report.retractions.pop();
        assert!(replay_service_supervision_report(&report_from_parts(&suite_value, &cleanup_report)).is_err());
    }

    fn report_from_parts(suite_value: &IOValue, report: &ServiceSupervisionRun) -> IOValue {
        service_supervision_report_value(ReportValueInput {
            suite_value,
            failure_markers: &report.failure_markers,
            statuses: &report.statuses,
            lifecycle_receipts: &report.lifecycle_receipts,
            monitor_notifications: &report.monitor_notifications,
            restart_decisions: &report.restart_decisions,
            scheduled_demands: &report.scheduled_demands,
            cleanup_receipts: &report.cleanup_receipts,
            retractions: &report.retractions,
            retention_inputs: &report.retention_inputs,
        })
        .expect("report from parts")
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_supervision_artifacts() {
        let suite_value = supervision_fixture_suite_value().expect("fixture suite");
        let run = run_service_supervision_suite_value(&suite_value).expect("run supervision");
        assert_eq!(ledger::artifact_kind(&suite_value), "service-supervision-suite");
        assert_eq!(ledger::artifact_kind(&run.value), "service-supervision-report");
        assert_eq!(ledger::artifact_kind(&run.failure_markers[0]), "service-failure");
        assert_eq!(ledger::artifact_kind(&run.monitor_notifications[0]), "service-monitor-notification");

        let denied_suite_value = suite_with_attempt(2);
        let denied = run_service_supervision_suite_value(&denied_suite_value).expect("denied supervision");
        assert_eq!(ledger::artifact_kind(&denied.cleanup_receipts[0]), "service-cleanup-receipt");
        assert_eq!(ledger::artifact_kind(&denied.retractions[0]), "service-retraction");
        assert_eq!(ledger::artifact_kind(&denied.retention_inputs[0]), "service-retention-input");

        let dir = temp_dir("service-supervision-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let imported = ledger::import_artifact(&ledger_root, &run.value).expect("ledger import supervision report");
        assert_eq!(imported.artifact_kind, "service-supervision-report");
        let listed = catalog::list(&registry, Some(&ledger_root), &CatalogListInput {
            kind: Some("service-supervision-report".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("catalog list supervision report");
        assert_eq!(listed.items.len(), 1);
        assert!(
            to_text(&listed.value)
                .expect("render catalog result")
                .contains("ledger-kind:service-supervision-report")
        );
        let request = catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "service-supervision-report",
        )])])
        .expect("MCP request");
        let mcp = catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list supervision report");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render MCP response").contains("service-supervision-report"));
    }

    #[test]
    fn malformed_os_parentage_is_not_supervision_evidence() {
        let value = parse_text(
            "<service-link-v1 \"molten.service.link.v1\" <supervisor-id \"supervisor:web\"> \
             <parent-service \"1234\"> <child-service \"svc:web\"> <propagation \"restart\"> \
             <policy []> <checks [<check \"logical-supervision\" \"pass\">]>>",
        )
        .expect("parse malformed link");
        assert!(service_records::parse_service_link(&value).is_err());
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_cleanup_bounded_and_monitor_order_deterministic(tc: TestCase) {
        let attempt = tc.draw(generators::integers::<u64>().min_value(0).max_value(3));
        let suite_value = suite_with_attempt(attempt);
        let run = run_service_supervision_suite_value(&suite_value).expect("generated supervision run");
        let replay = replay_service_supervision_report(&run.value).expect("generated replay");
        assert_eq!(replay.decision, "pass");
        let is_restart_denied = attempt >= 2;
        if is_restart_denied {
            assert_eq!(run.cleanup_receipts.len(), 1);
            assert!(run.scheduled_demands.is_empty());
        } else {
            assert!(run.cleanup_receipts.is_empty());
            assert_eq!(run.scheduled_demands.len(), 1);
        }
        let second_run = run_service_supervision_suite_value(&suite_value).expect("rerun generated supervision");
        assert_eq!(run.monitor_notifications, second_run.monitor_notifications);
    }
}
