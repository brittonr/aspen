use std::collections::BTreeMap;

use molten_core::world_snapshot::*;

use super::CanonicalSnapshotArtifact;
use super::ChaosControlSnapshotDescriptorPort;
use super::CurrentSnapshotAdmissionPort;
use super::LogicalSnapshotRestorePort;
use super::OpaqueSnapshotRestorePort;
use super::SnapshotAdmissionObservation;
use super::SnapshotHostHandlePort;
use super::SnapshotMaterializationObservation;
use super::SnapshotMaterializationPort;
use super::SnapshotReceiptPort;
use super::SnapshotStepObservation;
use super::canonical_snapshot_compatibility;
use super::canonical_snapshot_descriptor;
use super::canonical_snapshot_receipt;
use super::canonical_snapshot_restore_plan;
use crate::error::MoltenError;
use crate::error::Result;

pub struct LogicalSnapshotPorts<'a, M, A, H, R, P> {
    pub materialization: &'a mut M,
    pub admission: &'a mut A,
    pub handles: &'a mut H,
    pub runtime: &'a mut R,
    pub receipts: &'a mut P,
}

#[derive(Debug, Clone)]
pub struct LogicalSnapshotRestoreOutcome {
    pub descriptor: CanonicalSnapshotArtifact,
    pub compatibility: CanonicalSnapshotArtifact,
    pub plan: CanonicalSnapshotArtifact,
    pub receipt: CanonicalSnapshotArtifact,
    pub observations: Vec<SnapshotStepObservation>,
}

pub struct OpaqueSnapshotPorts<'a, M, C, A, H, R, P> {
    pub materialization: &'a mut M,
    pub chaoscontrol: &'a mut C,
    pub admission: &'a mut A,
    pub handles: &'a mut H,
    pub runtime: &'a mut R,
    pub receipts: &'a mut P,
}

#[derive(Debug, Clone)]
pub struct OpaqueSnapshotRestoreOutcome {
    pub descriptor: CanonicalSnapshotArtifact,
    pub compatibility: CanonicalSnapshotArtifact,
    pub plan: CanonicalSnapshotArtifact,
    pub receipt: CanonicalSnapshotArtifact,
    pub observations: Vec<SnapshotStepObservation>,
}

