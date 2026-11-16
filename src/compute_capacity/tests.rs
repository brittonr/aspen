use super::*;

fn reference() -> String {
    format!("blake3:{}", "a".repeat(64))
}

fn fixture_offer() -> Offer {
    Offer {
        schema: OFFER_SCHEMA.into(),
        host_id: "host-a".into(),
        host_incarnation: "host-1".into(),
        device_incarnation: "device-1".into(),
        provider_profile_ref: reference(),
        source_profile_ref: reference(),
        revision: 1,
        topology_generation: 1,
        observed_at: 10,
        expires_at: 100,
        observation_charge_cursor: Some(0),
        health: Health::Healthy,
        label_authority: molten_core::fabric_membership::LabelAuthority::Authoritative,
        compatibility: Compatibility {
            backend: Some("backend".into()),
            runtime: Some("runtime".into()),
            architecture: Some("arch".into()),
            topology: Some("single".into()),
            qualification_refs: vec![reference()],
        },
        pools: vec![Pool {
            id: "memory".into(),
            kind: PoolKind::DedicatedMemoryBytes,
            authority_id: "owner-a".into(),
            total: 100,
            allocatable: 100,
            observed_used: Some(0),
            observed_free: Some(100),
            is_observation_debited: false,
        }],
        views: vec![View {
            id: "partition".into(),
            device_id: "device-a".into(),
            partition_id: Some("p1".into()),
            granularity: Granularity::Partition,
            backing_pools: vec!["memory".into()],
            is_grouping_enforced: true,
        }],
    }
}

fn fixture_request(operation: &str) -> ReservationInput {
    ReservationInput {
        host_id: "host-a".into(),
        host_incarnation: "host-1".into(),
        device_incarnation: "device-1".into(),
        offer_revision: 1,
        topology_generation: 1,
        now: 20,
        view_id: "partition".into(),
        granularity: Granularity::Partition,
        compatibility: RequiredCompatibility {
            backend: Some("backend".into()),
            runtime: Some("runtime".into()),
            architecture: Some("arch".into()),
            topology: Some("single".into()),
        },
        minimum_label_authority: molten_core::fabric_membership::LabelAuthority::Authoritative,
        consumer_id: "worker".into(),
        operation_id: operation.into(),
        attempt_id: "attempt-1".into(),
        owner_generation: 1,
        grant_ref: reference(),
        grant: GrantFacts {
            grant_ref: reference(),
            consumer_id: "worker".into(),
            is_valid: true,
            is_revoked: false,
            expires_at: 80,
        },
        claims: vec![Claim {
            pool_id: "memory".into(),
            quantity: 60,
        }],
        serving_allocation_id: None,
        kind: AllocationKind::Execution,
    }
}

fn owner() -> OwnerInput {
    OwnerInput {
        allocation_id: "operation-1".into(),
        consumer_id: "worker".into(),
        owner_generation: 1,
    }
}

fn activation(allocation: &Allocation) -> ActivationEvidence {
    ActivationEvidence {
        allocation_id: allocation.id.clone(),
        owner_generation: allocation.owner_generation,
        backend_ref: reference(),
        is_active: true,
        enforcement: EnforcementStrength::BackendAcknowledged,
    }
}

fn released(allocation: &Allocation) -> BackendReleaseEvidence {
    BackendReleaseEvidence {
        allocation_id: allocation.id.clone(),
        owner_generation: allocation.owner_generation,
        backend_ref: reference(),
        is_released: true,
        enforcement: EnforcementStrength::BackendAcknowledged,
    }
}

#[derive(Default)]
struct RecordingBackend {
    activations: usize,
    releases: usize,
    is_lost_activation_ack: bool,
    is_lost_release_ack: bool,
    is_active_observed: bool,
    is_release_observed: bool,
    is_definite_activation_refusal: bool,
    is_forged_activation_observation: bool,
    is_forged_release_observation: bool,
}

