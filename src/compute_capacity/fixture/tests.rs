//! Authored Nickel fixtures exercised against the pure core and durable canonical owner.

use super::ports::ComputeOfferPort;
use super::ports::ComputeStorePort;

fn export_authored(name: &str) -> serde_json::Value {
    let path = format!("config/compute-capacity/fixtures/positive/{name}");
    let output = std::process::Command::new("nickel")
        .args(["export", "--format", "json", &path])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("pinned Nickel must be available to exercise authored compute fixtures");
    assert!(output.status.success(), "Nickel rejected {path}: {}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).expect("Nickel fixture export is JSON")
}

fn field<'a>(value: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    value.get(name).unwrap_or_else(|| panic!("missing authored field {name}"))
}

fn text(value: &serde_json::Value) -> String {
    value.as_str().expect("authored text").to_owned()
}

fn optional_text(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn number(value: &serde_json::Value) -> u64 {
    value.as_u64().expect("authored unsigned integer")
}

fn entries(value: &serde_json::Value) -> &[serde_json::Value] {
    value.as_array().expect("authored array")
}

fn texts(value: &serde_json::Value) -> Vec<String> {
    entries(value).iter().map(text).collect()
}

fn granularity(value: &serde_json::Value) -> molten_core::compute_capacity::Granularity {
    match value.as_str().expect("authored granularity") {
        "chip" => molten_core::compute_capacity::Granularity::Chip,
        "mmio-bundle" => molten_core::compute_capacity::Granularity::MmioBundle,
        "tray" => molten_core::compute_capacity::Granularity::Tray,
        "mesh" => molten_core::compute_capacity::Granularity::Mesh,
        "partition" => molten_core::compute_capacity::Granularity::Partition,
        "alias" => molten_core::compute_capacity::Granularity::Alias,
        other => panic!("unknown authored granularity {other}"),
    }
}

fn label_authority(value: &serde_json::Value) -> molten_core::fabric_membership::LabelAuthority {
    match value.as_str().expect("authored label authority") {
        "advisory" => molten_core::fabric_membership::LabelAuthority::Advisory,
        "measured" => molten_core::fabric_membership::LabelAuthority::Measured,
        "operator-declared" => molten_core::fabric_membership::LabelAuthority::OperatorDeclared,
        "authoritative" => molten_core::fabric_membership::LabelAuthority::Authoritative,
        other => panic!("unknown authored label authority {other}"),
    }
}

fn compatibility(value: &serde_json::Value) -> molten_core::compute_capacity::Compatibility {
    molten_core::compute_capacity::Compatibility {
        backend: optional_text(field(value, "backend")),
        runtime: optional_text(field(value, "runtime")),
        architecture: optional_text(field(value, "architecture")),
        topology: optional_text(field(value, "topology")),
        qualification_refs: texts(field(value, "qualification_refs")),
    }
}

fn offer(value: &serde_json::Value) -> molten_core::compute_capacity::Offer {
    let health = match field(value, "health").as_str().expect("authored health") {
        "healthy" => molten_core::compute_capacity::Health::Healthy,
        "degraded" => molten_core::compute_capacity::Health::Degraded,
        "unavailable" => molten_core::compute_capacity::Health::Unavailable,
        "unknown" => molten_core::compute_capacity::Health::Unknown,
        other => panic!("unknown authored health {other}"),
    };
    molten_core::compute_capacity::Offer {
        schema: text(field(value, "schema")),
        host_id: text(field(value, "host_id")),
        host_incarnation: text(field(value, "host_incarnation")),
        device_incarnation: text(field(value, "device_incarnation")),
        provider_profile_ref: text(field(value, "provider_profile_ref")),
        source_profile_ref: text(field(value, "source_profile_ref")),
        revision: number(field(value, "revision")),
        topology_generation: number(field(value, "topology_generation")),
        observed_at: number(field(value, "observed_at")),
        observation_charge_cursor: Some(number(field(value, "observation_charge_cursor"))),
        expires_at: number(field(value, "expires_at")),
        health,
        label_authority: label_authority(field(value, "label_authority")),
        compatibility: compatibility(field(value, "compatibility")),
        pools: entries(field(value, "pools"))
            .iter()
            .map(|pool| molten_core::compute_capacity::Pool {
                id: text(field(pool, "id")),
                kind: match field(pool, "kind").as_str().expect("authored pool kind") {
                    "chips" => molten_core::compute_capacity::PoolKind::Chips,
                    "dedicated-memory-bytes" => molten_core::compute_capacity::PoolKind::DedicatedMemoryBytes,
                    "unified-memory-bytes" => molten_core::compute_capacity::PoolKind::UnifiedMemoryBytes,
                    other => panic!("unknown authored pool kind {other}"),
                },
                authority_id: text(field(pool, "authority_id")),
                total: number(field(pool, "total")),
                allocatable: number(field(pool, "allocatable")),
                observed_used: field(pool, "observed_used").as_u64(),
                observed_free: field(pool, "observed_free").as_u64(),
                is_observation_debited: field(pool, "is_observation_debited").as_bool().expect("debit basis"),
            })
            .collect(),
        views: entries(field(value, "views"))
            .iter()
            .map(|view| molten_core::compute_capacity::View {
                id: text(field(view, "id")),
                device_id: text(field(view, "device_id")),
                partition_id: optional_text(field(view, "partition_id")),
                granularity: granularity(field(view, "granularity")),
                backing_pools: texts(field(view, "backing_pools")),
                is_grouping_enforced: field(view, "is_grouping_enforced").as_bool().expect("group enforcement"),
            })
            .collect(),
    }
}

fn request(value: &serde_json::Value) -> molten_core::compute_capacity::ReservationInput {
    let required = field(value, "compatibility");
    let grant = field(value, "grant");
    molten_core::compute_capacity::ReservationInput {
        host_id: text(field(value, "host_id")),
        host_incarnation: text(field(value, "host_incarnation")),
        device_incarnation: text(field(value, "device_incarnation")),
        offer_revision: number(field(value, "offer_revision")),
        topology_generation: number(field(value, "topology_generation")),
        now: number(field(value, "now")),
        view_id: text(field(value, "view_id")),
        granularity: granularity(field(value, "granularity")),
        compatibility: molten_core::compute_capacity::RequiredCompatibility {
            backend: optional_text(field(required, "backend")),
            runtime: optional_text(field(required, "runtime")),
            architecture: optional_text(field(required, "architecture")),
            topology: optional_text(field(required, "topology")),
        },
        minimum_label_authority: label_authority(field(value, "minimum_label_authority")),
        consumer_id: text(field(value, "consumer_id")),
        operation_id: text(field(value, "operation_id")),
        attempt_id: text(field(value, "attempt_id")),
        owner_generation: number(field(value, "owner_generation")),
        grant_ref: text(field(value, "grant_ref")),
        grant: molten_core::compute_capacity::GrantFacts {
            grant_ref: text(field(grant, "grant_ref")),
            consumer_id: text(field(grant, "consumer_id")),
            is_valid: field(grant, "is_valid").as_bool().expect("grant validity"),
            is_revoked: field(grant, "is_revoked").as_bool().expect("grant revocation"),
            expires_at: number(field(grant, "expires_at")),
        },
        claims: entries(field(value, "claims"))
            .iter()
            .map(|claim| molten_core::compute_capacity::Claim {
                pool_id: text(field(claim, "pool_id")),
                quantity: number(field(claim, "quantity")),
            })
            .collect(),
        serving_allocation_id: optional_text(field(value, "serving_allocation_id")),
        kind: match field(value, "kind").as_str().expect("authored allocation kind") {
            "execution" => molten_core::compute_capacity::AllocationKind::Execution,
            "serving-deployment" => molten_core::compute_capacity::AllocationKind::ServingDeployment,
            other => panic!("unknown authored allocation kind {other}"),
        },
    }
}

fn request_for_view(
    base: &molten_core::compute_capacity::ReservationInput,
    offer: &molten_core::compute_capacity::Offer,
    view_id: &str,
) -> molten_core::compute_capacity::ReservationInput {
    let view = offer.views.iter().find(|view| view.id == view_id).expect("authored TT view");
    let mut candidate = base.clone();
    candidate.operation_id = format!("attempt-{view_id}");
    candidate.attempt_id = format!("attempt-{view_id}");
    candidate.view_id = view.id.clone();
    candidate.granularity = view.granularity;
    candidate.claims = view
        .backing_pools
        .iter()
        .map(|pool_id| molten_core::compute_capacity::Claim {
            pool_id: pool_id.clone(),
            quantity: 1,
        })
        .collect();
    candidate
}

fn store_root(label: &str) -> (crate::test_support::Workspace, crate::local_store::DurableStoreRoot) {
    let workspace = crate::test_support::Workspace::new(label).expect("owned workspace");
    let state = workspace.state().expect("state capability");
    let root = crate::node_state::DirectoryRoot::from_dir(state.dir().try_clone().expect("clone capability"));
    root.create_layout().expect("node layout");
    let store = crate::local_store::DurableStoreRoot::from_ledger(
        &root.ledger_store().expect("ledger capability"),
        &crate::local_store::Locator::parse("compute-tt-fixture-owner").expect("relative namespace"),
    )
    .expect("compute owner capability");
    (workspace, store)
}

// r[verify molten.compute_capacity.fixtures]
// r[verify molten.compute_capacity.offers]
// r[verify molten.compute_capacity.backing_pools]
#[test]
fn authored_two_host_is_core_eligible_and_projected() {
    let fixture = export_authored("agent-a-serving-b.ncl");
    let mut state = molten_core::compute_capacity::CapacityState::default();
    for item in entries(field(&fixture, "offers")) {
        let supplied = offer(item);
        let record = super::canonical::offer(&supplied).expect("authored canonical offer");
        assert_eq!(super::canonical::parse_offer(&record.value).unwrap(), supplied);
        state = molten_core::compute_capacity::observe(&state, supplied).expect("authored core offer");
    }
    for step in entries(field(&fixture, "steps")) {
        let input = request(field(step, "request"));
        assert_eq!(
            molten_core::compute_capacity::eligibility(&state, &input),
            molten_core::compute_capacity::Eligibility::Eligible
        );
        let transition = molten_core::compute_capacity::reserve(&state, &input);
        let molten_core::compute_capacity::ReservationOutcome::Reserved { allocation, .. } = transition.decision else {
            panic!("authored reservation must pass core");
        };
        assert_eq!(allocation.id, text(field(step, "expected_allocation_id")));
        let record = super::canonical::allocation(&allocation).expect("authored canonical allocation");
        assert_eq!(super::canonical::parse_allocation(&record.value).unwrap(), allocation);
        state = transition.state;
    }
    assert_eq!(state.allocations["serving-b"].claims[0].quantity, 80);
    assert_eq!(state.allocations["agent-a-1"].claims[0].quantity, 40);
    assert_eq!(state.allocations["agent-a-2"].claims[0].quantity, 40);
}

include!("overlap.rs");
