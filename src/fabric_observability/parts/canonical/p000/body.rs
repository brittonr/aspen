use super::*;

pub const FABRIC_OBSERVATION_PORT_ID: &str = "molten.fabric.observability";
pub const FABRIC_INTEGRITY_PORT_ID: &str = "molten.fabric.integrity";
pub const FABRIC_OBSERVABILITY_PORT_VERSION: &str = "v1";

const PROFILE_RECORD: &str = "fabric-observation-profile-v1";
const DESCRIPTOR_RECORD: &str = "fabric-metric-descriptor-v1";
const SAMPLE_RECORD: &str = "fabric-metric-sample-v1";
const EVENT_RECORD: &str = "fabric-observation-event-v1";
const HEALTH_INPUT_RECORD: &str = "fabric-health-input-v1";
const READINESS_POLICY_RECORD: &str = "fabric-readiness-policy-v1";
const HEALTH_DECISION_RECORD: &str = "fabric-health-decision-v1";
const INTEGRITY_PLAN_RECORD: &str = "fabric-integrity-plan-v1";
const SCAN_OBSERVATION_RECORD: &str = "fabric-scan-observation-v1";
const INTEGRITY_RESULT_RECORD: &str = "fabric-integrity-result-v1";
const INTEGRITY_FINDING_RECORD: &str = "fabric-integrity-finding-v1";
const ADAPTER_PROFILE_RECORD: &str = "fabric-observation-adapter-profile-v1";
const ADAPTER_OUTCOME_RECORD: &str = "fabric-observation-adapter-outcome-v1";
const ADAPTER_STATUS_RECORD: &str = "fabric-observation-adapter-status-v1";
const SNAPSHOT_RECORD: &str = "fabric-observation-snapshot-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalArtifact<T> {
    pub artifact: T,
    pub artifact_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.fabric_observability.model]
pub fn canonical_observation_profile(
    profile: &ObservationProfile,
) -> crate::error::Result<CanonicalArtifact<ObservationProfile>> {
    let issues = validate_observation_profile(profile);
    require_valid("observation profile", &issues)?;
    canonical_artifact(profile.clone(), observation_profile_value(profile))
}

pub fn canonical_metric_descriptor(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
) -> crate::error::Result<CanonicalArtifact<MetricDescriptor>> {
    let issues = validate_metric_descriptor(profile, descriptor);
    require_valid("metric descriptor", &issues)?;
    canonical_artifact(descriptor.clone(), metric_descriptor_value(descriptor))
}

pub fn canonical_metric_sample(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    sample: &MetricSample,
    as_of_tick: u64,
) -> crate::error::Result<CanonicalArtifact<MetricSample>> {
    let sanitized = validate_metric_sample(profile, descriptor, sample, as_of_tick)
        .map_err(|issues| validation_error("metric sample", &issues))?;
    canonical_artifact(sanitized.clone(), metric_sample_value(&sanitized))
}

pub fn canonical_observation_event(
    profile: &ObservationProfile,
    event: &ObservationEvent,
    as_of_tick: u64,
) -> crate::error::Result<CanonicalArtifact<ObservationEvent>> {
    let sanitized =
        validate_event(profile, event, as_of_tick).map_err(|issues| validation_error("observation event", &issues))?;
    canonical_artifact(sanitized.clone(), observation_event_value(&sanitized))
}

// r[impl molten.fabric_observability.health_scope]
pub fn canonical_health_input(
    profile: &ObservationProfile,
    input: &HealthInput,
    as_of_tick: u64,
) -> crate::error::Result<CanonicalArtifact<HealthInput>> {
    let mut issues = validate_observation_profile(profile);
    validate_health_input(profile, input, &mut issues);
    if as_of_tick > input.context.valid_until_tick {
        issues.push(ObservabilityIssue::ObservationStale(input.context.source_id.clone()));
    }
    require_valid("health input", &issues)?;
    canonical_artifact(input.clone(), health_input_value(input))
}

