use super::*;

const GOLDEN_READBACK_REF: &str = "blake3:d23f8d4acdd511f6b9b85deaa076a7e42d8084cd49ab666ede920b711713fc11";
const ZERO_REF: &str = "blake3:0000000000000000000000000000000000000000000000000000000000000000";

fn sample_offer() -> molten_core::compute_capacity::Offer {
    molten_core::compute_capacity::Offer {
        schema: molten_core::compute_capacity::OFFER_SCHEMA.into(),
        host_id: "host-a".into(),
        host_incarnation: "host-1".into(),
        device_incarnation: "device-1".into(),
        provider_profile_ref: ZERO_REF.into(),
        source_profile_ref: ZERO_REF.into(),
        revision: 1,
        topology_generation: 1,
        observed_at: 10,
        observation_charge_cursor: Some(0),
        expires_at: 100,
        health: molten_core::compute_capacity::Health::Healthy,
        label_authority: molten_core::fabric_membership::LabelAuthority::Authoritative,
        compatibility: molten_core::compute_capacity::Compatibility {
            backend: Some("backend-a".into()),
            runtime: None,
            architecture: None,
            topology: None,
            qualification_refs: vec![ZERO_REF.into()],
        },
        pools: vec![molten_core::compute_capacity::Pool {
            id: "chip".into(),
            kind: molten_core::compute_capacity::PoolKind::Chips,
            authority_id: "authority-chip".into(),
            total: 1,
            allocatable: 1,
            observed_used: None,
            observed_free: Some(1),
            is_observation_debited: false,
        }],
        views: vec![molten_core::compute_capacity::View {
            id: "device".into(),
            device_id: "device-1".into(),
            partition_id: None,
            granularity: molten_core::compute_capacity::Granularity::Chip,
            backing_pools: vec!["chip".into()],
            is_grouping_enforced: true,
        }],
    }
}

fn sample_allocation() -> molten_core::compute_capacity::Allocation {
    molten_core::compute_capacity::Allocation {
        schema: molten_core::compute_capacity::RESERVATION_SCHEMA.into(),
        id: "alloc-a".into(),
        host_id: "host-a".into(),
        host_incarnation: "host-1".into(),
        device_incarnation: "device-1".into(),
        offer_revision: 1,
        topology_generation: 1,
        charge_cursor: 1,
        charge_at: 20,
        valid_until: 100,
        authority_ids: vec!["authority-chip".into()],
        view_id: "device".into(),
        granularity: molten_core::compute_capacity::Granularity::Chip,
        consumer_id: "consumer-a".into(),
        operation_id: "alloc-a".into(),
        attempt_id: "attempt-a".into(),
        owner_generation: 1,
        grant_ref: ZERO_REF.into(),
        claims: vec![molten_core::compute_capacity::Claim {
            pool_id: "chip".into(),
            quantity: 1,
        }],
        serving_allocation_id: None,
        kind: molten_core::compute_capacity::AllocationKind::Execution,
        status: molten_core::compute_capacity::ReservationStatus::Intent,
        enforcement: molten_core::compute_capacity::EnforcementStrength::LogicalFenceOnly,
        activation_backend_ref: None,
    }
}

fn sample_readback() -> molten_core::compute_capacity::CapacityReadback {
    molten_core::compute_capacity::CapacityReadback {
        host_id: "host-a".into(),
        revision: 1,
        is_fresh: true,
        health: molten_core::compute_capacity::Health::Healthy,
        qualification_refs: vec![ZERO_REF.into()],
        allocations: vec![molten_core::compute_capacity::AllocationReadback {
            id: "alloc-a".into(),
            consumer_id: "consumer-a".into(),
            owner_generation: 1,
            granularity: molten_core::compute_capacity::Granularity::Chip,
            status: molten_core::compute_capacity::ReservationStatus::Intent,
            is_logically_fenced: true,
            enforcement: molten_core::compute_capacity::EnforcementStrength::LogicalFenceOnly,
        }],
    }
}

fn cloned_fields(value: &preserves::IOValue, label: &str, arity: u64) -> Vec<preserves::IOValue> {
    fields(value.as_ref(), label, arity)
        .expect("record fields")
        .fields_iter()
        .map(crate::preserves_rail::value_to_iovalue)
        .collect()
}

fn replace(
    value: &preserves::IOValue,
    label: &'static str,
    arity: u64,
    index: usize,
    replacement: preserves::IOValue,
) -> preserves::IOValue {
    let mut values = cloned_fields(value, label, arity);
    values[index] = replacement;
    crate::preserves_rail::record(label, values)
}

include!("tests/accepted.rs");
include!("tests/denials.rs");
