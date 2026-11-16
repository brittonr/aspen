//! Typed effect boundaries for the optional compute-capacity service.

/// Supplied observations only. A provider does not acquire allocation authority by reporting
/// hardware.
pub trait ComputeOfferPort {
    fn next_offer(&mut self) -> crate::fabric::FabricPortResult<Option<molten_core::compute_capacity::Offer>>;
}

/// Bounded read-only adapter for externally supplied observations; it never probes a host.
#[derive(Debug, Clone)]
pub struct SuppliedOffers {
    offers: Vec<molten_core::compute_capacity::Offer>,
    next: usize,
}

impl SuppliedOffers {
    // r[impl molten.compute_capacity.offers]
    pub fn new(offers: Vec<molten_core::compute_capacity::Offer>) -> crate::fabric::FabricPortResult<Self> {
        if offers.len() > molten_core::compute_capacity::MAX_ITEMS {
            return Err(crate::fabric::FabricPortError::malformed("too many supplied compute offers"));
        }
        for offer in &offers {
            molten_core::compute_capacity::validate_offer(offer).map_err(|issue| {
                crate::fabric::FabricPortError::malformed(format!("invalid compute offer: {issue:?}"))
            })?;
        }
        Ok(Self { offers, next: 0 })
    }
}

impl ComputeOfferPort for SuppliedOffers {
    fn next_offer(&mut self) -> crate::fabric::FabricPortResult<Option<molten_core::compute_capacity::Offer>> {
        let offer = self.offers.get(self.next).cloned();
        if offer.is_some() {
            self.next += 1;
        }
        Ok(offer)
    }
}

/// A backend may have applied an effect even when it cannot return an acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendIssue {
    pub message: String,
    pub has_possible_effect: bool,
}

// r[impl molten.compute_capacity.uncertainty_release]
pub trait ComputeBackendPort {
    fn activate(
        &mut self,
        allocation: &molten_core::compute_capacity::Allocation,
    ) -> Result<molten_core::compute_capacity::ActivationEvidence, BackendIssue>;
    fn release(
        &mut self,
        allocation: &molten_core::compute_capacity::Allocation,
    ) -> Result<molten_core::compute_capacity::BackendReleaseEvidence, BackendIssue>;
    /// Read-only, exact-allocation reconciliation from the selected backend owner.
    fn observe_activation(
        &mut self,
        allocation: &molten_core::compute_capacity::Allocation,
    ) -> Result<Option<molten_core::compute_capacity::ActivationEvidence>, BackendIssue>;
    fn observe_release(
        &mut self,
        allocation: &molten_core::compute_capacity::Allocation,
    ) -> Result<Option<molten_core::compute_capacity::BackendReleaseEvidence>, BackendIssue>;
}

/// The shell does not trust caller-authored grant, revocation, or label-strength fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedReservationFacts {
    pub grant: molten_core::compute_capacity::GrantFacts,
    pub minimum_label_authority: molten_core::fabric_membership::LabelAuthority,
}

// r[impl molten.compute_capacity.reservation_authority]
pub trait ComputeAdmissionPort {
    fn reservation_facts(
        &self,
        input: &molten_core::compute_capacity::ReservationInput,
        current: &molten_core::compute_capacity::CapacityState,
    ) -> crate::fabric::FabricPortResult<AdmittedReservationFacts>;
}

// r[impl molten.compute_capacity.reservation_authority]
pub trait ComputeTimePort {
    fn now_ticks(&self) -> crate::fabric::FabricPortResult<u64>;
}

/// One selected serialized owner for every pool in a state. `compare_and_store` is a
/// durable atomic compare-and-swap; a stale expected state never writes `next`.
// r[impl molten.compute_capacity.reservation_authority]
// r[impl molten.compute_capacity.durable_reconciliation]
pub trait ComputeStorePort {
    fn load(&self) -> crate::fabric::FabricPortResult<molten_core::compute_capacity::CapacityState>;
    /// Local selected owner identity. An advertised pool for another owner cannot be spent here.
    fn selected_authority_id(&self) -> &str;
    fn compare_and_store(
        &mut self,
        expected: &molten_core::compute_capacity::CapacityState,
        next: &molten_core::compute_capacity::CapacityState,
    ) -> crate::fabric::FabricPortResult<()>;
}
