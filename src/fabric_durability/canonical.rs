use preserves::IOValue;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricPortKey;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::system_extension::SystemExtensionExecutor;
use crate::system_extension::SystemExtensionHost;

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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDurableTransition {
    pub transition_ref: String,
    pub profile_ref: String,
    pub namespace_id: String,
    pub generation: u64,
    pub outcome: MutationOutcome,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalRecoveryDecision {
    pub recovery_ref: String,
    pub decision: RecoveryDecision,
    pub value: IOValue,
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
    pub value: IOValue,
}

// r[impl molten.fabric_durability.port_contracts]
// r[impl molten.fabric_durability.non_claims]
pub fn canonical_durable_profile(profile: &DurableStateProfile) -> Result<CanonicalDurableProfile> {
    validate_durable_profile(profile).map_err(|issues| validation_error("durability profile", &issues))?;
    let value = durable_profile_value(profile);
    let profile_ref = canonical_hash(&value)?;
    Ok(CanonicalDurableProfile {
        profile: profile.clone(),
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_durability.port_contracts]
pub fn fabric_durability_port_descriptors(profile: &CanonicalDurableProfile) -> Vec<FabricPortDescriptor> {
    let (determinism, replay) = match profile.profile.adapter_kind {
        DurableAdapterKind::LiveRedb => (DeterminismClass::ExternalEffect, ReplayClass::RecordedEffectRequired),
        DurableAdapterKind::DeterministicSimulation => {
            (DeterminismClass::DeterministicWithRecordedInputs, ReplayClass::Recompute)
        }
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
        descriptors.push(FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: port_id.to_string(),
            version: FABRIC_DURABILITY_PORT_VERSION.to_string(),
            class: FabricPortClass::DurableState,
            operation_classes: operations.iter().map(|operation| (*operation).to_string()).collect(),
            input_schema_refs: vec![DURABLE_STATE_OPERATION_SCHEMA.to_string()],
            output_schema_refs: vec![DURABLE_STATE_OUTCOME_SCHEMA.to_string()],
            authority_requirements: vec![FabricAuthority::DurableState],
            resource_requirements: vec![FabricResource::StorageBytes, FabricResource::QueueDepth],
            determinism,
            replay,
            implementation_profile: profile.profile.profile_id.clone(),
            conformance_refs: vec![profile.profile_ref.clone()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        });
    }
    descriptors
}

// r[impl molten.fabric_durability.evidence]
// r[impl molten.fabric_durability.uncertain_outcomes]
pub fn canonical_durable_transition(
    profile: &CanonicalDurableProfile,
    transition: &DurableTransition,
) -> Result<CanonicalDurableTransition> {
    validate_namespace_descriptor(&profile.profile, &transition.next.descriptor)
        .map_err(|issues| validation_error("durability transition state", &issues))?;
    let value = record(DURABILITY_TRANSITION_RECORD, vec![
        string(DURABLE_STATE_OUTCOME_SCHEMA),
        field("profile-ref", string(&profile.profile_ref)),
        field("namespace-id", string(&transition.next.descriptor.namespace_id)),
        field("generation", u64_value(transition.next.descriptor.generation)),
        field("operation", string(&transition.operation)),
        field("outcome", string(transition.outcome.as_str())),
        field("affected-items", u64_value(transition.affected_items)),
        field("affected-bytes", u64_value(transition.affected_bytes)),
        field("retry-safe", bool_value(transition.retry_safe)),
        field("reconciliation-required", bool_value(transition.reconciliation_required)),
        field("non-claims", strings_value(profile.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "pure-transition-admitted",
            "generation-fenced",
            "outcome-explicit",
            "backend-handles-excluded",
            "local-durability-only",
        ]),
    ]);
    let transition_ref = canonical_hash(&value)?;
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
) -> Result<CanonicalRecoveryDecision> {
    validate_namespace_descriptor(&profile.profile, &state.descriptor)
        .map_err(|issues| validation_error("durability recovery state", &issues))?;
    let diagnostics = decision.diagnostics.iter().map(|issue| format!("{issue:?}")).collect::<Vec<_>>();
    let value = record(DURABILITY_RECOVERY_RECORD, vec![
        string(DURABLE_STATE_RECOVERY_SCHEMA),
        field("profile-ref", string(&profile.profile_ref)),
        field("namespace-id", string(&state.descriptor.namespace_id)),
        field("generation", u64_value(state.descriptor.generation)),
        field("disposition", string(decision.disposition.as_str())),
        field("durable-log-tail", optional_u64(decision.durable_log_tail)),
        field("snapshot-count", u64_value(decision.snapshot_count)),
        field("unresolved-effect-count", u64_value(decision.unresolved_effect_count)),
        field("diagnostics", strings_value(diagnostics.iter().map(String::as_str))),
        checks(&[
            "inventory-explicit",
            "schema-and-generation-checked",
            "uncertainty-not-replayed",
            "repair-or-quarantine-separately-authorized",
        ]),
    ]);
    let recovery_ref = canonical_hash(&value)?;
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
) -> Result<DurableStatusReadback> {
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
    let value = record(DURABILITY_STATUS_RECORD, vec![
        string(DURABLE_STATE_NAMESPACE_SCHEMA),
        field("profile-ref", string(&profile.profile_ref)),
        field("adapter-kind", string(profile.profile.adapter_kind.as_str())),
        field("namespace-id", string(&state.descriptor.namespace_id)),
        field("generation", u64_value(state.descriptor.generation)),
        field("durable-log-records", u64_value(durable_log_records)),
        field("buffered-log-records", u64_value(buffered_log_records)),
        field("ordered-entries", u64_value(ordered_entries)),
        field("snapshots", u64_value(snapshots)),
        field("unresolved-effects", u64_value(unresolved_effects)),
        field("buffered-bytes", u64_value(state.buffered_bytes)),
        field("durable-bytes", u64_value(state.durable_bytes)),
        field("non-claims", strings_value(profile.profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "bounded-aggregate-readback",
            "payloads-and-keys-excluded",
            "backend-handles-excluded",
            "local-durability-only",
        ]),
    ]);
    let status_ref = canonical_hash(&value)?;
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

impl ExtensionDurabilityContext {
    pub fn from_host<E: SystemExtensionExecutor>(
        host: &SystemExtensionHost<E>,
        profile: &CanonicalDurableProfile,
    ) -> Result<Self> {
        let mut bound_ports = Vec::new();
        for port_id in durability_port_ids() {
            let key = FabricPortKey {
                port_id: port_id.to_string(),
                version: FABRIC_DURABILITY_PORT_VERSION.to_string(),
            };
            if let Some(binding) = host.manifest().binding_for(&key) {
                if binding.binding.implementation_profile != profile.profile.profile_id {
                    return Err(MoltenError::invalid_harness(format!(
                        "system-extension durability profile {} does not match {}",
                        binding.binding.implementation_profile, profile.profile.profile_id
                    )));
                }
                bound_ports.push(port_id.to_string());
            }
        }
        if bound_ports.is_empty() {
            return Err(MoltenError::invalid_harness(
                "system extension has no admitted durable-state fabric port binding",
            ));
        }
        Ok(Self {
            service_id: host.manifest().manifest().service_id.clone(),
            generation: host.state().generation,
            profile_id: profile.profile.profile_id.clone(),
            max_operation_bytes: profile.profile.max_operation_bytes,
            bound_ports,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_snapshot(
        service_id: &str,
        generation: u64,
        profile: &CanonicalDurableProfile,
        bound_ports: Vec<String>,
    ) -> Self {
        Self {
            service_id: service_id.to_string(),
            generation,
            profile_id: profile.profile.profile_id.clone(),
            max_operation_bytes: profile.profile.max_operation_bytes,
            bound_ports,
        }
    }

    pub fn admit_operation(
        &self,
        profile: &CanonicalDurableProfile,
        port_id: &str,
        service_id: &str,
        generation: u64,
        operation_bytes: u64,
    ) -> Result<()> {
        if self.profile_id != profile.profile.profile_id {
            return Err(MoltenError::invalid_harness("durability profile substitution denied"));
        }
        if !self.bound_ports.iter().any(|bound| bound == port_id) {
            return Err(MoltenError::invalid_harness(format!(
                "durability port {port_id} is not bound to the system extension"
            )));
        }
        if self.service_id != service_id {
            return Err(MoltenError::invalid_harness("durability service identity mismatch"));
        }
        if self.generation != generation {
            return Err(MoltenError::invalid_harness("durability operation uses a stale service generation"));
        }
        if operation_bytes > self.max_operation_bytes {
            return Err(MoltenError::invalid_harness(format!(
                "durability operation bytes {operation_bytes} exceed {}",
                self.max_operation_bytes
            )));
        }
        Ok(())
    }
}

fn durable_profile_value(profile: &DurableStateProfile) -> IOValue {
    record(DURABILITY_PROFILE_RECORD, vec![
        string(DURABLE_STATE_PROFILE_SCHEMA),
        field("profile-id", string(&profile.profile_id)),
        field("declared-profile-ref", string(&profile.profile_ref)),
        field("adapter-kind", string(profile.adapter_kind.as_str())),
        field("durability-levels", strings_value(profile.supported_levels.iter().map(|level| level.as_str()))),
        field("max-namespaces", u64_value(profile.max_namespaces)),
        field("max-log-records", u64_value(profile.max_log_records)),
        field("max-ordered-entries", u64_value(profile.max_ordered_entries)),
        field("max-operation-bytes", u64_value(profile.max_operation_bytes)),
        field("max-namespace-bytes", u64_value(profile.max_namespace_bytes)),
        field("max-batch-operations", u64_value(profile.max_batch_operations)),
        field("max-snapshots", u64_value(profile.max_snapshots)),
        field("max-effect-transactions", u64_value(profile.max_effect_transactions)),
        field("non-claims", strings_value(profile.non_claims.iter().map(|claim| claim.as_str()))),
        checks(&[
            "canonical-profile",
            "atomicity-domain-explicit",
            "durability-boundary-explicit",
            "local-only-non-claims-complete",
        ]),
    ])
}

fn durability_port_ids() -> [&'static str; DURABILITY_PORT_COUNT] {
    [
        FABRIC_DURABLE_LOG_PORT_ID,
        FABRIC_ORDERED_STORE_PORT_ID,
        FABRIC_SNAPSHOT_PORT_ID,
        FABRIC_EFFECT_TRANSACTION_PORT_ID,
    ]
}

fn field(name: &str, value: IOValue) -> IOValue {
    record("field", vec![string(name), value])
}

fn strings_value<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    sequence(values.map(string).collect())
}

fn checks(values: &[&str]) -> IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn optional_u64(value: Option<u64>) -> IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn count(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| MoltenError::invalid_harness("durability collection count overflow"))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}
