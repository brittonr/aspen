use preserves::IOValue;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;

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
    pub value: IOValue,
}

// r[impl molten.fabric_observability.model]
pub fn canonical_observation_profile(profile: &ObservationProfile) -> Result<CanonicalArtifact<ObservationProfile>> {
    let issues = validate_observation_profile(profile);
    require_valid("observation profile", &issues)?;
    canonical_artifact(profile.clone(), observation_profile_value(profile))
}

pub fn canonical_metric_descriptor(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
) -> Result<CanonicalArtifact<MetricDescriptor>> {
    let issues = validate_metric_descriptor(profile, descriptor);
    require_valid("metric descriptor", &issues)?;
    canonical_artifact(descriptor.clone(), metric_descriptor_value(descriptor))
}

pub fn canonical_metric_sample(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    sample: &MetricSample,
    as_of_tick: u64,
) -> Result<CanonicalArtifact<MetricSample>> {
    let sanitized = validate_metric_sample(profile, descriptor, sample, as_of_tick)
        .map_err(|issues| validation_error("metric sample", &issues))?;
    canonical_artifact(sanitized.clone(), metric_sample_value(&sanitized))
}

pub fn canonical_observation_event(
    profile: &ObservationProfile,
    event: &ObservationEvent,
    as_of_tick: u64,
) -> Result<CanonicalArtifact<ObservationEvent>> {
    let sanitized =
        validate_event(profile, event, as_of_tick).map_err(|issues| validation_error("observation event", &issues))?;
    canonical_artifact(sanitized.clone(), observation_event_value(&sanitized))
}

// r[impl molten.fabric_observability.health_scope]
pub fn canonical_health_input(
    profile: &ObservationProfile,
    input: &HealthInput,
    as_of_tick: u64,
) -> Result<CanonicalArtifact<HealthInput>> {
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
) -> Result<CanonicalArtifact<ReadinessPolicy>> {
    let mut issues = validate_observation_profile(profile);
    validate_readiness_policy(profile, policy, &mut issues);
    require_valid("readiness policy", &issues)?;
    canonical_artifact(policy.clone(), readiness_policy_value(policy))
}

pub fn canonical_health_decision(decision: &HealthDecision) -> Result<CanonicalArtifact<HealthDecision>> {
    canonical_artifact(decision.clone(), health_decision_value(decision))
}

// r[impl molten.fabric_observability.integrity_readonly]
pub fn canonical_integrity_plan(
    profile: &ObservationProfile,
    plan: &IntegrityPlan,
) -> Result<CanonicalArtifact<IntegrityPlan>> {
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
) -> Result<CanonicalArtifact<ScanObservation>> {
    let issues = validate_scan_observation(plan, observation);
    require_valid("scan observation", &issues)?;
    canonical_artifact(observation.clone(), scan_observation_value(observation))
}

pub fn canonical_integrity_result(
    profile: &ObservationProfile,
    result: &IntegrityResult,
) -> Result<CanonicalArtifact<IntegrityResult>> {
    let issues = validate_integrity_result(profile, result);
    require_valid("integrity result", &issues)?;
    canonical_artifact(result.clone(), integrity_result_value(result))
}

// r[impl molten.fabric_observability.adapter_contract]
pub fn canonical_observation_adapter_profile(
    observation_profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
) -> Result<CanonicalArtifact<ObservationAdapterProfile>> {
    let issues = validate_adapter_profile(observation_profile, adapter);
    require_valid("observation adapter profile", &issues)?;
    canonical_artifact(adapter.clone(), adapter_profile_value(adapter))
}

pub fn canonical_adapter_outcome(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    outcome: &AdapterOutcome,
) -> Result<CanonicalArtifact<AdapterOutcome>> {
    let issues = validate_adapter_outcome(profile, adapter, outcome);
    require_valid("adapter outcome", &issues)?;
    canonical_artifact(outcome.clone(), adapter_outcome_value(outcome))
}

pub fn canonical_adapter_status(
    profile: &ObservationProfile,
    adapter: &ObservationAdapterProfile,
    status: &ObservationAdapterStatus,
) -> Result<CanonicalArtifact<ObservationAdapterStatus>> {
    let issues = validate_adapter_status(profile, adapter, status);
    require_valid("adapter status", &issues)?;
    canonical_artifact(status.clone(), adapter_status_value(status))
}

