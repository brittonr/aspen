
// The reference protocol oracle profile and the end-of-run evaluation.

const REFERENCE_PROTOCOL_ORACLE_PROFILE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/protocol-oracle/generated/profile.json"));

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleProfileDocument {
    schema: String,
    profile_id: String,
    projection_schema: String,
    oracle: OracleIdentityDocument,
    oracle_ref: String,
    services: Vec<OracleServiceDocument>,
    participants: Vec<OracleParticipantDocument>,
    completeness: String,
    liveness: OracleLivenessDocument,
    novelty_fields: Vec<String>,
    counters: Vec<OracleCounterDocument>,
    bounds: OracleBoundsDocument,
    non_claims: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleBoundsDocument {
    max_records: u32,
    max_state_bytes: u32,
    max_total_bytes: u64,
    max_participants: u32,
    max_counters: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleIdentityDocument {
    oracle_id: String,
    version: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleServiceDocument {
    service: String,
    logical_position: String,
    safety: Vec<String>,
    physical_layout: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleParticipantDocument {
    participant: String,
    service: String,
    is_required: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleLivenessDocument {
    requires_readiness: bool,
    requires_membership: bool,
    disruptive_faults_block: bool,
    requires_durability: bool,
    virtual_progress_horizon: u64,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleCounterDocument {
    name: String,
    resets_on_generation: bool,
}

fn profile_term<T: Copy>(value: &str, terms: &[(&str, T)]) -> crate::error::Result<T> {
    terms.iter().find(|(name, _)| *name == value).map(|(_, term)| *term).ok_or_else(|| {
        crate::error::Failure::invalid_harness(format!("unsupported protocol oracle profile term {value}"))
    })
}

fn parse_profile_services(
    documents: Vec<OracleServiceDocument>,
) -> crate::error::Result<Vec<molten_core::protocol_oracle::ServiceProfile>> {
    let levels = [("local", molten_core::protocol_oracle::SafetyLevel::Local), ("pairwise", molten_core::protocol_oracle::SafetyLevel::Pairwise), ("cohort", molten_core::protocol_oracle::SafetyLevel::Cohort), ("durability", molten_core::protocol_oracle::SafetyLevel::Durability)];
    let mut services = Vec::with_capacity(documents.len());
    for service in documents {
        services.push(molten_core::protocol_oracle::ServiceProfile {
            logical_position: profile_term(&service.logical_position, &[
                ("version", molten_core::protocol_oracle::LogicalPositionKind::Version),
                ("next-offset", molten_core::protocol_oracle::LogicalPositionKind::NextOffset),
                ("transition-count", molten_core::protocol_oracle::LogicalPositionKind::TransitionCount),
            ])?,
            safety: service.safety.iter().map(|level| profile_term(level, &levels)).collect::<crate::error::Result<_>>()?,
            physical_layout: profile_term(&service.physical_layout, &[
                ("semantic-only", molten_core::protocol_oracle::PhysicalLayout::SemanticOnly),
                ("canonical-physical", molten_core::protocol_oracle::PhysicalLayout::CanonicalPhysical),
            ])?,
            service: service.service,
        });
    }
    Ok(services)
}

fn parse_novelty_fields(fields: &[String]) -> crate::error::Result<Vec<molten_core::protocol_oracle::NoveltyField>> {
    let terms = [
        ("service", molten_core::protocol_oracle::NoveltyField::Service),
        ("participant", molten_core::protocol_oracle::NoveltyField::Participant),
        ("transition-class", molten_core::protocol_oracle::NoveltyField::TransitionClass),
        ("logical-position", molten_core::protocol_oracle::NoveltyField::LogicalPosition),
        ("state", molten_core::protocol_oracle::NoveltyField::State),
        ("durability", molten_core::protocol_oracle::NoveltyField::Durability),
        ("progress", molten_core::protocol_oracle::NoveltyField::Progress),
    ];
    fields.iter().map(|field| profile_term(field, &terms)).collect()
}

/// Parse and validate a generated protocol oracle profile.
// r[impl molten.fabric_simulation.protocol_projection]
pub fn parse_protocol_oracle_profile(json: &str) -> crate::error::Result<molten_core::protocol_oracle::ProtocolOracleProfile> {
    use molten_core::protocol_oracle::*;
    let document: OracleProfileDocument = serde_json::from_str(json)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("protocol oracle profile JSON: {error}")))?;
    if document.schema != ORACLE_PROFILE_SCHEMA || document.projection_schema != PROJECTION_SCHEMA {
        return Err(crate::error::Failure::invalid_harness("protocol oracle profile schema mismatch"));
    }
    let services = parse_profile_services(document.services)?;
    let liveness = document.liveness;
    let profile = ProtocolOracleProfile {
        profile_id: document.profile_id,
        oracle: OracleIdentity {
            oracle_id: document.oracle.oracle_id,
            version: document.oracle.version,
        },
        oracle_ref: document.oracle_ref,
        services,
        participants: document
            .participants
            .into_iter()
            .map(|participant| ParticipantProfile {
                participant: participant.participant,
                service: participant.service,
                is_required: participant.is_required,
            })
            .collect(),
        completeness: profile_term(&document.completeness, &[
            ("lossless", CompletenessMode::Lossless),
            ("sampled", CompletenessMode::Sampled),
        ])?,
        liveness: LivenessPreconditions {
            requires_readiness: liveness.requires_readiness,
            requires_membership: liveness.requires_membership,
            disruptive_faults_block: liveness.disruptive_faults_block,
            requires_durability: liveness.requires_durability,
            virtual_progress_horizon: liveness.virtual_progress_horizon,
        },
        novelty_fields: parse_novelty_fields(&document.novelty_fields)?,
        counters: document
            .counters
            .into_iter()
            .map(|counter| CounterSchema {
                name: counter.name,
                resets_on_generation: counter.resets_on_generation,
            })
            .collect(),
        bounds: ProjectionBounds {
            max_records: document.bounds.max_records,
            max_state_bytes: document.bounds.max_state_bytes,
            max_total_bytes: document.bounds.max_total_bytes,
            max_participants: document.bounds.max_participants,
            max_counters: document.bounds.max_counters,
        },
        non_claims: document.non_claims,
    };
    let issues = validate_oracle_profile(&profile);
    if !issues.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!("protocol oracle profile denied: {issues:?}")));
    }
    Ok(profile)
}

/// The reviewed reference protocol oracle profile.
pub fn reference_protocol_oracle_profile() -> crate::error::Result<molten_core::protocol_oracle::ProtocolOracleProfile> {
    parse_protocol_oracle_profile(REFERENCE_PROTOCOL_ORACLE_PROFILE)
}

/// A run's protocol oracle evidence, its canonical ref, and its operator status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolRunEvidence {
    pub evidence: molten_core::protocol_oracle::ProtocolOracleEvidence,
    pub evidence_ref: String,
    pub status: ProtocolStatus,
}

