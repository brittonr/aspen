use std::collections::BTreeMap;

use molten_core::world_snapshot::*;

use super::CanonicalSnapshotArtifact;
use super::ChaosControlDescriptorObservation;
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
    let descriptor_ref = &canonical_descriptor.artifact_ref;
    for step in &restore_plan.steps {
        let observation = match step {
            SnapshotRestoreStep::VerifyClosure => observed(*step, descriptor_ref.clone()),
            SnapshotRestoreStep::VerifyCohort => observed(*step, canonical_compatibility.artifact_ref.clone()),
            SnapshotRestoreStep::MaterializeArtifacts => {
                let observation = materialization_for(&materializations, SnapshotComponentKind::Artifact)?;
                observed(*step, observation.observation_ref.clone())
            }
            SnapshotRestoreStep::RecreateHostHandles => {
                let observation_ref = ports.handles.recreate_handles(descriptor_ref)?;
                validate_ref(&observation_ref, "host-handle recreation observation")?;
                observed(*step, observation_ref)
            }
            SnapshotRestoreStep::RecheckCurrentAdmission => {
                let admission = ports.admission.observe_current(descriptor, descriptor_ref, destination)?;
                let prior_generation = Some(final_admission.generation);
                validate_admission(&admission, descriptor, destination, descriptor_ref, prior_generation)?;
                let observation = observed(*step, admission.admission_ref.clone());
                final_admission = admission;
                observation
            }
            SnapshotRestoreStep::ActivateRuntime => {
                let observation_ref = ports.runtime.activate(descriptor_ref)?;
                validate_ref(&observation_ref, "snapshot activation observation")?;
                observed(*step, observation_ref)
            }
            SnapshotRestoreStep::RestoreOpaqueMachine => {
                return Err(MoltenError::invalid_harness("logical snapshot plan contains an opaque restore step"));
            }
            state_step => restore_state_component(ports.runtime, descriptor, &materializations, *state_step)?,
        };
        observations.push(observation);
    }

    let receipt_input = restored_receipt(
        &canonical_descriptor.artifact_ref,
        &canonical_compatibility.artifact_ref,
        &canonical_plan.artifact_ref,
        final_admission.admission_ref,
    );
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
    verify_chaos_descriptor(&chaos_observation, descriptor, destination)?;
    let initial_admission =
        ports.admission.observe_current(descriptor, &canonical_descriptor.artifact_ref, destination)?;
    validate_admission(&initial_admission, descriptor, destination, &canonical_descriptor.artifact_ref, None)?;
    let restore_plan = plan_restore(descriptor, destination, true).map_err(|report| {
        MoltenError::invalid_harness(format!("opaque snapshot restore denied: {:?}", report.issues))
    })?;
    let canonical_plan = canonical_snapshot_restore_plan(&restore_plan)?;
    let materializations = materialize_complete_inventory(descriptor, ports.materialization)?;
    let artifact_ref = materialization_for(&materializations, SnapshotComponentKind::Artifact)?.observation_ref.clone();
    let mut observations = vec![
        observed(SnapshotRestoreStep::VerifyClosure, canonical_descriptor.artifact_ref.clone()),
        observed(SnapshotRestoreStep::VerifyCohort, canonical_compatibility.artifact_ref.clone()),
        observed(SnapshotRestoreStep::MaterializeArtifacts, artifact_ref),
    ];
    let restored = ports.runtime.restore_exact(descriptor, destination)?;
    validate_opaque_restore(&restored)?;
    observations.extend(restored);
    let handle_ref = ports.handles.recreate_handles(&canonical_descriptor.artifact_ref)?;
    validate_ref(&handle_ref, "opaque host-handle recreation observation")?;
    observations.push(observed(SnapshotRestoreStep::RecreateHostHandles, handle_ref));
    let final_admission =
        ports.admission.observe_current(descriptor, &canonical_descriptor.artifact_ref, destination)?;
    validate_admission(
        &final_admission,
        descriptor,
        destination,
        &canonical_descriptor.artifact_ref,
        Some(initial_admission.generation),
    )?;
    observations.push(observed(SnapshotRestoreStep::RecheckCurrentAdmission, final_admission.admission_ref.clone()));
    let activation_ref = ports.runtime.activate(&canonical_descriptor.artifact_ref)?;
    validate_ref(&activation_ref, "opaque snapshot activation observation")?;
    observations.push(observed(SnapshotRestoreStep::ActivateRuntime, activation_ref));
    let receipt_input = restored_receipt(
        &canonical_descriptor.artifact_ref,
        &canonical_compatibility.artifact_ref,
        &canonical_plan.artifact_ref,
        final_admission.admission_ref,
    );
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

const fn observed(step: SnapshotRestoreStep, observation_ref: String) -> SnapshotStepObservation {
    SnapshotStepObservation { step, observation_ref }
}

fn restored_receipt(
    descriptor_ref: &str,
    compatibility_ref: &str,
    restore_plan_ref: &str,
    current_admission_ref: String,
) -> SnapshotReceipt {
    SnapshotReceipt {
        decision: SnapshotReceiptDecision::Restored,
        descriptor_ref: descriptor_ref.to_string(),
        compatibility_ref: compatibility_ref.to_string(),
        restore_plan_ref: Some(restore_plan_ref.to_string()),
        clone_plan_ref: None,
        current_admission_ref: Some(current_admission_ref),
        issues: Vec::new(),
        non_claims: SNAPSHOT_NON_CLAIMS.iter().map(ToString::to_string).collect(),
    }
}

/// Restores one logical state component from its materialization and requires an observation for
/// that step.
fn restore_state_component<R: LogicalSnapshotRestorePort>(
    runtime: &mut R,
    descriptor: &SnapshotDescriptor,
    materializations: &BTreeMap<SnapshotComponentKind, SnapshotMaterializationObservation>,
    state_step: SnapshotRestoreStep,
) -> Result<SnapshotStepObservation> {
    let kind = component_for_step(state_step)
        .ok_or_else(|| MoltenError::invalid_harness("logical snapshot plan contains an unmapped restore step"))?;
    let component = descriptor
        .components
        .iter()
        .find(|component| component.kind == kind)
        .ok_or_else(|| MoltenError::invalid_harness("logical snapshot component disappeared"))?;
    let materialization = materialization_for(materializations, kind)?;
    let observation = runtime.restore_component(state_step, component, materialization)?;
    if observation.step != state_step {
        return Err(MoltenError::invalid_harness(
            "logical snapshot adapter returned an observation for the wrong step",
        ));
    }
    validate_ref(&observation.observation_ref, "logical restore observation")?;
    Ok(observation)
}