// r[impl molten.fabric_observability.health_scope]
pub fn canonical_observation_snapshot(
    profile: &ObservationProfile,
    snapshot: &ObservationSnapshot,
    as_of_tick: u64,
) -> Result<CanonicalArtifact<ObservationSnapshot>> {
    let issues = validate_snapshot(profile, snapshot, as_of_tick);
    require_valid("observation snapshot", &issues)?;
    canonical_artifact(snapshot.clone(), observation_snapshot_value(snapshot))
}

pub fn fabric_observability_port_descriptors(profile_ref: &str) -> Vec<FabricPortDescriptor> {
    vec![
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: FABRIC_OBSERVATION_PORT_ID.to_string(),
            version: FABRIC_OBSERVABILITY_PORT_VERSION.to_string(),
            class: FabricPortClass::Evidence,
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
                FabricAuthority::Time,
                FabricAuthority::Resources,
                FabricAuthority::Evidence,
            ],
            resource_requirements: vec![
                FabricResource::Memory,
                FabricResource::NetworkBytes,
                FabricResource::QueueDepth,
                FabricResource::LogicalTime,
                FabricResource::Diagnostics,
            ],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: "bounded-canonical-observation-adapter".to_string(),
            conformance_refs: vec![profile_ref.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        },
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: FABRIC_INTEGRITY_PORT_ID.to_string(),
            version: FABRIC_OBSERVABILITY_PORT_VERSION.to_string(),
            class: FabricPortClass::Evidence,
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
                FabricAuthority::DurableState,
                FabricAuthority::Resources,
                FabricAuthority::Evidence,
            ],
            resource_requirements: vec![
                FabricResource::Memory,
                FabricResource::StorageBytes,
                FabricResource::Diagnostics,
            ],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: "read-only-capability-rooted-scan".to_string(),
            conformance_refs: vec![profile_ref.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        },
    ]
}

