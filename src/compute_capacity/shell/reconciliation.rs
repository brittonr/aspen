// Release, reconciliation, lifecycle and advisory operations of the selected owner.
impl<S: ComputeStorePort, P: ComputeOfferPort, B: ComputeBackendPort, A: ComputeAdmissionPort, T: ComputeTimePort>
    Service<S, P, B, A, T>
{
    // r[impl molten.compute_capacity.uncertainty_release]
    pub fn release(&mut self, owner: &molten_core::compute_capacity::OwnerInput) -> crate::fabric::FabricPortResult<ReleaseOutcome> {
        let current = self.store.load()?;
        let Some(allocation) = current.allocations.get(&owner.allocation_id) else {
            return Ok(match current.retired_allocations.iter().find(|row| row.id == owner.allocation_id) {
                Some(row) if row.consumer_id == owner.consumer_id && row.owner_generation == owner.owner_generation => {
                    ReleaseOutcome::AlreadyReleased(row.clone())
                }
                _ => ReleaseOutcome::Denied(molten_core::compute_capacity::CapacityIssue::WrongOwner),
            });
        };
        if allocation.consumer_id != owner.consumer_id || allocation.owner_generation != owner.owner_generation {
            return Ok(ReleaseOutcome::Denied(molten_core::compute_capacity::CapacityIssue::WrongOwner));
        }
        let allocation = allocation.clone();
        if allocation.status == molten_core::compute_capacity::ReservationStatus::Quarantined {
            return Ok(ReleaseOutcome::Uncertain(UncertainOutcome {
                allocation: Some(allocation),
                phase: Phase::BackendRelease,
                message: "pending release requires explicit backend reconciliation; do not repeat a possible effect"
                    .into(),
                has_possible_effect: true,
                port_error: None,
            }));
        }
        if allocation.kind == molten_core::compute_capacity::AllocationKind::ServingDeployment
            && current.allocations.values().any(|dependent| {
                dependent.serving_allocation_id.as_deref() == Some(&allocation.id)
                    && dependent.status != molten_core::compute_capacity::ReservationStatus::Released
            })
        {
            return Ok(ReleaseOutcome::Denied(molten_core::compute_capacity::CapacityIssue::InsufficientEvidence));
        }
        let pending = molten_core::compute_capacity::lifecycle(&current, owner, molten_core::compute_capacity::LifecycleEvent::Drain);
        if let Some(issue) = pending.issue {
            return Ok(ReleaseOutcome::Denied(issue));
        }
        if let Err(error) = self.store.compare_and_store(&current, &pending.state) {
            return Ok(if matches!(error, crate::fabric::FabricPortError::UncertainExternalOutcome { .. }) {
                ReleaseOutcome::Uncertain(UncertainOutcome {
                    allocation: Some(allocation),
                    phase: Phase::ReleasePersistence,
                    message: error.to_string(),
                    has_possible_effect: false,
                    port_error: Some(error),
                })
            } else {
                ReleaseOutcome::PortDenied(error)
            });
        }
        let evidence = match self.backend.release(&allocation) {
            Ok(evidence) => evidence,
            Err(error) => {
                return Ok(ReleaseOutcome::Uncertain(backend_uncertain(
                    &allocation,
                    Phase::BackendRelease,
                    error,
                )));
            }
        };
        self.reconcile_release_from(&pending.state, owner, &evidence)
    }

    /// Query the selected backend owner. Caller-authored evidence never establishes release.
    // r[impl molten.compute_capacity.uncertainty_release]
    pub fn reconcile_release(&mut self, owner: &molten_core::compute_capacity::OwnerInput) -> crate::fabric::FabricPortResult<ReleaseOutcome> {
        let state = self.store.load()?;
        let Some(allocation) = state.allocations.get(&owner.allocation_id) else {
            return Ok(match state.retired_allocations.iter().find(|row| row.id == owner.allocation_id) {
                Some(row) if row.consumer_id == owner.consumer_id && row.owner_generation == owner.owner_generation => {
                    ReleaseOutcome::AlreadyReleased(row.clone())
                }
                _ => ReleaseOutcome::Denied(molten_core::compute_capacity::CapacityIssue::WrongOwner),
            });
        };
        if allocation.consumer_id != owner.consumer_id || allocation.owner_generation != owner.owner_generation {
            return Ok(ReleaseOutcome::Denied(molten_core::compute_capacity::CapacityIssue::WrongOwner));
        }
        let evidence = match self.backend.observe_release(allocation) {
            Ok(Some(evidence)) => evidence,
            Ok(None) => return Ok(ReleaseOutcome::Denied(molten_core::compute_capacity::CapacityIssue::UncertainRelease)),
            Err(error) => {
                return Ok(ReleaseOutcome::Uncertain(backend_uncertain(
                    allocation,
                    Phase::BackendRelease,
                    error,
                )));
            }
        };
        self.reconcile_release_from(&state, owner, &evidence)
    }

    fn reconcile_release_from(
        &mut self,
        state: &molten_core::compute_capacity::CapacityState,
        owner: &molten_core::compute_capacity::OwnerInput,
        evidence: &molten_core::compute_capacity::BackendReleaseEvidence,
    ) -> crate::fabric::FabricPortResult<ReleaseOutcome> {
        let outcome = molten_core::compute_capacity::release(state, &molten_core::compute_capacity::ReleaseInput {
            owner: owner.clone(),
            backend_evidence: Some(evidence.clone()),
        });
        if let Some(issue) = outcome.issue {
            return Ok(if issue == molten_core::compute_capacity::CapacityIssue::UncertainRelease {
                ReleaseOutcome::Uncertain(UncertainOutcome {
                    allocation: state.allocations.get(&owner.allocation_id).cloned(),
                    phase: Phase::BackendRelease,
                    message: "release not established by matching backend evidence".into(),
                    has_possible_effect: true,
                    port_error: None,
                })
            } else {
                ReleaseOutcome::Denied(issue)
            });
        }
        if outcome.state == *state {
            let Some(allocation) = state
                .retired_allocations
                .iter()
                .find(|row| row.id == owner.allocation_id)
            else {
                return Ok(ReleaseOutcome::Uncertain(UncertainOutcome {
                    allocation: state.allocations.get(&owner.allocation_id).cloned(),
                    phase: Phase::BackendRelease,
                    message: "successful unchanged release has no retained retired allocation".into(),
                    has_possible_effect: true,
                    port_error: None,
                }));
            };
            return Ok(ReleaseOutcome::AlreadyReleased(allocation.clone()));
        }
        if let Err(error) = self.store.compare_and_store(state, &outcome.state) {
            return Ok(ReleaseOutcome::Uncertain(UncertainOutcome {
                allocation: state.allocations.get(&owner.allocation_id).cloned(),
                phase: Phase::ReleasePersistence,
                message: error.to_string(),
                has_possible_effect: true,
                port_error: Some(error),
            }));
        }
        let Some(allocation) = outcome
            .state
            .retired_allocations
            .iter()
            .find(|row| row.id == owner.allocation_id)
        else {
            return Ok(ReleaseOutcome::Uncertain(UncertainOutcome {
                allocation: state.allocations.get(&owner.allocation_id).cloned(),
                phase: Phase::ReleasePersistence,
                message: "successful release has no retained retired allocation".into(),
                has_possible_effect: true,
                port_error: None,
            }));
        };
        Ok(ReleaseOutcome::Released(allocation.clone()))
    }

    /// Read-only evidence query from the selected backend; never replay activation on reopen.
    // r[impl molten.compute_capacity.reservation_authority]
    pub fn reconcile_activation(&mut self, owner: &molten_core::compute_capacity::OwnerInput) -> crate::fabric::FabricPortResult<ReserveOutcome> {
        let state = self.store.load()?;
        let Some(allocation) = state.allocations.get(&owner.allocation_id) else {
            return Ok(ReserveOutcome::Denied(molten_core::compute_capacity::CapacityIssue::WrongOwner));
        };
        if allocation.consumer_id != owner.consumer_id || allocation.owner_generation != owner.owner_generation {
            return Ok(ReserveOutcome::Denied(molten_core::compute_capacity::CapacityIssue::WrongOwner));
        }
        let evidence = match self.backend.observe_activation(allocation) {
            Ok(Some(evidence)) => evidence,
            Ok(None) => return Ok(ReserveOutcome::Denied(molten_core::compute_capacity::CapacityIssue::InsufficientEvidence)),
            Err(error) => {
                return Ok(ReserveOutcome::Uncertain(backend_uncertain(
                    allocation,
                    Phase::BackendActivation,
                    error,
                )));
            }
        };
        let outcome = molten_core::compute_capacity::acknowledge_activation(&state, owner, &evidence);
        if let Some(issue) = outcome.issue {
            return Ok(ReserveOutcome::Denied(issue));
        }
        if outcome.state == state {
            return Ok(ReserveOutcome::Existing(allocation.clone()));
        }
        if let Err(error) = self.store.compare_and_store(&state, &outcome.state) {
            return Ok(ReserveOutcome::Uncertain(UncertainOutcome {
                allocation: Some(allocation.clone()),
                phase: Phase::CommitPersistence,
                message: error.to_string(),
                has_possible_effect: true,
                port_error: Some(error),
            }));
        }
        Ok(ReserveOutcome::Reserved(outcome.state.allocations[&owner.allocation_id].clone()))
    }

    /// Persist quarantine before returning the pure effect plan. An already quarantined
    /// allocation only requests read-only reconciliation; it never repeats a possible stop.
    // r[impl molten.compute_capacity.uncertainty_release]
    pub fn event(
        &mut self,
        owner: &molten_core::compute_capacity::OwnerInput,
        event: molten_core::compute_capacity::LifecycleEvent,
    ) -> crate::fabric::FabricPortResult<Result<EventOutcome, molten_core::compute_capacity::CapacityIssue>> {
        let current = self.store.load()?;
        let Some(allocation) = current.allocations.get(&owner.allocation_id) else {
            return Ok(Err(molten_core::compute_capacity::CapacityIssue::WrongOwner));
        };
        if allocation.consumer_id != owner.consumer_id || allocation.owner_generation != owner.owner_generation {
            return Ok(Err(molten_core::compute_capacity::CapacityIssue::WrongOwner));
        }
        if allocation.status == molten_core::compute_capacity::ReservationStatus::Quarantined {
            return Ok(Ok(EventOutcome {
                allocation: allocation.clone(),
                effects: vec![molten_core::compute_capacity::EffectIntent::Reconcile(owner.allocation_id.clone())],
            }));
        }
        let outcome = molten_core::compute_capacity::lifecycle(&current, owner, event);
        if let Some(issue) = outcome.issue {
            return Ok(Err(issue));
        }
        self.store.compare_and_store(&current, &outcome.state)?;
        Ok(Ok(EventOutcome {
            allocation: outcome.state.allocations[&owner.allocation_id].clone(),
            effects: outcome.effects,
        }))
    }

    /// Advisory check uses the currently durable state; callers still use `reserve` for authority.
    pub fn eligibility(&self, input: &molten_core::compute_capacity::ReservationInput) -> crate::fabric::FabricPortResult<molten_core::compute_capacity::Eligibility> {
        let current = self.store.load()?;
        let admitted = self.admitted_input(&current, input)?;
        let outcome = molten_core::compute_capacity::eligibility(&current, &admitted);
        if outcome == molten_core::compute_capacity::Eligibility::Eligible {
            let offer = &current.offers[&admitted.host_id];
            if admitted.claims.iter().any(|claim| {
                offer
                    .pools
                    .iter()
                    .any(|pool| pool.id == claim.pool_id && pool.authority_id != self.store.selected_authority_id())
            }) {
                return Ok(molten_core::compute_capacity::Eligibility::Rejected(molten_core::compute_capacity::CapacityIssue::WeakAuthority));
            }
        }
        Ok(outcome)
    }
}
