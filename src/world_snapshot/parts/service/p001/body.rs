
/// The ChaosControl observation must match the machine descriptor and destination cohort and be
/// verified.
fn verify_chaos_descriptor(
    chaos_observation: &ChaosControlDescriptorObservation,
    descriptor: &SnapshotDescriptor,
    destination: &SnapshotCohort,
) -> Result<()> {
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
    Ok(())
}

fn validate_opaque_restore(restored: &[SnapshotStepObservation]) -> Result<()> {
    if restored.is_empty()
        || restored.len() > MAX_SNAPSHOT_COMPONENTS
        || restored.iter().any(|observation| observation.step != SnapshotRestoreStep::RestoreOpaqueMachine)
    {
        return Err(MoltenError::invalid_harness(
            "ChaosControl restore observations are empty, overbound, or use the wrong step",
        ));
    }
    for observation in restored {
        validate_ref(&observation.observation_ref, "ChaosControl opaque restore observation")?;
    }
    Ok(())
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
    let is_stale = minimum_generation.is_some_and(|minimum| observation.generation < minimum);
    if !observation.allowed
        || observation.descriptor_ref != descriptor_ref
        || observation.profile_ref != descriptor.profile_ref.as_str()
        || observation.cohort_ref != destination.cohort_ref.as_str()
        || is_stale
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
