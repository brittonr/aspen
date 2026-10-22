use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::super::*;
use super::*;

/// The admitted observation boundary of one replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayObservationAdmission {
    pub inventory_ref: String,
    pub ledger_ref: String,
    /// The last recorded slot, as (step, logical position).
    pub horizon: Option<(u64, u64)>,
    /// The recorded observations of external mutations, returned sealed.
    pub sealed_effect_refs: Vec<String>,
    pub recorded_observations: u32,
    pub deterministic_sources: u32,
    pub simulated_sources: u32,
    pub recorded_sources: u32,
    pub opaque_diagnostic_refs: Vec<String>,
}

/// Admit the observation boundary of `trace`, or return every blocker.
///
/// A blocker cannot be waived: replay success requires an empty list.
// r[impl molten.world_replay_boundary.completeness]
pub fn admit_replay_observations(
    boundary: &WorldReplayObservationBoundary,
    trace: &WorldTransitionTrace,
) -> Result<ReplayObservationAdmission, Vec<ReplayBoundaryBlocker>> {
    let mut blockers = validate_replay_source_inventory(&boundary.inventory);
    blockers.extend(binding_blockers(boundary, trace));
    blockers.extend(source_reach_blockers(&boundary.inventory, &boundary.ledger, trace));
    blockers.extend(observation_blockers(&boundary.inventory, &boundary.ledger));
    blockers.extend(opaque_blockers(&boundary.inventory, &boundary.opaque_diagnostics));
    blockers.sort();
    blockers.dedup();
    if !blockers.is_empty() {
        blockers.truncate(MAX_REPLAY_BOUNDARY_BLOCKERS);
        return Err(blockers);
    }
    admission(boundary)
}

fn binding_blockers(
    boundary: &WorldReplayObservationBoundary,
    trace: &WorldTransitionTrace,
) -> Vec<ReplayBoundaryBlocker> {
    let ledger = &boundary.ledger;
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    if boundary.inventory.profile_ref != trace.profile.profile_ref.as_str() {
        blockers.push(ReplayBoundaryBlocker::ProfileDrift);
    }
    if ledger.schema != REPLAY_OBSERVATION_LEDGER_SCHEMA {
        blockers.push(ReplayBoundaryBlocker::InvalidRecord("ledger-schema"));
    }
    if ledger.inventory_ref != boundary.inventory.inventory_ref {
        blockers.push(ReplayBoundaryBlocker::LedgerBindingMismatch("inventory"));
    }
    if ledger.trace_ref != trace.trace_ref {
        blockers.push(ReplayBoundaryBlocker::LedgerBindingMismatch("trace"));
    }
    let is_over_limit = ledger.slots.len() > MAX_REPLAY_OBSERVATIONS
        || ledger.observations.len() > MAX_REPLAY_OBSERVATIONS
        || ledger.reached_sources.len() > MAX_REPLAY_SOURCES
        || boundary.opaque_diagnostics.len() > MAX_REPLAY_OPAQUE_DIAGNOSTICS;
    if is_over_limit {
        blockers.push(ReplayBoundaryBlocker::LimitExceeded("ledger"));
    }
    match identify_replay_observation_ledger(ledger) {
        Ok(identity) if identity == ledger.ledger_ref => {}
        _ => blockers.push(ReplayBoundaryBlocker::LedgerIdentityMismatch),
    }
    blockers
}

fn source_reach_blockers(
    inventory: &ReplaySourceInventory,
    ledger: &ReplayObservationLedger,
    trace: &WorldTransitionTrace,
) -> Vec<ReplayBoundaryBlocker> {
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    for source_id in &ledger.reached_sources {
        match inventory.source(source_id) {
            None => blockers.push(ReplayBoundaryBlocker::UnknownSource(source_id.clone())),
            Some(source) if source.handling == ReplaySourceHandling::Unsupported => {
                blockers.push(ReplayBoundaryBlocker::UnsupportedSource(source_id.clone()));
            }
            Some(_) => {}
        }
    }
    let reached = ledger.reached_sources.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let steps = trace.steps.iter().map(|step| step.position).collect::<BTreeSet<_>>();
    for slot in &ledger.slots {
        match inventory.source(&slot.source_id) {
            None => blockers.push(ReplayBoundaryBlocker::UnknownSource(slot.source_id.clone())),
            Some(source) if source.handling != ReplaySourceHandling::RecordedObservation => {
                blockers.push(ReplayBoundaryBlocker::UnrecordedHandling(slot.source_id.clone()));
            }
            Some(_) => {}
        }
        if !reached.contains(slot.source_id.as_str()) {
            blockers.push(ReplayBoundaryBlocker::UnknownSource(slot.source_id.clone()));
        }
        if !steps.contains(&slot.step_position) {
            blockers.push(ReplayBoundaryBlocker::StepOutsideTrace(slot.step_position));
        }
    }
    blockers
}

