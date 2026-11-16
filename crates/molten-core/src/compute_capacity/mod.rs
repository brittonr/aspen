//! Deterministic compute offers and reservation decisions. Effects, clocks and storage belong to
//! the caller.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use crate::fabric_membership::LabelAuthority;

pub const OFFER_SCHEMA: &str = "molten.compute.offer.v1";
pub const RESERVATION_SCHEMA: &str = "molten.compute.reservation.v1";
pub const MAX_ITEMS: usize = 128;
pub const MAX_TEXT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Granularity {
    Chip,
    MmioBundle,
    Tray,
    Mesh,
    Partition,
    Alias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolKind {
    DedicatedMemoryBytes,
    UnifiedMemoryBytes,
    Chips,
}

/// Observations are explicit: absence is not an assertion of zero use or full availability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pool {
    pub id: String,
    pub kind: PoolKind,
    pub authority_id: String,
    pub total: u64,
    pub allocatable: u64,
    pub observed_used: Option<u64>,
    pub observed_free: Option<u64>,
    pub is_observation_debited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub id: String,
    pub device_id: String,
    pub partition_id: Option<String>,
    pub granularity: Granularity,
    /// Physical identities, never advertised object counts. A grouping consumes each chip.
    pub backing_pools: Vec<String>,
    pub is_grouping_enforced: bool,
}

