// Independently provisioned cohort-policy admission.

pub(super) fn policy(expected: &super::ExpectedCohort) -> Result<chaoscontrol_smr::SmrWorkloadPlan, super::ImportIssue> {
    super::bounds::encoded(expected, super::MAX_EXPECTED_BYTES)?;
    if expected.schema != super::EXPECTED_SCHEMA || expected.producer_revision != super::super::PRODUCER_REVISION {
        return Err(super::ImportIssue::Schema);
    }
    // This source-side importer has no attestation rail. An execution identity,
    // self-declared class or accepted semantic comparison cannot supply one.
    if expected.evidence_class != chaoscontrol_smr::observation::guest::EvidenceClass::Instrumentation || expected.claim != "bounded-instrumentation" {
        return Err(super::ImportIssue::Claim);
    }
    chaoscontrol_smr::observation::guest::validate_claim(expected.evidence_class, &expected.claim).map_err(|_| super::ImportIssue::Claim)?;
    for value in [
        &expected.group_ref,
        &expected.boundary_ref,
        &expected.scheduler_ref,
        &expected.molten_source_ref,
        &expected.molten_build_ref,
        &expected.dependency_closure_ref,
        &expected.adapter_ref,
        &expected.observer_ref,
    ] {
        reference(value)?;
    }
    let plan = chaoscontrol_smr::admit_profile(&expected.workload).map_err(|_| super::ImportIssue::Policy)?;
    expected.protocol.admit().map_err(|_| super::ImportIssue::Policy)?;
    profile_limits(expected)?;
    operations(expected)?;
    sources(expected)?;
    Ok(plan)
}

fn profile_limits(expected: &super::ExpectedCohort) -> Result<(), super::ImportIssue> {
    let p = &expected.protocol;
    let w = &expected.workload.bounds;
    let operation_count = u64::try_from(expected.operations.len()).map_err(|_| super::ImportIssue::Bounds)?;
    if expected.workload.observation_mode != chaoscontrol_smr::ObservationMode::Lossless
        || p.marker_policy != chaoscontrol_smr::observation::MarkerPolicy::Disabled
        || p.oracle_adapter_ref != expected.adapter_ref
        || p.independent_oracle_ref == expected.observer_ref
        || u64::try_from(p.participants.len()).map_err(|_| super::ImportIssue::Bounds)? > super::MAX_PARTICIPANTS
        || p.bounds.records > super::MAX_OBSERVATIONS
        || p.bounds.oracle_work > super::MAX_OBSERVATIONS
        || w.maximum_trace_events > super::MAX_OBSERVATIONS
        || w.maximum_commands > super::MAX_OPERATIONS
        || w.maximum_command_bytes > super::MAX_COMMAND_BYTES
        || w.maximum_fault_actions > super::MAX_EFFECTS
        || w.maximum_evidence_bytes > super::MAX_BUNDLE_BYTES
        || w.maximum_replay_events > (4 * super::MAX_OBSERVATIONS)
        || expected.operations.is_empty()
        || operation_count > super::MAX_OPERATIONS
        || operation_count > w.maximum_commands
        || u64::try_from(expected.faults.len()).map_err(|_| super::ImportIssue::Bounds)? > super::MAX_EFFECTS
    {
        return Err(super::ImportIssue::Policy);
    }
    Ok(())
}

fn operations(expected: &super::ExpectedCohort) -> Result<(), super::ImportIssue> {
    let w = &expected.workload.bounds;
    let mut operations = std::collections::BTreeSet::new();
    let mut logicals = std::collections::BTreeSet::new();
    let mut boundaries = std::collections::BTreeSet::new();
    for (offset, operation) in expected.operations.iter().enumerate() {
        if operation.command_index != u64::try_from(offset).map_err(|_| super::ImportIssue::Bounds)? + 1
            || operation.operation_ref != super::operation_ref(&operation.client_session, operation.sequence)?
            || !operations.insert(&operation.operation_ref)
            || !logicals.insert((&operation.client_session, operation.sequence))
        {
            return Err(super::ImportIssue::Policy);
        }
        let command_bytes = u64::try_from(operation.command_bytes.len()).map_err(|_| super::ImportIssue::Bounds)?;
        if operation.command_bytes.is_empty()
            || command_bytes > super::MAX_COMMAND_BYTES
            || command_bytes > w.maximum_command_bytes
        {
            return Err(super::ImportIssue::Bounds);
        }
        for value in [
            &operation.command_ref,
            &operation.application_state_ref,
            &operation.boundary_ref,
            &operation.transition_class_ref,
        ] {
            reference(value)?;
        }
        if !boundaries.insert(&operation.boundary_ref) {
            return Err(super::ImportIssue::Policy);
        }
        // Commands are canonical packed Molten envelopes, never rendered JSON or Rust layout.
        super::bounds::command(&operation.command_bytes, &operation.command_ref)?;
    }
    Ok(())
}

fn sources(expected: &super::ExpectedCohort) -> Result<(), super::ImportIssue> {
    let operation_count = u64::try_from(expected.operations.len()).map_err(|_| super::ImportIssue::Bounds)?;
    let p = &expected.protocol;
    let mut faults = std::collections::BTreeSet::new();
    for fault in &expected.faults {
        if fault.action_id.is_empty() || fault.action_id.len() > 256 || !faults.insert(&fault.action_id) {
            return Err(super::ImportIssue::Policy);
        }
    }
    if expected.operations.last().map(|o| &o.boundary_ref) != Some(&expected.boundary_ref)
        || operation_count > p.bounds.boundaries
        || operation_count > p.bounds.cohort_backlog
    {
        return Err(super::ImportIssue::Policy);
    }
    if u64::try_from(expected.sources.len()).map_err(|_| super::ImportIssue::Bounds)? > super::MAX_PARTICIPANTS {
        return Err(super::ImportIssue::Bounds);
    }
    let intake = super::Intake {
        sources: &expected.sources,
        execution_ref: &expected.group_ref,
        records: &[],
        drains: &[],
    };
    chaoscontrol_smr::observation::guest::validate_sources(p, &intake).map_err(|_| super::ImportIssue::Policy)?;
    for source in &expected.sources {
        if source.source_revision_ref != expected.molten_source_ref
            || source.executable_ref != expected.molten_build_ref
            || source.adapter_ref != expected.adapter_ref
        {
            return Err(super::ImportIssue::Policy);
        }
    }
    Ok(())
}
