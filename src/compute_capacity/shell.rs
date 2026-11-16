//! Imperative shell: current state → pure decision → durable intent → backend effect → durable
//! acknowledgement.

use super::ports::ComputeAdmissionPort;
use super::ports::ComputeBackendPort;
use super::ports::ComputeOfferPort;
use super::ports::ComputeStorePort;
use super::ports::ComputeTimePort;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Observation,
    IntentPersistence,
    BackendActivation,
    CommitPersistence,
    BackendRelease,
    ReleasePersistence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertainOutcome {
    pub allocation: Option<molten_core::compute_capacity::Allocation>,
    pub phase: Phase,
    pub message: String,
    pub has_possible_effect: bool,
    pub port_error: Option<crate::fabric::FabricPortError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveOutcome {
    Reserved(molten_core::compute_capacity::Allocation),
    Existing(molten_core::compute_capacity::Allocation),
    Denied(molten_core::compute_capacity::CapacityIssue),
    PortDenied(crate::fabric::FabricPortError),
    Uncertain(UncertainOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseOutcome {
    Released(molten_core::compute_capacity::Allocation),
    AlreadyReleased(molten_core::compute_capacity::Allocation),
    Denied(molten_core::compute_capacity::CapacityIssue),
    PortDenied(crate::fabric::FabricPortError),
    Uncertain(UncertainOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventOutcome {
    pub allocation: molten_core::compute_capacity::Allocation,
    pub effects: Vec<molten_core::compute_capacity::EffectIntent>,
}

pub struct Service<S, P, B, A, T> {
    pub store: S,
    pub provider: P,
    pub backend: B,
    pub admission: A,
    pub time: T,
}

impl<S: ComputeStorePort, P: ComputeOfferPort, B: ComputeBackendPort, A: ComputeAdmissionPort, T: ComputeTimePort>
    Service<S, P, B, A, T>
{
    pub fn new(store: S, provider: P, backend: B, admission: A, time: T) -> Self {
        Self {
            store,
            provider,
            backend,
            admission,
            time,
        }
    }

    // r[impl molten.compute_capacity.offers]
    pub fn observe(&mut self) -> crate::fabric::FabricPortResult<Option<molten_core::compute_capacity::Offer>> {
        let Some(offer) = self.provider.next_offer()? else {
            return Ok(None);
        };
        let current = self.store.load()?;
        let next = molten_core::compute_capacity::observe(&current, offer.clone()).map_err(|issue| {
            crate::fabric::FabricPortError::malformed(format!("compute observation denied: {issue:?}"))
        })?;
        self.store.compare_and_store(&current, &next)?;
        Ok(Some(offer))
    }

    // r[impl molten.compute_capacity.readback]
    pub fn inspect(
        &self,
        host_id: &str,
        now: u64,
    ) -> crate::fabric::FabricPortResult<Option<molten_core::compute_capacity::CapacityReadback>> {
        Ok(molten_core::compute_capacity::inspect(&self.store.load()?, host_id, now))
    }

    // r[impl molten.compute_capacity.readback]
    // r[impl molten.compute_capacity.durable_reconciliation]
    pub fn inspect_record(
        &self,
        host_id: &str,
        now: u64,
    ) -> crate::fabric::FabricPortResult<Option<super::canonical::Record>> {
        self.inspect(host_id, now)?
            .map(|readback| super::canonical::readback(&readback).map_err(crate::fabric::FabricPortError::from))
            .transpose()
    }

    // r[impl molten.compute_capacity.reservation_authority]
    fn admitted_input(
        &self,
        current: &molten_core::compute_capacity::CapacityState,
        input: &molten_core::compute_capacity::ReservationInput,
    ) -> crate::fabric::FabricPortResult<molten_core::compute_capacity::ReservationInput> {
        let mut admitted = input.clone();
        let facts = self.admission.reservation_facts(input, current)?;
        admitted.grant = facts.grant;
        admitted.minimum_label_authority = facts.minimum_label_authority;
        admitted.now = self.time.now_ticks()?;
        Ok(admitted)
    }

    // r[impl molten.compute_capacity.reservation_authority]
    pub fn reserve(
        &mut self,
        input: &molten_core::compute_capacity::ReservationInput,
    ) -> crate::fabric::FabricPortResult<ReserveOutcome> {
        let current = self.store.load()?;
        let admitted = match self.admitted_input(&current, input) {
            Ok(input) => input,
            Err(error) => return Ok(ReserveOutcome::PortDenied(error)),
        };
        let transition = molten_core::compute_capacity::reserve(&current, &admitted);
        let allocation = match transition.decision {
            molten_core::compute_capacity::ReservationOutcome::Rejected(issue) => {
                return Ok(ReserveOutcome::Denied(issue));
            }
            molten_core::compute_capacity::ReservationOutcome::Existing { allocation } => {
                return Ok(if allocation.status == molten_core::compute_capacity::ReservationStatus::Active {
                    ReserveOutcome::Existing(allocation)
                } else {
                    ReserveOutcome::Uncertain(UncertainOutcome {
                        allocation: Some(allocation),
                        phase: Phase::IntentPersistence,
                        message: "existing operation has no acknowledged activation; reconcile without replay".into(),
                        has_possible_effect: true,
                        port_error: None,
                    })
                });
            }
            molten_core::compute_capacity::ReservationOutcome::Reserved { allocation, .. } => allocation,
        };
        if allocation.authority_ids.iter().any(|authority| authority != self.store.selected_authority_id()) {
            return Ok(ReserveOutcome::PortDenied(crate::fabric::FabricPortError::capability(
                "selected compute owner does not control every claimed pool",
            )));
        }
        if let Err(error) = self.store.compare_and_store(&current, &transition.state) {
            return Ok(if matches!(error, crate::fabric::FabricPortError::UncertainExternalOutcome { .. }) {
                ReserveOutcome::Uncertain(UncertainOutcome {
                    allocation: None,
                    phase: Phase::IntentPersistence,
                    message: error.to_string(),
                    has_possible_effect: false,
                    port_error: Some(error),
                })
            } else {
                ReserveOutcome::PortDenied(error)
            });
        }
        let evidence = match self.backend.activate(&allocation) {
            Ok(evidence) => evidence,
            Err(error) if !error.has_possible_effect => {
                let aborted = molten_core::compute_capacity::abort_unstarted_intent(
                    &transition.state,
                    &owner_of(&allocation),
                    true,
                );
                if aborted.issue.is_some() {
                    return Ok(ReserveOutcome::Uncertain(backend_uncertain(
                        &allocation,
                        Phase::BackendActivation,
                        error,
                    )));
                }
                if let Err(persist_error) = self.store.compare_and_store(&transition.state, &aborted.state) {
                    return Ok(ReserveOutcome::Uncertain(UncertainOutcome {
                        allocation: Some(allocation),
                        phase: Phase::ReleasePersistence,
                        message: persist_error.to_string(),
                        has_possible_effect: false,
                        port_error: Some(persist_error),
                    }));
                }
                return Ok(ReserveOutcome::PortDenied(crate::fabric::FabricPortError::capability(error.message)));
            }
            Err(error) => {
                return Ok(ReserveOutcome::Uncertain(backend_uncertain(&allocation, Phase::BackendActivation, error)));
            }
        };
        let acknowledged =
            molten_core::compute_capacity::acknowledge_activation(&transition.state, &owner_of(&allocation), &evidence);
        if acknowledged.issue.is_some() {
            return Ok(ReserveOutcome::Uncertain(UncertainOutcome {
                allocation: Some(allocation),
                phase: Phase::BackendActivation,
                message: "backend activation evidence did not match reservation intent".into(),
                has_possible_effect: true,
                port_error: None,
            }));
        }
        if let Err(error) = self.store.compare_and_store(&transition.state, &acknowledged.state) {
            return Ok(ReserveOutcome::Uncertain(UncertainOutcome {
                allocation: Some(allocation),
                phase: Phase::CommitPersistence,
                message: error.to_string(),
                has_possible_effect: true,
                port_error: Some(error),
            }));
        }
        Ok(ReserveOutcome::Reserved(acknowledged.state.allocations[&allocation.id].clone()))
    }
}

include!("shell/reconciliation.rs");

fn owner_of(allocation: &molten_core::compute_capacity::Allocation) -> molten_core::compute_capacity::OwnerInput {
    molten_core::compute_capacity::OwnerInput {
        allocation_id: allocation.id.clone(),
        consumer_id: allocation.consumer_id.clone(),
        owner_generation: allocation.owner_generation,
    }
}

fn backend_uncertain(
    allocation: &molten_core::compute_capacity::Allocation,
    phase: Phase,
    error: super::ports::BackendIssue,
) -> UncertainOutcome {
    UncertainOutcome {
        allocation: Some(allocation.clone()),
        phase,
        message: error.message,
        has_possible_effect: error.has_possible_effect,
        port_error: None,
    }
}
