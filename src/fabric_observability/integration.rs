use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub struct HealthProjectionInput<'a> {
    pub source_id: &'a str,
    pub source_ref: &'a str,
    pub profile_ref: &'a str,
    pub scope: ClaimScope,
    pub generation: u64,
    pub observed_tick: u64,
    pub valid_until_tick: u64,
    pub resource_ref: &'a str,
    pub evidence_refs: Vec<String>,
    pub diagnostic_refs: Vec<String>,
}

// r[impl molten.fabric_observability.health_scope]
pub fn system_extension_health_input(
    input: HealthProjectionInput<'_>,
    state: &crate::system_extension::LifecycleState,
) -> HealthInput {
    let mut context = projection_context(&input);
    context.generation = state.generation;
    HealthInput {
        schema: HEALTH_INPUT_SCHEMA.to_string(),
        health_ref: health_projection_ref(&input, state.generation, state.health.as_str()),
        context,
        state: match state.health {
            crate::system_extension::HealthState::Healthy => HealthState::Healthy,
            crate::system_extension::HealthState::Degraded => HealthState::Degraded,
            crate::system_extension::HealthState::Failed | crate::system_extension::HealthState::Quarantined => {
                HealthState::Failed
            }
            crate::system_extension::HealthState::Unknown
            | crate::system_extension::HealthState::Starting
            | crate::system_extension::HealthState::Stopped => HealthState::Unavailable,
        },
        diagnostic_refs: input.diagnostic_refs,
    }
}

// r[impl molten.fabric_observability.health_scope]
pub fn node_health_input(input: HealthProjectionInput<'_>, node_decision: &str) -> Result<HealthInput> {
    let state = match node_decision {
        "pass" | "healthy" => HealthState::Healthy,
        "degraded" => HealthState::Degraded,
        "unavailable" | "stopped" => HealthState::Unavailable,
        "deny" | "failed" => HealthState::Failed,
        other => {
            return Err(MoltenError::invalid_harness(format!("unsupported node health decision {other}")));
        }
    };
    Ok(HealthInput {
        schema: HEALTH_INPUT_SCHEMA.to_string(),
        health_ref: health_projection_ref(&input, input.generation, state.as_str()),
        context: projection_context(&input),
        state,
        diagnostic_refs: input.diagnostic_refs,
    })
}

// r[impl molten.fabric_observability.adapter_contract]
pub fn runtime_counter_sample(
    profile: &ObservationProfile,
    descriptor: &MetricDescriptor,
    sample_ref: String,
    context: ObservationContext,
    labels: Vec<MetricLabel>,
    value: i64,
    as_of_tick: u64,
) -> Result<CanonicalArtifact<MetricSample>> {
    canonical_metric_sample(
        profile,
        descriptor,
        &MetricSample {
            schema: METRIC_SAMPLE_SCHEMA.to_string(),
            sample_ref,
            descriptor_ref: descriptor.descriptor_ref.clone(),
            context,
            labels,
            value,
        },
        as_of_tick,
    )
}

pub struct SnapshotBuildInput<'a> {
    pub snapshot_id: &'a str,
    pub profile_ref: &'a str,
    pub scope: ClaimScope,
    pub generation: u64,
    pub as_of_tick: u64,
    pub valid_until_tick: u64,
    pub series: Vec<AggregatedSeries>,
    pub event_refs: Vec<String>,
    pub health_refs: Vec<String>,
    pub integrity_result_refs: Vec<String>,
    pub adapter_outcome_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

// r[impl molten.fabric_observability.health_scope]
pub fn bounded_operator_snapshot(
    profile: &ObservationProfile,
    input: SnapshotBuildInput<'_>,
) -> Result<CanonicalArtifact<ObservationSnapshot>> {
    let snapshot = ObservationSnapshot {
        schema: OBSERVATION_SNAPSHOT_SCHEMA.to_string(),
        snapshot_id: input.snapshot_id.to_string(),
        profile_ref: input.profile_ref.to_string(),
        scope: input.scope,
        generation: input.generation,
        as_of_tick: input.as_of_tick,
        valid_until_tick: input.valid_until_tick,
        series: input.series,
        event_refs: sorted_refs(input.event_refs),
        health_refs: sorted_refs(input.health_refs),
        integrity_result_refs: sorted_refs(input.integrity_result_refs),
        adapter_outcome_refs: sorted_refs(input.adapter_outcome_refs),
        evidence_refs: sorted_refs(input.evidence_refs),
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    };
    canonical_observation_snapshot(profile, &snapshot, input.as_of_tick)
}

fn projection_context(input: &HealthProjectionInput<'_>) -> ObservationContext {
    ObservationContext {
        source_id: input.source_id.to_string(),
        source_ref: input.source_ref.to_string(),
        profile_ref: input.profile_ref.to_string(),
        scope: input.scope,
        generation: input.generation,
        observed_tick: input.observed_tick,
        valid_until_tick: input.valid_until_tick,
        resource_ref: input.resource_ref.to_string(),
        evidence_refs: sorted_refs(input.evidence_refs.clone()),
        non_claims: REQUIRED_OBSERVABILITY_NON_CLAIMS.to_vec(),
    }
}

fn health_projection_ref(input: &HealthProjectionInput<'_>, generation: u64, state: &str) -> String {
    let identity = format!(
        "{}\0{}\0{}\0{}\0{}",
        input.source_ref, generation, input.observed_tick, input.valid_until_tick, state
    );
    crate::preserves_rail::content_ref_from_bytes(identity.as_bytes())
}

fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
    refs.sort();
    refs.dedup();
    refs
}