/// Unknown fields never match a requested runtime, architecture, backend or topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compatibility {
    pub backend: Option<String>,
    pub runtime: Option<String>,
    pub architecture: Option<String>,
    pub topology: Option<String>,
    pub qualification_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub schema: String,
    pub host_id: String,
    pub host_incarnation: String,
    pub device_incarnation: String,
    pub provider_profile_ref: String,
    pub source_profile_ref: String,
    pub revision: u64,
    pub topology_generation: u64,
    pub observed_at: u64,
    /// Charge cursor captured by the observer, not by offer delivery. None means unknown basis.
    pub observation_charge_cursor: Option<u64>,
    pub expires_at: u64,
    pub health: Health,
    pub label_authority: LabelAuthority,
    pub compatibility: Compatibility,
    pub pools: Vec<Pool>,
    pub views: Vec<View>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredCompatibility {
    pub backend: Option<String>,
    pub runtime: Option<String>,
    pub architecture: Option<String>,
    pub topology: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub pool_id: String,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationInput {
    pub host_id: String,
    pub host_incarnation: String,
    pub device_incarnation: String,
    pub offer_revision: u64,
    pub topology_generation: u64,
    pub now: u64,
    pub view_id: String,
    pub granularity: Granularity,
    pub compatibility: RequiredCompatibility,
    pub minimum_label_authority: LabelAuthority,
    pub consumer_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub owner_generation: u64,
    pub grant_ref: String,
    pub grant: GrantFacts,
    pub claims: Vec<Claim>,
    /// Existing deployment-owned allocation; its physical claims are not copied.
    pub serving_allocation_id: Option<String>,
    pub kind: AllocationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationKind {
    Execution,
    ServingDeployment,
}

/// Grant authority is supplied by the admitted resource owner, never inferred from a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantFacts {
    pub grant_ref: String,
    pub consumer_id: String,
    pub is_valid: bool,
    pub is_revoked: bool,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationStatus {
    Intent,
    Active,
    Quarantined,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementStrength {
    LogicalFenceOnly,
    BackendAcknowledged,
    BackendFenced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocation {
    pub schema: String,
    pub id: String,
    pub host_id: String,
    pub host_incarnation: String,
    pub device_incarnation: String,
    pub offer_revision: u64,
    pub topology_generation: u64,
    /// Monotonic charge identity assigned on reservation, including across releases.
    pub charge_cursor: u64,
    /// Reservation logical time, used to reject a basis falsely claiming future charges.
    pub charge_at: u64,
    pub valid_until: u64,
    pub authority_ids: Vec<String>,
    pub view_id: String,
    pub granularity: Granularity,
    pub consumer_id: String,
    pub operation_id: String,
    pub attempt_id: String,
    pub owner_generation: u64,
    pub grant_ref: String,
    pub claims: Vec<Claim>,
    pub serving_allocation_id: Option<String>,
    pub kind: AllocationKind,
    pub status: ReservationStatus,
    pub enforcement: EnforcementStrength,
    /// Exact committed activation evidence identity, retained for reconciliation replay.
    pub activation_backend_ref: Option<String>,
}

/// A shell persists the returned intent before executing any protected effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectIntent {
    PersistReservation(String),
    Activate(String),
    RequestBackendRelease(String),
    PersistRelease(String),
    Reconcile(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapacityState {
    pub offers: BTreeMap<String, Offer>,
    pub allocations: BTreeMap<String, Allocation>,
    /// Durable charge sequence; a delayed offer cannot acquire a newer charge basis on delivery.
    pub charge_cursor: u64,
    /// Released records retained in FIFO order for a bounded idempotency horizon.
    /// Once evicted, an operation ID is unknown and can be admitted again by a fresh reserve.
    pub retired_allocations: VecDeque<Allocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityIssue {
    InvalidInput,
    Stale,
    Unavailable,
    Unsupported,
    InsufficientEvidence,
    InsufficientCapacity,
    Incompatible,
    WeakAuthority,
    RevokedGrant,
    InvalidGrant,
    WrongOwner,
    UncertainRelease,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eligibility {
    Eligible,
    Rejected(CapacityIssue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationOutcome {
    Reserved {
        allocation: Allocation,
        effects: Vec<EffectIntent>,
    },
    Existing {
        allocation: Allocation,
    },
    Rejected(CapacityIssue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionOutcome {
    pub state: CapacityState,
    pub decision: ReservationOutcome,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT && !value.bytes().any(|byte| byte.is_ascii_control())
}

fn bounded_unique<'a>(ids: impl IntoIterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !valid_id(id) || !seen.insert(id) || seen.len() > MAX_ITEMS {
            return false;
        }
    }
    true
}

// r[impl molten.compute_capacity.offers]
pub fn validate_offer(offer: &Offer) -> Result<(), CapacityIssue> {
    if offer.schema != OFFER_SCHEMA
        || ![
            &offer.host_id,
            &offer.host_incarnation,
            &offer.device_incarnation,
            &offer.provider_profile_ref,
            &offer.source_profile_ref,
        ]
        .into_iter()
        .all(|id| valid_id(id))
        || offer.revision == 0
        || offer.topology_generation == 0
        || offer.expires_at <= offer.observed_at
        || offer.pools.is_empty()
        || offer.views.is_empty()
        || offer.pools.len() > MAX_ITEMS
        || offer.views.len() > MAX_ITEMS
        || !bounded_unique(offer.pools.iter().map(|pool| pool.id.as_str()))
        || !bounded_unique(offer.views.iter().map(|view| view.id.as_str()))
        || offer.compatibility.qualification_refs.len() > MAX_ITEMS
        || !bounded_unique(offer.compatibility.qualification_refs.iter().map(String::as_str))
        || [
            offer.compatibility.backend.as_deref(),
            offer.compatibility.runtime.as_deref(),
            offer.compatibility.architecture.as_deref(),
            offer.compatibility.topology.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|id| !valid_id(id))
    {
        return Err(CapacityIssue::InvalidInput);
    }
    for pool in &offer.pools {
        if !valid_id(&pool.authority_id)
            || pool.allocatable > pool.total
            || pool.observed_used.is_some_and(|used| used > pool.total)
            || pool.observed_free.is_some_and(|free| free > pool.total)
            || matches!((pool.observed_used, pool.observed_free), (Some(used), Some(free)) if used.checked_add(free).is_none_or(|sum| sum > pool.total))
        {
            return Err(CapacityIssue::InvalidInput);
        }
    }
    for view in &offer.views {
        if !valid_id(&view.device_id)
            || view.partition_id.as_deref().is_some_and(|id| !valid_id(id))
            || view.backing_pools.is_empty()
            || view.backing_pools.len() > MAX_ITEMS
            || !bounded_unique(view.backing_pools.iter().map(String::as_str))
            || view.backing_pools.iter().any(|id| !offer.pools.iter().any(|pool| pool.id == *id))
        {
            return Err(CapacityIssue::InvalidInput);
        }
    }
    Ok(())
}

// r[impl molten.compute_capacity.offers]
pub fn observe(state: &CapacityState, offer: Offer) -> Result<CapacityState, CapacityIssue> {
    validate_offer(&offer)?;
    if offer.observation_charge_cursor.is_some_and(|cursor| cursor > state.charge_cursor) {
        return Err(CapacityIssue::InsufficientEvidence);
    }
    if state.offers.len() >= MAX_ITEMS && !state.offers.contains_key(&offer.host_id) {
        return Err(CapacityIssue::InvalidInput);
    }
    if state.allocations.values().any(|allocation| {
        allocation.host_id == offer.host_id
            && offer.observation_charge_cursor.is_some_and(|cursor| allocation.charge_cursor <= cursor)
            && allocation.charge_at > offer.observed_at
    }) {
        return Err(CapacityIssue::InsufficientEvidence);
    }
    if let Some(previous) = state.offers.get(&offer.host_id) {
        if offer.revision <= previous.revision || offer.observed_at < previous.observed_at {
            return Err(CapacityIssue::Stale);
        }
        // Physical pool authorities and identities cannot silently change while charged.
        for allocation in state
            .allocations
            .values()
            .filter(|a| a.host_id == offer.host_id && a.status != ReservationStatus::Released)
        {
            for claim in &allocation.claims {
                let Some(old) = previous.pools.iter().find(|pool| pool.id == claim.pool_id) else {
                    return Err(CapacityIssue::InvalidInput);
                };
                let Some(new) = offer.pools.iter().find(|pool| pool.id == claim.pool_id) else {
                    return Err(CapacityIssue::Unavailable);
                };
                if old.authority_id != new.authority_id || old.kind != new.kind {
                    return Err(CapacityIssue::Unsupported);
                }
            }
        }
    }
    let mut next = state.clone();
    next.offers.insert(offer.host_id.clone(), offer);
    Ok(next)
}

fn compatibility_issue(actual: &Compatibility, required: &RequiredCompatibility) -> Option<CapacityIssue> {
    for (required, actual) in [
        (&required.backend, &actual.backend),
        (&required.runtime, &actual.runtime),
        (&required.architecture, &actual.architecture),
        (&required.topology, &actual.topology),
    ] {
        if let Some(required) = required {
            match actual {
                None => return Some(CapacityIssue::InsufficientEvidence),
                Some(actual) if actual != required => return Some(CapacityIssue::Incompatible),
                _ => {}
            }
        }
    }
    if actual.qualification_refs.is_empty() {
        return Some(CapacityIssue::InsufficientEvidence);
    }
    None
}

fn charged(state: &CapacityState, host_id: &str, pool_id: &str) -> Option<u64> {
    charged_since(state, host_id, pool_id, None)
}

fn charged_since(state: &CapacityState, host_id: &str, pool_id: &str, basis: Option<u64>) -> Option<u64> {
    state
        .allocations
        .values()
        .filter(|a| a.host_id == host_id && a.status != ReservationStatus::Released)
        .filter(|a| basis.is_none_or(|cursor| a.charge_cursor > cursor))
        .try_fold(0_u64, |sum, a| {
            a.claims
                .iter()
                .filter(|claim| claim.pool_id == pool_id)
                .try_fold(sum, |sum, claim| sum.checked_add(claim.quantity))
        })
}

fn available(pool: &Pool, charged: u64, unobserved_charged: u64) -> Result<u64, CapacityIssue> {
    let observed = match (pool.observed_free, pool.observed_used) {
        (Some(free), _) => free,
        (None, Some(used)) => pool.total.checked_sub(used).ok_or(CapacityIssue::InvalidInput)?,
        (None, None) => return Err(CapacityIssue::InsufficientEvidence),
    };
    let policy_left = pool.allocatable.saturating_sub(charged);
    let observed_left = observed.saturating_sub(unobserved_charged);
    Ok(policy_left.min(observed_left))
}

// r[impl molten.compute_capacity.backing_pools]
// r[impl molten.compute_capacity.reservation_authority]
pub fn eligibility(state: &CapacityState, input: &ReservationInput) -> Eligibility {
    let Some(offer) = state.offers.get(&input.host_id) else {
        return Eligibility::Rejected(CapacityIssue::Unavailable);
    };
    if validate_offer(offer).is_err()
        || input.claims.is_empty()
        || input.claims.len() > MAX_ITEMS
        || !bounded_unique(input.claims.iter().map(|claim| claim.pool_id.as_str()))
        || ![
            &input.consumer_id,
            &input.operation_id,
            &input.attempt_id,
            &input.grant_ref,
            &input.view_id,
            &input.host_incarnation,
            &input.device_incarnation,
        ]
        .into_iter()
        .all(|id| valid_id(id))
        || input.serving_allocation_id.as_deref().is_some_and(|id| !valid_id(id))
        || [
            input.compatibility.backend.as_deref(),
            input.compatibility.runtime.as_deref(),
            input.compatibility.architecture.as_deref(),
            input.compatibility.topology.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|id| !valid_id(id))
        || input.owner_generation == 0
        || input.claims.iter().any(|claim| claim.quantity == 0)
    {
        return Eligibility::Rejected(CapacityIssue::InvalidInput);
    }
    if input.host_incarnation != offer.host_incarnation
        || input.device_incarnation != offer.device_incarnation
        || input.offer_revision != offer.revision
        || input.topology_generation != offer.topology_generation
        || input.now < offer.observed_at
        || input.now >= offer.expires_at
    {
        return Eligibility::Rejected(CapacityIssue::Stale);
    }
    if offer.health != Health::Healthy {
        return Eligibility::Rejected(CapacityIssue::Unavailable);
    }
    let Some(view) = offer.views.iter().find(|view| view.id == input.view_id) else {
        return Eligibility::Rejected(CapacityIssue::Unsupported);
    };
    if view.granularity != input.granularity || !view.is_grouping_enforced {
        return Eligibility::Rejected(CapacityIssue::Unsupported);
    }
    if let Some(issue) = compatibility_issue(&offer.compatibility, &input.compatibility) {
        return Eligibility::Rejected(issue);
    }
    if offer.label_authority < input.minimum_label_authority {
        return Eligibility::Rejected(CapacityIssue::WeakAuthority);
    }
    if input.grant.is_revoked {
        return Eligibility::Rejected(CapacityIssue::RevokedGrant);
    }
    if !input.grant.is_valid
        || input.grant.grant_ref != input.grant_ref
        || input.grant.consumer_id != input.consumer_id
        || input.now >= input.grant.expires_at
    {
        return Eligibility::Rejected(CapacityIssue::InvalidGrant);
    }
    if let Some(serving_id) = &input.serving_allocation_id {
        let Some(serving) = state.allocations.get(serving_id) else {
            return Eligibility::Rejected(CapacityIssue::InsufficientEvidence);
        };
        if input.kind != AllocationKind::Execution
            || serving.kind != AllocationKind::ServingDeployment
            || serving.status == ReservationStatus::Released
        {
            return Eligibility::Rejected(CapacityIssue::InsufficientEvidence);
        }
    }
    for claim in &input.claims {
        if !view.backing_pools.contains(&claim.pool_id) {
            return Eligibility::Rejected(CapacityIssue::Unsupported);
        }
        let Some(pool) = offer.pools.iter().find(|pool| pool.id == claim.pool_id) else {
            return Eligibility::Rejected(CapacityIssue::Unsupported);
        };
        let Some(used) = charged(state, &input.host_id, &claim.pool_id) else {
            return Eligibility::Rejected(CapacityIssue::InsufficientEvidence);
        };
        let basis = if pool.is_observation_debited {
            offer.observation_charge_cursor
        } else {
            None
        };
        let Some(unobserved) = charged_since(state, &input.host_id, &claim.pool_id, basis) else {
            return Eligibility::Rejected(CapacityIssue::InsufficientEvidence);
        };
        match available(pool, used, unobserved) {
            Ok(free) if free < claim.quantity => return Eligibility::Rejected(CapacityIssue::InsufficientCapacity),
            Ok(_) => {}
            Err(issue) => return Eligibility::Rejected(issue),
        }
    }
    // Every physical chip in a supported grouping must be claimed; omitting one
    // must not turn a tray/mesh into an unqualified partial grouping.
    if view.backing_pools.iter().any(|pool_id| !input.claims.iter().any(|claim| &claim.pool_id == pool_id)) {
        return Eligibility::Rejected(CapacityIssue::Unsupported);
    }
    Eligibility::Eligible
}

// r[impl molten.compute_capacity.reservation_authority]
pub fn reserve(state: &CapacityState, input: &ReservationInput) -> TransitionOutcome {
    if let Some(allocation) = state.allocations.values().find(|a| a.operation_id == input.operation_id) {
        let decision = if allocation.consumer_id == input.consumer_id
            && allocation.attempt_id == input.attempt_id
            && allocation.owner_generation == input.owner_generation
            && allocation.status != ReservationStatus::Released
            && allocation.host_id == input.host_id
            && allocation.host_incarnation == input.host_incarnation
            && allocation.device_incarnation == input.device_incarnation
            && allocation.offer_revision == input.offer_revision
            && allocation.topology_generation == input.topology_generation
            && allocation.view_id == input.view_id
            && allocation.granularity == input.granularity
            && allocation.grant_ref == input.grant_ref
            && allocation.claims == input.claims
            && allocation.kind == input.kind
            && allocation.serving_allocation_id == input.serving_allocation_id
        {
            ReservationOutcome::Existing {
                allocation: allocation.clone(),
            }
        } else {
            ReservationOutcome::Rejected(CapacityIssue::WrongOwner)
        };
        return TransitionOutcome {
            state: state.clone(),
            decision,
        };
    }
    if state.retired_allocations.iter().any(|a| a.operation_id == input.operation_id) {
        return TransitionOutcome {
            state: state.clone(),
            decision: ReservationOutcome::Rejected(CapacityIssue::WrongOwner),
        };
    }
    if let Eligibility::Rejected(issue) = eligibility(state, input) {
        return TransitionOutcome {
            state: state.clone(),
            decision: ReservationOutcome::Rejected(issue),
        };
    }
    if state.allocations.len() >= MAX_ITEMS {
        return TransitionOutcome {
            state: state.clone(),
            decision: ReservationOutcome::Rejected(CapacityIssue::InvalidInput),
        };
    }
    let Some(charge_cursor) = state.charge_cursor.checked_add(1) else {
        return TransitionOutcome {
            state: state.clone(),
            decision: ReservationOutcome::Rejected(CapacityIssue::InvalidInput),
        };
    };
    let id = input.operation_id.clone();
    let offer = &state.offers[&input.host_id];
    let allocation = Allocation {
        schema: RESERVATION_SCHEMA.into(),
        id: id.clone(),
        host_id: input.host_id.clone(),
        host_incarnation: input.host_incarnation.clone(),
        device_incarnation: input.device_incarnation.clone(),
        offer_revision: input.offer_revision,
        topology_generation: input.topology_generation,
        charge_cursor,
        charge_at: input.now,
        valid_until: input.grant.expires_at.min(offer.expires_at),
        authority_ids: input
            .claims
            .iter()
            .map(|claim| {
                offer
                    .pools
                    .iter()
                    .find(|pool| pool.id == claim.pool_id)
                    .expect("eligibility checked pool")
                    .authority_id
                    .clone()
            })
            .collect(),
        view_id: input.view_id.clone(),
        granularity: input.granularity,
        consumer_id: input.consumer_id.clone(),
        operation_id: id.clone(),
        attempt_id: input.attempt_id.clone(),
        owner_generation: input.owner_generation,
        grant_ref: input.grant_ref.clone(),
        claims: input.claims.clone(),
        serving_allocation_id: input.serving_allocation_id.clone(),
        kind: input.kind,
        status: ReservationStatus::Intent,
        enforcement: EnforcementStrength::LogicalFenceOnly,
        activation_backend_ref: None,
    };
    let mut next = state.clone();
    next.charge_cursor = charge_cursor;
    next.allocations.insert(id.clone(), allocation.clone());
    TransitionOutcome {
        state: next,
        decision: ReservationOutcome::Reserved {
            allocation,
            effects: vec![EffectIntent::PersistReservation(id.clone()), EffectIntent::Activate(id)],
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Drain,
    Cancel,
    Timeout,
    LostStopAcknowledgement,
    BackendDisappeared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerInput {
    pub allocation_id: String,
    pub consumer_id: String,
    pub owner_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleOutcome {
    pub state: CapacityState,
    pub effects: Vec<EffectIntent>,
    pub issue: Option<CapacityIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationEvidence {
    pub allocation_id: String,
    pub owner_generation: u64,
    pub backend_ref: String,
    pub is_active: bool,
    pub enforcement: EnforcementStrength,
}

// r[impl molten.compute_capacity.reservation_authority]
pub fn acknowledge_activation(
    state: &CapacityState,
    owner: &OwnerInput,
    evidence: &ActivationEvidence,
) -> LifecycleOutcome {
    let Some(existing) = state.allocations.get(&owner.allocation_id) else {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::WrongOwner),
        };
    };
    if existing.consumer_id != owner.consumer_id || existing.owner_generation != owner.owner_generation {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::WrongOwner),
        };
    }
    if existing.status == ReservationStatus::Active {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: if evidence.allocation_id == existing.id
                && evidence.owner_generation == existing.owner_generation
                && evidence.is_active
                && evidence.enforcement == existing.enforcement
                && existing.activation_backend_ref.as_deref() == Some(evidence.backend_ref.as_str())
            {
                None
            } else {
                Some(CapacityIssue::InsufficientEvidence)
            },
        };
    }
    if existing.status != ReservationStatus::Intent
        || evidence.allocation_id != existing.id
        || evidence.owner_generation != owner.owner_generation
        || !valid_id(&evidence.backend_ref)
        || !evidence.is_active
        || evidence.enforcement == EnforcementStrength::LogicalFenceOnly
    {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::InsufficientEvidence),
        };
    }
    let mut next = state.clone();
    let allocation = next.allocations.get_mut(&owner.allocation_id).expect("previously found allocation");
    allocation.status = ReservationStatus::Active;
    allocation.enforcement = evidence.enforcement;
    allocation.activation_backend_ref = Some(evidence.backend_ref.clone());
    LifecycleOutcome {
        state: next,
        effects: vec![],
        issue: None,
    }
}

fn retired_owner_outcome(state: &CapacityState, owner: &OwnerInput) -> LifecycleOutcome {
    let is_owner = state.retired_allocations.iter().any(|allocation| {
        allocation.id == owner.allocation_id
            && allocation.consumer_id == owner.consumer_id
            && allocation.owner_generation == owner.owner_generation
    });
    LifecycleOutcome {
        state: state.clone(),
        effects: vec![],
        issue: if is_owner {
            None
        } else {
            Some(CapacityIssue::WrongOwner)
        },
    }
}

fn retire(state: &CapacityState, owner: &OwnerInput, enforcement: EnforcementStrength) -> LifecycleOutcome {
    let mut next = state.clone();
    let mut allocation = next.allocations.remove(&owner.allocation_id).expect("previously found allocation");
    allocation.status = ReservationStatus::Released;
    allocation.enforcement = enforcement;
    if next.retired_allocations.len() == MAX_ITEMS {
        next.retired_allocations.pop_front();
    }
    next.retired_allocations.push_back(allocation);
    LifecycleOutcome {
        state: next,
        effects: vec![EffectIntent::PersistRelease(owner.allocation_id.clone())],
        issue: None,
    }
}

// r[impl molten.compute_capacity.uncertainty_release]
pub fn lifecycle(state: &CapacityState, owner: &OwnerInput, event: LifecycleEvent) -> LifecycleOutcome {
    let Some(existing) = state.allocations.get(&owner.allocation_id) else {
        return retired_owner_outcome(state, owner);
    };
    if existing.consumer_id != owner.consumer_id || existing.owner_generation != owner.owner_generation {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::WrongOwner),
        };
    }
    let mut next = state.clone();
    let allocation = next.allocations.get_mut(&owner.allocation_id).expect("previously found allocation");
    let effects = match event {
        LifecycleEvent::Drain | LifecycleEvent::Cancel => {
            allocation.status = ReservationStatus::Quarantined;
            vec![EffectIntent::RequestBackendRelease(owner.allocation_id.clone())]
        }
        LifecycleEvent::Timeout | LifecycleEvent::LostStopAcknowledgement | LifecycleEvent::BackendDisappeared => {
            allocation.status = ReservationStatus::Quarantined;
            vec![EffectIntent::Reconcile(owner.allocation_id.clone())]
        }
    };
    LifecycleOutcome {
        state: next,
        effects,
        issue: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInput {
    pub owner: OwnerInput,
    pub backend_evidence: Option<BackendReleaseEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendReleaseEvidence {
    pub allocation_id: String,
    pub owner_generation: u64,
    pub backend_ref: String,
    pub is_released: bool,
    pub enforcement: EnforcementStrength,
}

// r[impl molten.compute_capacity.uncertainty_release]
pub fn release(state: &CapacityState, input: &ReleaseInput) -> LifecycleOutcome {
    let owner = &input.owner;
    let Some(existing) = state.allocations.get(&owner.allocation_id) else {
        return retired_owner_outcome(state, owner);
    };
    if existing.consumer_id != owner.consumer_id || existing.owner_generation != owner.owner_generation {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::WrongOwner),
        };
    }
    let Some(evidence) = &input.backend_evidence else {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::UncertainRelease),
        };
    };
    if evidence.allocation_id != existing.id
        || evidence.owner_generation != existing.owner_generation
        || !valid_id(&evidence.backend_ref)
        || !evidence.is_released
        || evidence.enforcement == EnforcementStrength::LogicalFenceOnly
    {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::UncertainRelease),
        };
    }
    // Serving allocations cannot be freed while live execution allocations reference them.
    if existing.kind == AllocationKind::ServingDeployment
        && state.allocations.values().any(|a| {
            a.serving_allocation_id.as_deref() == Some(&existing.id) && a.status != ReservationStatus::Released
        })
    {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::UncertainRelease),
        };
    }
    retire(state, owner, evidence.enforcement)
}