pub fn canonical_readiness_policy(
    profile: &ObservationProfile,
    policy: &ReadinessPolicy,
) -> crate::error::Result<CanonicalArtifact<ReadinessPolicy>> {
    let mut issues = validate_observation_profile(profile);
    validate_readiness_policy(profile, policy, &mut issues);
    require_valid("readiness policy", &issues)?;
    canonical_artifact(policy.clone(), readiness_policy_value(policy))
}

pub fn canonical_health_decision(decision: &HealthDecision) -> crate::error::Result<CanonicalArtifact<HealthDecision>> {
    canonical_artifact(decision.clone(), health_decision_value(decision))
}

// r[impl molten.fabric_observability.integrity_readonly]
pub fn canonical_integrity_plan(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
) -> crate::error::Result<CanonicalArtifact<IntegrityPlan>> {
    let probe = evaluate_integrity_plan(profile, plan, &[], &ScanCompletion {
        scanned_items: 0,
        declared_items: plan.targets.len(),
        exhausted: false,
        cancelled: false,
        unavailable: false,
    });
    let structural_issues = probe
        .issues
        .iter()
        .filter(|issue| !matches!(issue, ObservabilityIssue::PartialScan))
        .cloned()
        .collect::<Vec<_>>();
    require_valid("integrity plan", &structural_issues)?;
    canonical_artifact(plan.clone(), integrity_plan_value(plan))
}

pub fn canonical_scan_observation(
    plan: &IntegrityPlan,
    observation: &ScanObservation,
) -> crate::error::Result<CanonicalArtifact<ScanObservation>> {
    let issues = validate_scan_observation(plan, observation);
    require_valid("scan observation", &issues)?;
    canonical_artifact(observation.clone(), scan_observation_value(observation))
}

pub fn canonical_integrity_result(
    profile: &ObservationProfile,
    result: &IntegrityResult,
) -> crate::error::Result<CanonicalArtifact<IntegrityResult>> {
    let issues = validate_integrity_result(profile, result);
    require_valid("integrity result", &issues)?;
    canonical_artifact(result.clone(), integrity_result_value(result))
}

// r[impl molten.fabric_observability.adapter_contract]
pub fn canonical_observation_adapter_profile(
    observation_profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
) -> crate::error::Result<CanonicalArtifact<ObservationAdapterProfile>> {
    let issues = validate_adapter_profile(observation_profile, adapter);
    require_valid("observation adapter profile", &issues)?;
    canonical_artifact(adapter.clone(), adapter_profile_value(adapter))
}

pub fn canonical_adapter_outcome(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    outcome: &AdapterOutcome,
) -> crate::error::Result<CanonicalArtifact<AdapterOutcome>> {
    let issues = validate_adapter_outcome(profile, adapter, outcome);
    require_valid("adapter outcome", &issues)?;
    canonical_artifact(outcome.clone(), adapter_outcome_value(outcome))
}

pub fn canonical_adapter_status(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    status: &ObservationAdapterStatus,
) -> crate::error::Result<CanonicalArtifact<ObservationAdapterStatus>> {
    let issues = validate_adapter_status(profile, adapter, status);
    require_valid("adapter status", &issues)?;
    canonical_artifact(status.clone(), adapter_status_value(status))
}

// r[impl molten.fabric_observability.health_scope]
pub fn canonical_observation_snapshot(
    profile: &ObservationProfile,
    snapshot: &ObservationSnapshot,
    as_of_tick: u64,
) -> crate::error::Result<CanonicalArtifact<ObservationSnapshot>> {
    let issues = validate_snapshot(profile, snapshot, as_of_tick);
    require_valid("observation snapshot", &issues)?;
    canonical_artifact(snapshot.clone(), observation_snapshot_value(snapshot))
}