impl ComputeBackendPort for RecordingBackend {
    fn activate(&mut self, allocation: &Allocation) -> Result<ActivationEvidence, BackendIssue> {
        self.activations += 1;
        if self.is_definite_activation_refusal {
            return Err(BackendIssue {
                message: "activation definitively refused".into(),
                has_possible_effect: false,
            });
        }
        if self.is_lost_activation_ack {
            Err(BackendIssue {
                message: "backend start acknowledgement lost".into(),
                has_possible_effect: true,
            })
        } else {
            Ok(activation(allocation))
        }
    }

    fn release(&mut self, allocation: &Allocation) -> Result<BackendReleaseEvidence, BackendIssue> {
        self.releases += 1;
        if self.is_lost_release_ack {
            Err(BackendIssue {
                message: "backend stop acknowledgement lost".into(),
                has_possible_effect: true,
            })
        } else {
            Ok(released(allocation))
        }
    }
    fn observe_activation(&mut self, allocation: &Allocation) -> Result<Option<ActivationEvidence>, BackendIssue> {
        if self.is_forged_activation_observation {
            let mut evidence = activation(allocation);
            evidence.owner_generation += 1;
            return Ok(Some(evidence));
        }
        Ok(self.is_active_observed.then(|| activation(allocation)))
    }

    fn observe_release(&mut self, allocation: &Allocation) -> Result<Option<BackendReleaseEvidence>, BackendIssue> {
        if self.is_forged_release_observation {
            let mut evidence = released(allocation);
            evidence.owner_generation += 1;
            return Ok(Some(evidence));
        }
        Ok(self.is_release_observed.then(|| released(allocation)))
    }
}

struct TestAdmission {
    grant: GrantFacts,
    minimum_label_authority: molten_core::fabric_membership::LabelAuthority,
}

impl Default for TestAdmission {
    fn default() -> Self {
        Self {
            grant: fixture_request("operation-1").grant,
            minimum_label_authority: molten_core::fabric_membership::LabelAuthority::Authoritative,
        }
    }
}

impl ComputeAdmissionPort for TestAdmission {
    fn reservation_facts(
        &self,
        _input: &ReservationInput,
        _current: &CapacityState,
    ) -> crate::fabric::FabricPortResult<AdmittedReservationFacts> {
        Ok(AdmittedReservationFacts {
            grant: self.grant.clone(),
            minimum_label_authority: self.minimum_label_authority,
        })
    }
}

struct TestClock(u64);

impl ComputeTimePort for TestClock {
    fn now_ticks(&self) -> crate::fabric::FabricPortResult<u64> {
        Ok(self.0)
    }
}

fn make_service<S: ComputeStorePort>(
    store: S,
    backend: RecordingBackend,
) -> Service<S, SuppliedOffers, RecordingBackend, TestAdmission, TestClock> {
    Service::new(store, SuppliedOffers::new(vec![]).unwrap(), backend, TestAdmission::default(), TestClock(20))
}

fn store_root(label: &str) -> (crate::test_support::Workspace, crate::local_store::DurableStoreRoot) {
    let workspace = crate::test_support::Workspace::new(label).expect("owned workspace");
    let state = workspace.state().expect("state capability");
    let root = crate::node_state::DirectoryRoot::from_dir(state.dir().try_clone().expect("clone capability"));
    root.create_layout().expect("node layout");
    let store = crate::local_store::DurableStoreRoot::from_ledger(
        &root.ledger_store().expect("ledger capability"),
        &crate::local_store::Locator::parse("compute-owner").expect("relative namespace"),
    )
    .expect("compute owner capability");
    (workspace, store)
}

fn observed_store(root: &crate::local_store::DurableStoreRoot) -> persistence::Redb {
    let mut store = persistence::Redb::open(root, "owner-a").expect("open compute owner");
    let current = store.load().expect("initial state");
    let observed = molten_core::compute_capacity::observe(&current, fixture_offer()).expect("valid offer");
    store.compare_and_store(&current, &observed).expect("durable offer");
    store
}

include!("tests/reopen.rs");
include!("tests/authority.rs");
include!("tests/reconciliation.rs");