/// What the end-of-run evaluation binds besides the trace.
struct ProtocolRunContext<'a> {
    seed: u64,
    scheduler: &'a SimulationSchedulerState,
    choice_records: &'a [SchedulerChoiceRecord],
    is_disruptive_fault_fired: bool,
}

/// Evaluate a run's protocol trace with the independent reference oracle.
// r[impl molten.fabric_simulation.oracle_independence]
// r[impl molten.fabric_simulation.protocol_oracle_evidence]
fn evaluate_protocol_trace(
    prepared: &PreparedReferenceWorld,
    trace: ProtocolTrace,
    context: &ProtocolRunContext<'_>,
) -> crate::error::Result<ProtocolRunEvidence> {
    use molten_core::protocol_oracle::*;
    let mut profile = reference_protocol_oracle_profile()?;
    profile.participants.retain(|participant| prepared.hosts.contains_key(&participant.participant));
    let counters = admit_counters(&profile.counters, &trace.counters)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("protocol counters denied: {issues:?}")))?;
    let liveness_facts = trace
        .tracks
        .iter()
        .map(|(participant, track)| ParticipantStabilization {
            participant: participant.clone(),
            is_ready: true,
            has_required_membership: true,
            is_disruptive_fault_active: context.is_disruptive_fault_fired,
            is_durability_satisfied: true,
            start_progress: 0,
            end_progress: Some(track.applied_transitions),
            observed_ticks: context.scheduler.virtual_tick,
        })
        .collect::<Vec<_>>();
    let cohort = assemble_cohort(&profile, trace.projections)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("protocol cohort denied: {issues:?}")))?;
    let manifest = &prepared.world.admitted.manifest;
    let choice_material = context.choice_records.iter().map(|record| record.selected.choice_id.as_str()).collect::<Vec<_>>();
    let evidence = ProtocolOracleEvidence {
        run: ProtocolRunIdentity {
            world_ref: prepared.world.world_ref.clone(),
            runtime_ref: manifest.runtime_ref.clone(),
            scheduler_ref: blake3_ref(choice_material.join("\n").as_bytes()),
            workload_ref: manifest.workload_ref.clone(),
            fault_ref: manifest.fault_plan_ref.clone(),
            replay_ref: blake3_ref(format!("{}:{}", prepared.world.world_ref, context.seed).as_bytes()),
        },
        profile_id: profile.profile_id.clone(),
        projection_schema: PROJECTION_SCHEMA.to_string(),
        oracle_ref: profile.oracle_ref.clone(),
        participants: profile.participants.iter().map(|participant| participant.participant.clone()).collect(),
        projection_refs: cohort.projections.iter().map(|admitted| admitted.projection_ref.clone()).collect(),
        gaps: cohort.gaps.clone(),
        safety: evaluate_safety(&profile, &cohort, &ReferenceProtocolSemantics),
        liveness: evaluate_cohort_liveness(&profile.liveness, &liveness_facts),
        novelty_refs: cohort_novelty(&profile.novelty_fields, &cohort),
        counters,
        non_claims: profile.non_claims.clone(),
    };
    let evidence_ref = evidence_ref(&evidence);
    let status = protocol_status(&evidence, &evidence_ref)?;
    Ok(ProtocolRunEvidence {
        evidence,
        evidence_ref,
        status,
    })
}