fn observation_blockers(
    inventory: &ReplaySourceInventory,
    ledger: &ReplayObservationLedger,
) -> Vec<ReplayBoundaryBlocker> {
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    if let Some((step_position, logical_position)) =
        first_order_violation(ledger.slots.iter().map(ReplayObservationSlot::order_key))
    {
        blockers.push(ReplayBoundaryBlocker::OrderingDivergence {
            step_position,
            logical_position,
        });
    }
    let mut recorded = BTreeMap::new();
    for observation in &ledger.observations {
        let key = observation.slot.order_key();
        if recorded.insert(key, observation).is_some() {
            blockers.push(ReplayBoundaryBlocker::DuplicateObservation {
                step_position: key.0,
                logical_position: key.1,
            });
        }
        blockers.extend(sealing_blockers(inventory, observation));
    }
    if let Some((step_position, logical_position)) =
        first_order_violation(ledger.observations.iter().map(|observation| observation.slot.order_key()))
        && !recorded.is_empty()
    {
        blockers.push(ReplayBoundaryBlocker::OrderingDivergence {
            step_position,
            logical_position,
        });
    }
    let slots = ledger.slots.iter().map(ReplayObservationSlot::order_key).collect::<BTreeSet<_>>();
    for (step_position, logical_position) in slots.iter().filter(|key| !recorded.contains_key(*key)) {
        blockers.push(ReplayBoundaryBlocker::MissingObservation {
            step_position: *step_position,
            logical_position: *logical_position,
        });
    }
    for (step_position, logical_position) in recorded.keys().filter(|key| !slots.contains(*key)) {
        blockers.push(ReplayBoundaryBlocker::ExtraObservation {
            step_position: *step_position,
            logical_position: *logical_position,
        });
    }
    blockers.extend(first_slot_divergence(&ledger.slots, &ledger.observations));
    blockers
}

/// Identity, cohort, protection, and secrecy of one recorded observation.
fn sealing_blockers(
    inventory: &ReplaySourceInventory,
    observation: &RecordedReplayObservation,
) -> Vec<ReplayBoundaryBlocker> {
    let reference = observation.observation_ref.clone();
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    match identify_recorded_replay_observation(observation) {
        Ok(identity) if identity == observation.observation_ref => {}
        _ => blockers.push(ReplayBoundaryBlocker::ObservationIdentityMismatch(reference.clone())),
    }
    if observation.adapter_cohort_ref != inventory.adapter_cohort_ref {
        blockers.push(ReplayBoundaryBlocker::AdapterDrift(reference.clone()));
    }
    if observation.protection_profile_ref != inventory.protection_profile_ref {
        blockers.push(ReplayBoundaryBlocker::ProtectionDrift(reference.clone()));
    }
    let is_sensitive = inventory.source(&observation.slot.source_id).is_some_and(|source| source.is_sensitive);
    if is_sensitive && observation.protection == WorldReplayMemberProtection::Public {
        blockers.push(ReplayBoundaryBlocker::PlaintextSecret(reference.clone()));
    }
    let is_well_formed = match &observation.outcome {
        ReplayObservationOutcome::Result {
            result_ref,
            byte_length,
        } => valid_reference(result_ref) && *byte_length <= MAX_REPLAY_OBSERVATION_BYTES,
        ReplayObservationOutcome::Error { error_class } => valid_boundary_text(error_class),
    };
    let is_slot_well_formed =
        valid_boundary_text(&observation.slot.operation_id) && valid_reference(&observation.slot.request_ref);
    if !is_well_formed || !is_slot_well_formed {
        blockers.push(ReplayBoundaryBlocker::MalformedObservation(reference));
    }
    blockers
}

/// An opaque diagnostic stays detached: it binds the one admitted cohort,
/// claims only detached diagnostics, and never counts toward a slot.
// r[impl molten.world_replay_boundary.opaque_native]
fn opaque_blockers(
    inventory: &ReplaySourceInventory,
    diagnostics: &[OpaqueReplayDiagnostic],
) -> Vec<ReplayBoundaryBlocker> {
    let mut blockers = Vec::with_capacity(MAX_REPLAY_BOUNDARY_BLOCKERS);
    for diagnostic in diagnostics {
        let reference = diagnostic.descriptor_ref.clone();
        if inventory.opaque_diagnostic_cohort_ref.as_deref() != Some(diagnostic.cohort_ref.as_str()) {
            blockers.push(ReplayBoundaryBlocker::OpaqueCohortMismatch(reference.clone()));
        }
        if diagnostic.claims != [OpaqueDiagnosticClaim::DetachedDiagnostic] {
            blockers.push(ReplayBoundaryBlocker::OpaqueOverclaim(reference.clone()));
        }
        if !valid_reference(&diagnostic.descriptor_ref) {
            blockers.push(ReplayBoundaryBlocker::InvalidRecord("opaque-descriptor-ref"));
        }
    }
    blockers
}

fn bounded_count(count: usize) -> Result<u32, Vec<ReplayBoundaryBlocker>> {
    u32::try_from(count).map_err(|_| vec![ReplayBoundaryBlocker::LimitExceeded("count")])
}

fn admission(
    boundary: &WorldReplayObservationBoundary,
) -> Result<ReplayObservationAdmission, Vec<ReplayBoundaryBlocker>> {
    let inventory = &boundary.inventory;
    let count = |handling| bounded_count(inventory.sources.iter().filter(|source| source.handling == handling).count());
    let sealed_effect_refs = boundary
        .ledger
        .observations
        .iter()
        .filter(|observation| {
            inventory.source(&observation.slot.source_id).is_some_and(|source| source.is_external_mutation)
        })
        .map(|observation| observation.observation_ref.clone())
        .collect();
    Ok(ReplayObservationAdmission {
        inventory_ref: inventory.inventory_ref.clone(),
        ledger_ref: boundary.ledger.ledger_ref.clone(),
        horizon: boundary.ledger.slots.last().map(ReplayObservationSlot::order_key),
        sealed_effect_refs,
        recorded_observations: bounded_count(boundary.ledger.observations.len())?,
        deterministic_sources: count(ReplaySourceHandling::Deterministic)?,
        simulated_sources: count(ReplaySourceHandling::Simulated)?,
        recorded_sources: count(ReplaySourceHandling::RecordedObservation)?,
        opaque_diagnostic_refs: boundary
            .opaque_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.descriptor_ref.clone())
            .collect(),
    })
}