pub fn fabric_observability_port_descriptors(profile_ref: &str) -> Vec<crate::fabric::FabricPortDescriptor> {
    vec![
        observation_port_descriptor(profile_ref),
        integrity_port_descriptor(profile_ref),
    ]
}

fn observation_port_descriptor(profile_ref: &str) -> crate::fabric::FabricPortDescriptor {
    crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: FABRIC_OBSERVATION_PORT_ID.to_string(),
        version: FABRIC_OBSERVABILITY_PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Evidence,
        operation_classes: vec![
            "emit-event".to_string(),
            "emit-sample".to_string(),
            "evaluate-health".to_string(),
            "export".to_string(),
            "snapshot".to_string(),
            "status".to_string(),
        ],
        input_schema_refs: vec![METRIC_SAMPLE_SCHEMA.to_string(), OBSERVATION_PROFILE_SCHEMA.to_string()],
        output_schema_refs: vec![
            OBSERVATION_ADAPTER_STATUS_SCHEMA.to_string(),
            OBSERVATION_SNAPSHOT_SCHEMA.to_string(),
        ],
        authority_requirements: vec![
            crate::fabric::FabricAuthority::Time,
            crate::fabric::FabricAuthority::Resources,
            crate::fabric::FabricAuthority::Evidence,
        ],
        resource_requirements: vec![
            crate::fabric::FabricResource::Memory,
            crate::fabric::FabricResource::NetworkBytes,
            crate::fabric::FabricResource::QueueDepth,
            crate::fabric::FabricResource::LogicalTime,
            crate::fabric::FabricResource::Diagnostics,
        ],
        determinism: crate::fabric::DeterminismClass::ExternalEffect,
        replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: "bounded-canonical-observation-adapter".to_string(),
        conformance_refs: vec![profile_ref.to_string()],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn integrity_port_descriptor(profile_ref: &str) -> crate::fabric::FabricPortDescriptor {
    crate::fabric::FabricPortDescriptor {
        schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: FABRIC_INTEGRITY_PORT_ID.to_string(),
        version: FABRIC_OBSERVABILITY_PORT_VERSION.to_string(),
        class: crate::fabric::FabricPortClass::Evidence,
        operation_classes: vec![
            "plan".to_string(),
            "scan".to_string(),
            "status".to_string(),
            "verify".to_string(),
        ],
        input_schema_refs: vec![INTEGRITY_PLAN_SCHEMA.to_string()],
        output_schema_refs: vec![
            INTEGRITY_FINDING_SCHEMA.to_string(),
            SCAN_OBSERVATION_SCHEMA.to_string(),
        ],
        authority_requirements: vec![
            crate::fabric::FabricAuthority::DurableState,
            crate::fabric::FabricAuthority::Resources,
            crate::fabric::FabricAuthority::Evidence,
        ],
        resource_requirements: vec![
            crate::fabric::FabricResource::Memory,
            crate::fabric::FabricResource::StorageBytes,
            crate::fabric::FabricResource::Diagnostics,
        ],
        determinism: crate::fabric::DeterminismClass::ExternalEffect,
        replay: crate::fabric::ReplayClass::RecordedEffectRequired,
        implementation_profile: "read-only-capability-rooted-scan".to_string(),
        conformance_refs: vec![profile_ref.to_string()],
        non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn observation_profile_value(profile: &ObservationProfile) -> preserves::IOValue {
    record(PROFILE_RECORD, vec![
        string(OBSERVATION_PROFILE_SCHEMA),
        field("profile-id", string(&profile.profile_id)),
        field("declared-profile-ref", string(&profile.profile_ref)),
        field("bounds", bounds_value(&profile.bounds)),
        field("redaction-rules", sequence(profile.redaction_rules.iter().map(redaction_rule_value).collect())),
        field("non-claims", non_claims_value(&profile.non_claims)),
        checks(&[
            "bounded-cardinality",
            "classified-redaction",
            "telemetry-is-not-authority",
        ]),
    ])
}
