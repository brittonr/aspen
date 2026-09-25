use super::*;
use crate::system_extension::SystemExtensionExecutor;

pub const FABRIC_DURABLE_LOG_PORT_ID: &str = "molten.fabric.durability.log";
pub const FABRIC_ORDERED_STORE_PORT_ID: &str = "molten.fabric.durability.ordered-store";
pub const FABRIC_SNAPSHOT_PORT_ID: &str = "molten.fabric.durability.snapshot";
pub const FABRIC_EFFECT_TRANSACTION_PORT_ID: &str = "molten.fabric.durability.effect-transaction";
pub const FABRIC_DURABILITY_PORT_VERSION: &str = "v1";

const DURABILITY_PROFILE_RECORD: &str = "fabric-durability-profile-v1";
const DURABILITY_TRANSITION_RECORD: &str = "fabric-durability-transition-v1";
const DURABILITY_RECOVERY_RECORD: &str = "fabric-durability-recovery-v1";
const DURABILITY_STATUS_RECORD: &str = "fabric-durability-status-v1";
const DURABILITY_PORT_COUNT: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDurableProfile {
    pub profile: DurableStateProfile,
    pub profile_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDurableTransition {
    pub transition_ref: String,
    pub profile_ref: String,
    pub namespace_id: String,
    pub generation: u64,
    pub outcome: MutationOutcome,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRecoveryDecision {
    pub recovery_ref: String,
    pub decision: RecoveryDecision,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableStatusReadback {
    pub profile_ref: String,
    pub adapter_kind: DurableAdapterKind,
    pub namespace_id: String,
    pub generation: u64,
    pub durable_log_records: u64,
    pub buffered_log_records: u64,
    pub ordered_entries: u64,
    pub snapshots: u64,
    pub unresolved_effects: u64,
    pub buffered_bytes: u64,
    pub durable_bytes: u64,
    pub non_claims: Vec<DurabilityNonClaim>,
    pub status_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.modularity.fabric_boundary.compatibility]
// r[impl molten.fabric_durability.port_contracts]
// r[impl molten.fabric_durability.non_claims]
pub fn canonical_durable_profile(profile: &DurableStateProfile) -> crate::error::Result<CanonicalDurableProfile> {
    validate_durable_profile(profile).map_err(|issues| validation_error("durability profile", &issues))?;
    let value = durable_profile_value(profile);
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalDurableProfile {
        profile: profile.clone(),
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_durability.port_contracts]
pub fn fabric_durability_port_descriptors(
    profile: &CanonicalDurableProfile,
) -> Vec<crate::fabric::FabricPortDescriptor> {
    let (determinism, replay) = match profile.profile.adapter_kind {
        DurableAdapterKind::LiveRedb => {
            (crate::fabric::DeterminismClass::ExternalEffect, crate::fabric::ReplayClass::RecordedEffectRequired)
        }
        DurableAdapterKind::DeterministicSimulation => (
            crate::fabric::DeterminismClass::DeterministicWithRecordedInputs,
            crate::fabric::ReplayClass::Recompute,
        ),
    };
    let definitions = [
        (
            FABRIC_DURABLE_LOG_PORT_ID,
            &[
                "append",
                "batch-append",
                "read",
                "scan",
                "tail",
                "flush",
                "truncate",
                "retain",
            ][..],
        ),
        (FABRIC_ORDERED_STORE_PORT_ID, &["get", "scan", "put", "delete", "compare-write", "atomic-batch"][..]),
        (FABRIC_SNAPSHOT_PORT_ID, &["create", "inspect", "inventory", "restore", "quarantine"][..]),
        (
            FABRIC_EFFECT_TRANSACTION_PORT_ID,
            &["reserve", "commit", "abort", "inspect", "expire", "reconcile"][..],
        ),
    ];
    let mut descriptors = Vec::with_capacity(DURABILITY_PORT_COUNT);
    for (port_id, operations) in definitions {
        descriptors.push(crate::fabric::FabricPortDescriptor {
            schema: crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port_id.to_string(),
            version: FABRIC_DURABILITY_PORT_VERSION.to_string(),
            class: crate::fabric::FabricPortClass::DurableState,
            operation_classes: operations.iter().map(|operation| (*operation).to_string()).collect(),
            input_schema_refs: vec![DURABLE_STATE_OPERATION_SCHEMA.to_string()],
            output_schema_refs: vec![DURABLE_STATE_OUTCOME_SCHEMA.to_string()],
            authority_requirements: vec![crate::fabric::FabricAuthority::DurableState],
            resource_requirements: vec![
                crate::fabric::FabricResource::StorageBytes,
                crate::fabric::FabricResource::QueueDepth,
            ],
            determinism,
            replay,
            implementation_profile: profile.profile.profile_id.clone(),
            conformance_refs: vec![profile.profile_ref.clone()],
            non_claims: crate::fabric::REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        });
    }
    descriptors
}

// r[impl molten.modularity.fabric_boundary.compatibility]
// r[impl molten.fabric_durability.evidence]
// r[impl molten.fabric_durability.uncertain_outcomes]
pub fn canonical_durable_transition(
    profile: &CanonicalDurableProfile,
    transition: &DurableTransition,
) -> crate::error::Result<CanonicalDurableTransition> {
    validate_namespace_descriptor(&profile.profile, &transition.next.descriptor)
        .map_err(|issues| validation_error("durability transition state", &issues))?;
    let value = crate::preserves_rail::record(DURABILITY_TRANSITION_RECORD, vec![
        crate::preserves_rail::string(DURABLE_STATE_OUTCOME_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("namespace-id", crate::preserves_rail::string(&transition.next.descriptor.namespace_id)),
        field("generation", crate::preserves_rail::u64_value(transition.next.descriptor.generation)),
        field("operation", crate::preserves_rail::string(&transition.operation)),
        field("outcome", crate::preserves_rail::string(transition.outcome.as_str())),
        field("affected-items", crate::preserves_rail::u64_value(transition.affected_items)),
        field("affected-bytes", crate::preserves_rail::u64_value(transition.affected_bytes)),
        field("retry-safe", crate::preserves_rail::bool_value(transition.retry_safe)),
        field("reconciliation-required", crate::preserves_rail::bool_value(transition.reconciliation_required)),
        field("non-claims", strings_value(profile.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "pure-transition-admitted",
            "generation-fenced",
            "outcome-explicit",
            "backend-handles-excluded",
            "local-durability-only",
        ]),
    ]);
    let transition_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalDurableTransition {
        transition_ref,
        profile_ref: profile.profile_ref.clone(),
        namespace_id: transition.next.descriptor.namespace_id.clone(),
        generation: transition.next.descriptor.generation,
        outcome: transition.outcome,
        value,
    })
}

// r[impl molten.fabric_durability.snapshot_recovery]
// r[impl molten.fabric_durability.evidence]
pub fn canonical_recovery_decision(
    profile: &CanonicalDurableProfile,
    state: &DurableState,
    decision: RecoveryDecision,
) -> crate::error::Result<CanonicalRecoveryDecision> {
    validate_namespace_descriptor(&profile.profile, &state.descriptor)
        .map_err(|issues| validation_error("durability recovery state", &issues))?;
    let diagnostics = decision.diagnostics.iter().map(|issue| format!("{issue:?}")).collect::<Vec<_>>();
    let value = crate::preserves_rail::record(DURABILITY_RECOVERY_RECORD, vec![
        crate::preserves_rail::string(DURABLE_STATE_RECOVERY_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("namespace-id", crate::preserves_rail::string(&state.descriptor.namespace_id)),
        field("generation", crate::preserves_rail::u64_value(state.descriptor.generation)),
        field("disposition", crate::preserves_rail::string(decision.disposition.as_str())),
        field("durable-log-tail", optional_u64(decision.durable_log_tail)),
        field("snapshot-count", crate::preserves_rail::u64_value(decision.snapshot_count)),
        field("unresolved-effect-count", crate::preserves_rail::u64_value(decision.unresolved_effect_count)),
        field("diagnostics", strings_value(diagnostics.iter().map(String::as_str))),
        checks(&[
            "inventory-explicit",
            "schema-and-generation-checked",
            "uncertainty-not-replayed",
            "repair-or-quarantine-separately-authorized",
        ]),
    ]);
    let recovery_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalRecoveryDecision {
        recovery_ref,
        decision,
        value,
    })
}

// r[impl molten.fabric_durability.evidence]
// r[impl molten.fabric_durability.non_claims]
pub fn durable_status_readback(
    profile: &CanonicalDurableProfile,
    state: &DurableState,
) -> crate::error::Result<DurableStatusReadback> {
    validate_namespace_descriptor(&profile.profile, &state.descriptor)
        .map_err(|issues| validation_error("durability status state", &issues))?;
    let durable_log_records = count(state.durable_log.len())?;
    let buffered_log_records = count(state.buffered_log.len())?;
    let ordered_entries = count(state.ordered.len())?;
    let snapshots = count(state.snapshots.len())?;
    let unresolved_effects = count(
        state
            .effects
            .values()
            .filter(|effect| {
                matches!(effect.phase, EffectTransactionPhase::Reserved | EffectTransactionPhase::Uncertain)
            })
            .count(),
    )?;
    let value = crate::preserves_rail::record(DURABILITY_STATUS_RECORD, vec![
        crate::preserves_rail::string(DURABLE_STATE_NAMESPACE_SCHEMA),
        field("profile-ref", crate::preserves_rail::string(&profile.profile_ref)),
        field("adapter-kind", crate::preserves_rail::string(profile.profile.adapter_kind.as_str())),
        field("namespace-id", crate::preserves_rail::string(&state.descriptor.namespace_id)),
        field("generation", crate::preserves_rail::u64_value(state.descriptor.generation)),
        field("durable-log-records", crate::preserves_rail::u64_value(durable_log_records)),
        field("buffered-log-records", crate::preserves_rail::u64_value(buffered_log_records)),
        field("ordered-entries", crate::preserves_rail::u64_value(ordered_entries)),
        field("snapshots", crate::preserves_rail::u64_value(snapshots)),
        field("unresolved-effects", crate::preserves_rail::u64_value(unresolved_effects)),
        field("buffered-bytes", crate::preserves_rail::u64_value(state.buffered_bytes)),
        field("durable-bytes", crate::preserves_rail::u64_value(state.durable_bytes)),
        field("non-claims", strings_value(profile.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "bounded-aggregate-readback",
            "payloads-and-keys-excluded",
            "backend-handles-excluded",
            "local-durability-only",
        ]),
    ]);
    let status_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(DurableStatusReadback {
        profile_ref: profile.profile_ref.clone(),
        adapter_kind: profile.profile.adapter_kind,
        namespace_id: state.descriptor.namespace_id.clone(),
        generation: state.descriptor.generation,
        durable_log_records,
        buffered_log_records,
        ordered_entries,
        snapshots,
        unresolved_effects,
        buffered_bytes: state.buffered_bytes,
        durable_bytes: state.durable_bytes,
        non_claims: profile.profile.non_claims.clone(),
        status_ref,
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDurabilityContext {
    service_id: String,
    generation: u64,
    profile_id: String,
    max_operation_bytes: u64,
    bound_ports: Vec<String>,
}