/// A definitive backend refusal before any effect frees an intent without fabricating release
/// evidence.
// r[impl molten.compute_capacity.uncertainty_release]
pub fn abort_unstarted_intent(state: &CapacityState, owner: &OwnerInput, explicit_no_effect: bool) -> LifecycleOutcome {
    let Some(existing) = state.allocations.get(&owner.allocation_id) else {
        return retired_owner_outcome(state, owner);
    };
    if existing.consumer_id != owner.consumer_id || existing.owner_generation != owner.owner_generation {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::WrongOwner),
        };
    }
    if !explicit_no_effect
        || existing.status != ReservationStatus::Intent
        || (existing.kind == AllocationKind::ServingDeployment
            && state.allocations.values().any(|a| a.serving_allocation_id.as_deref() == Some(&existing.id)))
    {
        return LifecycleOutcome {
            state: state.clone(),
            effects: vec![],
            issue: Some(CapacityIssue::UncertainRelease),
        };
    }
    retire(state, owner, EnforcementStrength::LogicalFenceOnly)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapacityReadback {
    pub host_id: String,
    pub revision: u64,
    pub is_fresh: bool,
    pub health: Health,
    pub qualification_refs: Vec<String>,
    pub allocations: Vec<AllocationReadback>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationReadback {
    pub id: String,
    pub consumer_id: String,
    pub owner_generation: u64,
    pub granularity: Granularity,
    pub status: ReservationStatus,
    pub is_logically_fenced: bool,
    pub enforcement: EnforcementStrength,
}

// r[impl molten.compute_capacity.readback]
pub fn inspect(state: &CapacityState, host_id: &str, now: u64) -> Option<CapacityReadback> {
    if !valid_id(host_id) || state.allocations.len() > MAX_ITEMS {
        return None;
    }
    let offer = state.offers.get(host_id)?;
    Some(CapacityReadback {
        host_id: host_id.into(),
        revision: offer.revision,
        is_fresh: now >= offer.observed_at && now < offer.expires_at,
        health: offer.health,
        qualification_refs: offer.compatibility.qualification_refs.clone(),
        allocations: state
            .allocations
            .values()
            .filter(|a| a.host_id == host_id)
            .map(|a| AllocationReadback {
                id: a.id.clone(),
                consumer_id: a.consumer_id.clone(),
                owner_generation: a.owner_generation,
                granularity: a.granularity,
                status: a.status,
                is_logically_fenced: a.status != ReservationStatus::Released,
                enforcement: a.enforcement,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests;