fn observation_profile_value(profile: &ObservationProfile) -> IOValue {
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

fn metric_descriptor_value(descriptor: &MetricDescriptor) -> IOValue {
    record(DESCRIPTOR_RECORD, vec![
        string(METRIC_DESCRIPTOR_SCHEMA),
        field("descriptor-id", string(&descriptor.descriptor_id)),
        field("declared-descriptor-ref", string(&descriptor.descriptor_ref)),
        field("profile-ref", string(&descriptor.profile_ref)),
        field("name", string(&descriptor.name)),
        field("unit", string(&descriptor.unit)),
        field("kind", string(descriptor.kind.as_str())),
        field("aggregation", string(descriptor.aggregation.as_str())),
        field("allowed-label-names", strings_value(descriptor.allowed_label_names.iter().map(String::as_str))),
        field("description", string(&descriptor.description)),
        checks(&["exporter-neutral", "bounded-label-vocabulary"]),
    ])
}

fn metric_sample_value(sample: &MetricSample) -> IOValue {
    record(SAMPLE_RECORD, vec![
        string(METRIC_SAMPLE_SCHEMA),
        field("declared-sample-ref", string(&sample.sample_ref)),
        field("descriptor-ref", string(&sample.descriptor_ref)),
        field("context", context_value(&sample.context)),
        field("labels", labels_value(&sample.labels)),
        field("value", i64_value(sample.value)),
        checks(&["labels-validated", "secret-values-excluded"]),
    ])
}

fn observation_event_value(event: &ObservationEvent) -> IOValue {
    record(EVENT_RECORD, vec![
        string(OBSERVATION_EVENT_SCHEMA),
        field("declared-event-ref", string(&event.event_ref)),
        field("kind", string(&event.event_kind)),
        field("severity", string(event.severity.as_str())),
        field("context", context_value(&event.context)),
        field("detail", string(&event.detail)),
        field("attributes", labels_value(&event.attributes)),
        checks(&["bounded-event", "secret-values-excluded"]),
    ])
}

fn health_input_value(input: &HealthInput) -> IOValue {
    record(HEALTH_INPUT_RECORD, vec![
        string(HEALTH_INPUT_SCHEMA),
        field("declared-health-ref", string(&input.health_ref)),
        field("context", context_value(&input.context)),
        field("state", string(input.state.as_str())),
        field("diagnostic-refs", strings_value(input.diagnostic_refs.iter().map(String::as_str))),
        checks(&["freshness-explicit", "health-is-observation-only"]),
    ])
}

fn readiness_policy_value(policy: &ReadinessPolicy) -> IOValue {
    record(READINESS_POLICY_RECORD, vec![
        string(READINESS_POLICY_SCHEMA),
        field("policy-ref", string(&policy.policy_ref)),
        field("target-scope", string(policy.target_scope.as_str())),
        field("required-source-ids", strings_value(policy.required_source_ids.iter().map(String::as_str))),
        field("scope-evidence-refs", strings_value(policy.scope_evidence_refs.iter().map(String::as_str))),
        field("allow-degraded", bool_value(policy.allow_degraded)),
        field("as-of-tick", u64_value(policy.as_of_tick)),
        checks(&["scope-promotion-explicit", "unavailable-does-not-pass"]),
    ])
}

fn health_decision_value(decision: &HealthDecision) -> IOValue {
    record(HEALTH_DECISION_RECORD, vec![
        string(HEALTH_DECISION_SCHEMA),
        field("prior-state", string(decision.prior_state.as_str())),
        field("state", string(decision.state.as_str())),
        field("readiness", string(decision.readiness.as_str())),
        field("scope", string(decision.scope.as_str())),
        field("supporting-health-refs", strings_value(decision.supporting_health_refs.iter().map(String::as_str))),
        field("issues", issues_value(&decision.issues)),
        checks(&[
            "freshness-evaluated",
            "scope-promotion-explicit",
            "health-is-not-authority",
        ]),
    ])
}

fn integrity_plan_value(plan: &IntegrityPlan) -> IOValue {
    record(INTEGRITY_PLAN_RECORD, vec![
        string(INTEGRITY_PLAN_SCHEMA),
        field("declared-plan-ref", string(&plan.plan_ref)),
        field("profile-ref", string(&plan.profile_ref)),
        field("scope-ref", string(&plan.scope_ref)),
        field("generation", u64_value(plan.generation)),
        field("read-only", bool_value(plan.read_only)),
        field("require-complete", bool_value(plan.require_complete)),
        field("max-items", usize_value(plan.max_items)),
        field("max-findings", usize_value(plan.max_findings)),
        field("targets", sequence(plan.targets.iter().map(integrity_target_value).collect())),
        field("resource-ref", string(&plan.resource_ref)),
        field("policy-refs", strings_value(plan.policy_refs.iter().map(String::as_str))),
        field("evidence-refs", strings_value(plan.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&plan.non_claims)),
        checks(&["read-only-default", "bounded-targets", "repair-authority-excluded"]),
    ])
}

fn scan_observation_value(observation: &ScanObservation) -> IOValue {
    record(SCAN_OBSERVATION_RECORD, vec![
        string(SCAN_OBSERVATION_SCHEMA),
        field("declared-observation-ref", string(&observation.observation_ref)),
        field("plan-ref", string(&observation.plan_ref)),
        field("item-ref", string(&observation.item_ref)),
        field("kind", string(observation.kind.as_str())),
        field("status", string(observation.status.as_str())),
        field("observed-content-ref", optional_string(observation.observed_content_ref.as_deref())),
        field("observed-length", optional_u64(observation.observed_length)),
        field("evidence-refs", strings_value(observation.evidence_refs.iter().map(String::as_str))),
        checks(&["read-only-observation", "capability-rooted-shell"]),
    ])
}

fn integrity_result_value(result: &IntegrityResult) -> IOValue {
    record(INTEGRITY_RESULT_RECORD, vec![
        string(INTEGRITY_RESULT_SCHEMA),
        field("plan-ref", string(&result.plan_ref)),
        field("decision", string(result.decision.as_str())),
        field("scanned-items", usize_value(result.scanned_items)),
        field("declared-items", usize_value(result.declared_items)),
        field("findings", sequence(result.findings.iter().map(integrity_finding_value).collect())),
        field("complete", bool_value(result.complete)),
        field("mutation-performed", bool_value(result.mutation_performed)),
        field("issues", issues_value(&result.issues)),
        checks(&[
            "partial-scan-cannot-pass",
            "findings-grant-no-mutation-authority",
            "result-scope-bounded-to-plan",
        ]),
    ])
}

fn adapter_profile_value(adapter: &ObservationAdapterProfile) -> IOValue {
    record(ADAPTER_PROFILE_RECORD, vec![
        string(OBSERVATION_ADAPTER_PROFILE_SCHEMA),
        field("adapter-id", string(&adapter.adapter_id)),
        field("declared-adapter-ref", string(&adapter.adapter_ref)),
        field("profile-ref", string(&adapter.profile_ref)),
        field("class", string(adapter.class.as_str())),
        field("max-queued-bytes", u64_value(adapter.max_queued_bytes)),
        field("timeout-ticks", u64_value(adapter.timeout_ticks)),
        field("drop-on-backpressure", bool_value(adapter.drop_on_backpressure)),
        field("required", bool_value(adapter.required)),
        field("evidence-refs", strings_value(adapter.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&adapter.non_claims)),
        checks(&["versioned-adapter", "terminal-failures-explicit"]),
    ])
}

fn adapter_outcome_value(outcome: &AdapterOutcome) -> IOValue {
    record(ADAPTER_OUTCOME_RECORD, vec![
        string(OBSERVATION_ADAPTER_OUTCOME_SCHEMA),
        field("operation-ref", string(&outcome.operation_ref)),
        field("adapter-ref", string(&outcome.adapter_ref)),
        field("payload-ref", string(&outcome.payload_ref)),
        field("kind", string(outcome.kind.as_str())),
        field("dropped-observations", u64_value(outcome.dropped_observations)),
        field("service-policy-signal", bool_value(outcome.service_policy_signal)),
        field("issues", issues_value(&outcome.issues)),
        checks(&["bounded-terminal-outcome", "exporter-failure-does-not-mutate-service"]),
    ])
}

fn adapter_status_value(status: &ObservationAdapterStatus) -> IOValue {
    record(ADAPTER_STATUS_RECORD, vec![
        string(OBSERVATION_ADAPTER_STATUS_SCHEMA),
        field("adapter-ref", string(&status.adapter_ref)),
        field("class", string(status.class.as_str())),
        field("kind", string(status.kind.as_str())),
        field("observed-tick", u64_value(status.observed_tick)),
        field("queued-bytes", u64_value(status.queued_bytes)),
        field("dropped-observations", u64_value(status.dropped_observations)),
        field("evidence-refs", strings_value(status.evidence_refs.iter().map(String::as_str))),
        field("issues", issues_value(&status.issues)),
        checks(&["terminal-status-explicit", "adapter-status-is-not-service-authority"]),
    ])
}

fn observation_snapshot_value(snapshot: &ObservationSnapshot) -> IOValue {
    record(SNAPSHOT_RECORD, vec![
        string(OBSERVATION_SNAPSHOT_SCHEMA),
        field("snapshot-id", string(&snapshot.snapshot_id)),
        field("profile-ref", string(&snapshot.profile_ref)),
        field("scope", string(snapshot.scope.as_str())),
        field("generation", u64_value(snapshot.generation)),
        field("as-of-tick", u64_value(snapshot.as_of_tick)),
        field("valid-until-tick", u64_value(snapshot.valid_until_tick)),
        field("series", sequence(snapshot.series.iter().map(aggregated_series_value).collect())),
        field("event-refs", strings_value(snapshot.event_refs.iter().map(String::as_str))),
        field("health-refs", strings_value(snapshot.health_refs.iter().map(String::as_str))),
        field("integrity-result-refs", strings_value(snapshot.integrity_result_refs.iter().map(String::as_str))),
        field("adapter-outcome-refs", strings_value(snapshot.adapter_outcome_refs.iter().map(String::as_str))),
        field("evidence-refs", strings_value(snapshot.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&snapshot.non_claims)),
        checks(&[
            "bounded-operator-view",
            "freshness-explicit",
            "snapshot-is-not-release-authority",
        ]),
    ])
}

fn bounds_value(bounds: &ObservationBounds) -> IOValue {
    record("observation-bounds", vec![
        field("max-descriptors", usize_value(bounds.max_descriptors)),
        field("max-labels-per-sample", usize_value(bounds.max_labels_per_sample)),
        field("max-label-name-bytes", usize_value(bounds.max_label_name_bytes)),
        field("max-label-value-bytes", usize_value(bounds.max_label_value_bytes)),
        field("max-series", usize_value(bounds.max_series)),
        field("max-events", usize_value(bounds.max_events)),
        field("max-event-detail-bytes", usize_value(bounds.max_event_detail_bytes)),
        field("max-queued-bytes", u64_value(bounds.max_queued_bytes)),
        field("max-snapshots", usize_value(bounds.max_snapshots)),
        field("max-scan-items", usize_value(bounds.max_scan_items)),
        field("max-findings", usize_value(bounds.max_findings)),
        field("max-diagnostics", usize_value(bounds.max_diagnostics)),
        field("min-export-interval-ticks", u64_value(bounds.min_export_interval_ticks)),
    ])
}

fn context_value(context: &ObservationContext) -> IOValue {
    record("observation-context", vec![
        field("source-id", string(&context.source_id)),
        field("source-ref", string(&context.source_ref)),
        field("profile-ref", string(&context.profile_ref)),
        field("scope", string(context.scope.as_str())),
        field("generation", u64_value(context.generation)),
        field("observed-tick", u64_value(context.observed_tick)),
        field("valid-until-tick", u64_value(context.valid_until_tick)),
        field("resource-ref", string(&context.resource_ref)),
        field("evidence-refs", strings_value(context.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&context.non_claims)),
    ])
}

fn redaction_rule_value(rule: &RedactionRule) -> IOValue {
    record("redaction-rule", vec![
        field("label-name", string(&rule.label_name)),
        field("class", string(rule.class.as_str())),
        field("marker", string(&rule.marker)),
    ])
}

fn labels_value(labels: &[MetricLabel]) -> IOValue {
    sequence(
        labels
            .iter()
            .map(|label| {
                record("label", vec![
                    field("name", string(&label.name)),
                    field("value", string(&label.value)),
                    field("class", string(label.class.as_str())),
                ])
            })
            .collect(),
    )
}

fn integrity_target_value(target: &IntegrityTarget) -> IOValue {
    record("integrity-target", vec![
        field("item-ref", string(&target.item_ref)),
        field("kind", string(target.kind.as_str())),
        field("expected-content-ref", optional_string(target.expected_content_ref.as_deref())),
        field("expected-length", optional_u64(target.expected_length)),
    ])
}

fn integrity_finding_value(finding: &IntegrityFinding) -> IOValue {
    record(INTEGRITY_FINDING_RECORD, vec![
        string(INTEGRITY_FINDING_SCHEMA),
        field("finding-id", string(&finding.finding_id)),
        field("item-ref", optional_string(finding.item_ref.as_deref())),
        field("class", string(finding.class.as_str())),
        field("expected-ref", optional_string(finding.expected_ref.as_deref())),
        field("observed-ref", optional_string(finding.observed_ref.as_deref())),
        field("recommendation", string(finding.recommendation.as_str())),
        field("grants-mutation-authority", bool_value(finding.grants_mutation_authority)),
    ])
}

fn aggregated_series_value(series: &AggregatedSeries) -> IOValue {
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

fn non_claims_value(non_claims: &[ObservabilityNonClaim]) -> IOValue {
    strings_value(non_claims.iter().map(|claim| claim.as_str()))
}

fn issues_value(issues: &[ObservabilityIssue]) -> IOValue {
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

fn canonical_artifact<T>(artifact: T, value: IOValue) -> Result<CanonicalArtifact<T>> {
    let artifact_ref = canonical_hash(&value)?;
    Ok(CanonicalArtifact {
        artifact,
        artifact_ref,
        value,
    })
}

fn require_valid(label: &str, issues: &[ObservabilityIssue]) -> Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(validation_error(label, issues))
    }
}

fn validation_error(label: &str, issues: &[ObservabilityIssue]) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}

fn checks(names: &[&str]) -> IOValue {
    field(
        "checks",
        sequence(names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect()),
    )
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    sequence(values.map(string).collect())
}

fn optional_string(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn usize_value(value: usize) -> IOValue {
    match u64::try_from(value) {
        Ok(value) => u64_value(value),
        Err(_) => record("usize-overflow", Vec::new()),
    }
}

fn i64_value(value: i64) -> IOValue {
    IOValue::new(value)
}

fn bool_value(value: bool) -> IOValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IOValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn u64_value(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}