pub fn restore_logical_snapshot<M, A, H, R, P>(
    descriptor: &SnapshotDescriptor,
    destination: &SnapshotCohort,
    ports: LogicalSnapshotPorts<'_, M, A, H, R, P>,
) -> Result<LogicalSnapshotRestoreOutcome>
where
    M: SnapshotMaterializationPort,
    A: CurrentSnapshotAdmissionPort,
    H: SnapshotHostHandlePort,
    R: LogicalSnapshotRestorePort,
    P: SnapshotReceiptPort,
{
    if descriptor.class != SnapshotClass::Logical {
        return Err(MoltenError::invalid_harness("logical snapshot restore rejects non-logical profiles"));
    }
    let canonical_descriptor = canonical_snapshot_descriptor(descriptor)?;
    let compatibility_report = validate_snapshot(descriptor, destination);
    let canonical_compatibility = canonical_snapshot_compatibility(&compatibility_report)?;
    let initial_admission =
        ports.admission.observe_current(descriptor, &canonical_descriptor.artifact_ref, destination)?;
    validate_admission(&initial_admission, descriptor, destination, &canonical_descriptor.artifact_ref, None)?;
    let restore_plan = plan_restore(descriptor, destination, true)
        .map_err(|report| MoltenError::invalid_harness(format!("snapshot restore denied: {:?}", report.issues)))?;
    let canonical_plan = canonical_snapshot_restore_plan(&restore_plan)?;
    let materializations = materialize_complete_inventory(descriptor, ports.materialization)?;
    let mut observations = Vec::with_capacity(restore_plan.steps.len());
    let mut final_admission = initial_admission;

    for step in &restore_plan.steps {
        match step {
            SnapshotRestoreStep::VerifyClosure => observations.push(SnapshotStepObservation {
                step: *step,
                observation_ref: canonical_descriptor.artifact_ref.clone(),
            }),
            SnapshotRestoreStep::VerifyCohort => observations.push(SnapshotStepObservation {
                step: *step,
                observation_ref: canonical_compatibility.artifact_ref.clone(),
            }),
            SnapshotRestoreStep::MaterializeArtifacts => {
                let observation = materialization_for(&materializations, SnapshotComponentKind::Artifact)?;
                observations.push(SnapshotStepObservation {
                    step: *step,
                    observation_ref: observation.observation_ref.clone(),
                });
            }
            SnapshotRestoreStep::RecreateHostHandles => {
                let observation_ref = ports.handles.recreate_handles(&canonical_descriptor.artifact_ref)?;
                validate_ref(&observation_ref, "host-handle recreation observation")?;
                observations.push(SnapshotStepObservation {
                    step: *step,
                    observation_ref,
                });
            }
            SnapshotRestoreStep::RecheckCurrentAdmission => {
                let observed =
                    ports.admission.observe_current(descriptor, &canonical_descriptor.artifact_ref, destination)?;
                validate_admission(
                    &observed,
                    descriptor,
                    destination,
                    &canonical_descriptor.artifact_ref,
                    Some(final_admission.generation),
                )?;
                observations.push(SnapshotStepObservation {
                    step: *step,
                    observation_ref: observed.admission_ref.clone(),
                });
                final_admission = observed;
            }
            SnapshotRestoreStep::ActivateRuntime => {
                let observation_ref = ports.runtime.activate(&canonical_descriptor.artifact_ref)?;
                validate_ref(&observation_ref, "snapshot activation observation")?;
                observations.push(SnapshotStepObservation {
                    step: *step,
                    observation_ref,
                });
            }
            SnapshotRestoreStep::RestoreOpaqueMachine => {
                return Err(MoltenError::invalid_harness("logical snapshot plan contains an opaque restore step"));
            }
            state_step => {
                let kind = component_for_step(*state_step).ok_or_else(|| {
                    MoltenError::invalid_harness("logical snapshot plan contains an unmapped restore step")
                })?;
                let component = descriptor
                    .components
                    .iter()
                    .find(|component| component.kind == kind)
                    .ok_or_else(|| MoltenError::invalid_harness("logical snapshot component disappeared"))?;
                let materialization = materialization_for(&materializations, kind)?;
                let observation = ports.runtime.restore_component(*state_step, component, materialization)?;
                if observation.step != *state_step {
                    return Err(MoltenError::invalid_harness(
                        "logical snapshot adapter returned an observation for the wrong step",
                    ));
                }
                validate_ref(&observation.observation_ref, "logical restore observation")?;
                observations.push(observation);
            }
        }
    }

    let receipt_input = SnapshotReceipt {
        decision: SnapshotReceiptDecision::Restored,
        descriptor_ref: canonical_descriptor.artifact_ref.clone(),
        compatibility_ref: canonical_compatibility.artifact_ref.clone(),
        restore_plan_ref: Some(canonical_plan.artifact_ref.clone()),
        clone_plan_ref: None,
        current_admission_ref: Some(final_admission.admission_ref),
        issues: Vec::new(),
        non_claims: SNAPSHOT_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    let canonical_receipt = canonical_snapshot_receipt(&receipt_input)?;
    ports.receipts.publish_receipt(&canonical_receipt.artifact_ref, &canonical_receipt.bytes)?;
    Ok(LogicalSnapshotRestoreOutcome {
        descriptor: canonical_descriptor,
        compatibility: canonical_compatibility,
        plan: canonical_plan,
        receipt: canonical_receipt,
        observations,
    })
}

/// Restores one exact opaque snapshot through the admitted ChaosControl profile.
///
/// # Errors
///
/// Returns a bounded error for descriptor drift, unavailable components, stale
/// admission, wrong restore observations, handle failure, or activation failure.
// r[impl molten.world_snapshot.opaque]
// r[impl molten.world_snapshot.restore]
pub fn restore_opaque_snapshot<M, C, A, H, R, P>(
    descriptor: &SnapshotDescriptor,
    destination: &SnapshotCohort,
    ports: OpaqueSnapshotPorts<'_, M, C, A, H, R, P>,
) -> Result<OpaqueSnapshotRestoreOutcome>
where
    M: SnapshotMaterializationPort,
    C: ChaosControlSnapshotDescriptorPort,
    A: CurrentSnapshotAdmissionPort,
    H: SnapshotHostHandlePort,
    R: OpaqueSnapshotRestorePort,
    P: SnapshotReceiptPort,
{
    if descriptor.class != SnapshotClass::Opaque {
        return Err(MoltenError::invalid_harness("opaque snapshot restore rejects non-opaque profiles"));
    }
    let canonical_descriptor = canonical_snapshot_descriptor(descriptor)?;
    let compatibility_report = validate_snapshot(descriptor, destination);
    let canonical_compatibility = canonical_snapshot_compatibility(&compatibility_report)?;
    if compatibility_report.verdict != CompatibilityVerdict::Compatible {
        return Err(MoltenError::invalid_harness(format!(
            "opaque snapshot restore denied: {:?}",
            compatibility_report.issues
        )));
    }
    let chaos_observation = ports.chaoscontrol.observe_descriptor(descriptor)?;
    let machine_descriptor = descriptor
        .components
        .iter()
        .find(|component| component.kind == SnapshotComponentKind::MachineDescriptor)
        .ok_or_else(|| MoltenError::invalid_harness("opaque machine descriptor is missing"))?;
    if chaos_observation.descriptor_ref != machine_descriptor.identity
        || chaos_observation.cohort_ref != destination.cohort_ref.as_str()
        || !chaos_observation.available
        || !chaos_observation.identity_verified
    {
        return Err(MoltenError::invalid_harness("ChaosControl descriptor observation is unavailable or drifted"));
    }
    let initial_admission =
        ports.admission.observe_current(descriptor, &canonical_descriptor.artifact_ref, destination)?;
    validate_admission(&initial_admission, descriptor, destination, &canonical_descriptor.artifact_ref, None)?;
    let restore_plan = plan_restore(descriptor, destination, true).map_err(|report| {
        MoltenError::invalid_harness(format!("opaque snapshot restore denied: {:?}", report.issues))
    })?;
    let canonical_plan = canonical_snapshot_restore_plan(&restore_plan)?;
    let materializations = materialize_complete_inventory(descriptor, ports.materialization)?;
    let mut observations = vec![
        SnapshotStepObservation {
            step: SnapshotRestoreStep::VerifyClosure,
            observation_ref: canonical_descriptor.artifact_ref.clone(),
        },
        SnapshotStepObservation {
            step: SnapshotRestoreStep::VerifyCohort,
            observation_ref: canonical_compatibility.artifact_ref.clone(),
        },
        SnapshotStepObservation {
            step: SnapshotRestoreStep::MaterializeArtifacts,
            observation_ref: materialization_for(&materializations, SnapshotComponentKind::Artifact)?
                .observation_ref
                .clone(),
        },
    ];
    let restored = ports.runtime.restore_exact(descriptor, destination)?;
    if restored.is_empty()
        || restored.len() > MAX_SNAPSHOT_COMPONENTS
        || restored.iter().any(|observation| observation.step != SnapshotRestoreStep::RestoreOpaqueMachine)
    {
        return Err(MoltenError::invalid_harness(
            "ChaosControl restore observations are empty, overbound, or use the wrong step",
        ));
    }
    for observation in &restored {
        validate_ref(&observation.observation_ref, "ChaosControl opaque restore observation")?;
    }
    observations.extend(restored);
    let handle_ref = ports.handles.recreate_handles(&canonical_descriptor.artifact_ref)?;
    validate_ref(&handle_ref, "opaque host-handle recreation observation")?;
    observations.push(SnapshotStepObservation {
        step: SnapshotRestoreStep::RecreateHostHandles,
        observation_ref: handle_ref,
    });
    let final_admission =
        ports.admission.observe_current(descriptor, &canonical_descriptor.artifact_ref, destination)?;
    validate_admission(
        &final_admission,
        descriptor,
        destination,
        &canonical_descriptor.artifact_ref,
        Some(initial_admission.generation),
    )?;
    observations.push(SnapshotStepObservation {
        step: SnapshotRestoreStep::RecheckCurrentAdmission,
        observation_ref: final_admission.admission_ref.clone(),
    });
    let activation_ref = ports.runtime.activate(&canonical_descriptor.artifact_ref)?;
    validate_ref(&activation_ref, "opaque snapshot activation observation")?;
    observations.push(SnapshotStepObservation {
        step: SnapshotRestoreStep::ActivateRuntime,
        observation_ref: activation_ref,
    });
    let receipt_input = SnapshotReceipt {
        decision: SnapshotReceiptDecision::Restored,
        descriptor_ref: canonical_descriptor.artifact_ref.clone(),
        compatibility_ref: canonical_compatibility.artifact_ref.clone(),
        restore_plan_ref: Some(canonical_plan.artifact_ref.clone()),
        clone_plan_ref: None,
        current_admission_ref: Some(final_admission.admission_ref),
        issues: Vec::new(),
        non_claims: SNAPSHOT_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    let canonical_receipt = canonical_snapshot_receipt(&receipt_input)?;
    ports.receipts.publish_receipt(&canonical_receipt.artifact_ref, &canonical_receipt.bytes)?;
    Ok(OpaqueSnapshotRestoreOutcome {
        descriptor: canonical_descriptor,
        compatibility: canonical_compatibility,
        plan: canonical_plan,
        receipt: canonical_receipt,
        observations,
    })
}

fn materialize_complete_inventory<M: SnapshotMaterializationPort>(
    descriptor: &SnapshotDescriptor,
    port: &mut M,
) -> Result<BTreeMap<SnapshotComponentKind, SnapshotMaterializationObservation>> {
    let mut components = descriptor.components.iter().collect::<Vec<_>>();
    components.sort_by_key(|component| component.kind);
    let mut observations = BTreeMap::new();
    for component in components {
        let observation = port.observe_component(component)?;
        validate_ref(&observation.observation_ref, "snapshot materialization observation")?;
        if observation.component_identity != component.identity
            || !observation.available
            || !observation.identity_verified
        {
            return Err(MoltenError::invalid_harness(
                "snapshot component materialization is unavailable or unverified",
            ));
        }
        if observations.insert(component.kind, observation).is_some() {
            return Err(MoltenError::invalid_harness("snapshot component materialization was duplicated"));
        }
    }
    Ok(observations)
}

fn materialization_for(
    observations: &BTreeMap<SnapshotComponentKind, SnapshotMaterializationObservation>,
    kind: SnapshotComponentKind,
) -> Result<&SnapshotMaterializationObservation> {
    observations
        .get(&kind)
        .ok_or_else(|| MoltenError::invalid_harness("snapshot materialization observation is missing"))
}

fn validate_admission(
    observation: &SnapshotAdmissionObservation,
    descriptor: &SnapshotDescriptor,
    destination: &SnapshotCohort,
    descriptor_ref: &str,
    minimum_generation: Option<u64>,
) -> Result<()> {
    validate_ref(&observation.admission_ref, "snapshot current-admission observation")?;
    let stale = minimum_generation.is_some_and(|minimum| observation.generation < minimum);
    if !observation.allowed
        || observation.descriptor_ref != descriptor_ref
        || observation.profile_ref != descriptor.profile_ref.as_str()
        || observation.cohort_ref != destination.cohort_ref.as_str()
        || stale
    {
        return Err(MoltenError::invalid_harness("snapshot current admission denied, drifted, or became stale"));
    }
    Ok(())
}

fn validate_ref(reference: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|_| MoltenError::invalid_harness(format!("{field} is not a canonical content reference")))
}

const fn component_for_step(step: SnapshotRestoreStep) -> Option<SnapshotComponentKind> {
    match step {
        SnapshotRestoreStep::RestoreDurableState => Some(SnapshotComponentKind::DurableState),
        SnapshotRestoreStep::RestoreHistory => Some(SnapshotComponentKind::History),
        SnapshotRestoreStep::RestoreTasks => Some(SnapshotComponentKind::Tasks),
        SnapshotRestoreStep::RestoreScheduler => Some(SnapshotComponentKind::Scheduler),
        SnapshotRestoreStep::RestoreTime => Some(SnapshotComponentKind::Time),
        SnapshotRestoreStep::RestoreEntropy => Some(SnapshotComponentKind::Entropy),
        SnapshotRestoreStep::RestoreEffects => Some(SnapshotComponentKind::Effects),
        SnapshotRestoreStep::VerifyClosure
        | SnapshotRestoreStep::VerifyCohort
        | SnapshotRestoreStep::MaterializeArtifacts
        | SnapshotRestoreStep::RestoreOpaqueMachine
        | SnapshotRestoreStep::RecreateHostHandles
        | SnapshotRestoreStep::RecheckCurrentAdmission
        | SnapshotRestoreStep::ActivateRuntime => None,
    }
}
