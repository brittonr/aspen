use super::*;
use crate::fabric_consistency::raft::CommittedBatchHandler;
use crate::system_extension::NativeCallbackValuePort;
#[path = "simulation/world.rs"]
mod world;

#[derive(Clone)]
struct Values(crate::system_extension::SharedCallbackValuePort);
impl NativeCallbackValuePort for Values {
    fn materialize(
        &mut self,
        reference: &str,
        bound: u64,
    ) -> Result<crate::system_extension::NativeCallbackValue, crate::system_extension::ValuePortFailure> {
        self.0.lock().map_err(|_| value_error())?.materialize(reference, bound)
    }
    fn publish(
        &mut self,
        value: &crate::system_extension::NativeCallbackValue,
        bound: u64,
    ) -> Result<crate::system_extension::ValuePublicationReceipt, crate::system_extension::ValuePortFailure> {
        self.0.lock().map_err(|_| value_error())?.publish(value, bound)
    }
}
fn value_error() -> crate::system_extension::ValuePortFailure {
    crate::system_extension::ValuePortFailure::new(
        crate::system_extension::ValuePortFailureKind::RejectedBeforeAcceptance,
        "simulation value port unavailable",
    )
}
fn store() -> crate::error::Result<Database> {
    let mut profile =
        crate::fabric_durability::tests::profile(crate::fabric_durability::DurableAdapterKind::DeterministicSimulation)
            .profile;
    profile.max_operation_bytes = MAX_RECORD_BYTES;
    profile.max_namespace_bytes = MAX_RECORD_BYTES * 8;
    profile.max_ordered_entries = 128;
    let profile = crate::fabric_durability::durable_profile(&profile)?;
    let mut descriptor = crate::fabric_durability::tests::descriptor();
    descriptor.atomicity_domain.max_bytes = MAX_RECORD_BYTES;
    descriptor.quota_bytes = MAX_RECORD_BYTES * 8;
    let domain = descriptor.atomicity_domain.clone();
    Database::simulated(crate::fabric_durability::SimulatedDurableStateAdapter::new(profile, descriptor)?, domain)
}
fn application(values: Values) -> crate::error::Result<Application> {
    let mut store = store()?;
    store.initialize(&fixtures::initial_state()?)?;
    Application::open(store, Box::new(values))
}
fn current(application: &Application) -> CurrentState {
    CurrentState {
        currentness: application.state().currentness.clone(),
        state: application.state_ref().clone(),
    }
}
struct Run {
    states: Vec<State>,
    trace: String,
}
fn run(seed: u64) -> crate::error::Result<Run> {
    let values = Values(crate::system_extension::shared_callback_value_port(
        crate::system_extension::InMemoryCallbackValuePort::empty(),
    ));
    let mut nodes = (0..3).map(|_| application(values.clone())).collect::<crate::error::Result<Vec<_>>>()?;
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(store()?);
    let initial = nodes[0].state_ref().clone();
    let labels = ["simulation-first", "simulation-conflict"];
    let mut commands = Vec::with_capacity(labels.len());
    for label in labels {
        let current = current(&nodes[0]);
        let mutation = fixtures::add_member(nodes[0].state(), label)?;
        commands.push(authority.stage(&mut nodes[0], current, mutation, &replay)?);
    }
    let world = world::manifest(&initial, seed, &commands)?;
    let mut scheduler = crate::fabric_simulation::SimulationSchedulerState::initial();
    let mut network = crate::fabric_simulation::SimulatedTransportState::new();
    network
        .open_partition(crate::fabric_simulation::SimulatedPartition {
            fault_id: "scope-partition".to_string(),
            destination: "scope-node-2".to_string(),
            heals_at_tick: 5,
        })
        .map_err(|_| denied(Denial::Partition))?;
    let mut events = Vec::new();
    for (sequence, command) in commands.iter().enumerate() {
        submit(&mut network, command, sequence)?;
        for _ in 0..3 {
            dispatch::deliver(
                (&world, &mut scheduler),
                &mut network,
                &mut nodes,
                (command, sequence, seed),
                &mut events,
            )?;
        }
        if sequence == 0 {
            let before = nodes[2].state_ref().clone();
            nodes[2].inject_simulated_fault(crate::fabric_durability::SimulatedFault::ProcessCrash)?;
            nodes[2].reload_durable()?;
            if nodes[2].state_ref() != &before {
                return Err(denied(Denial::Conflict));
            }
        }
    }
    if nodes.iter().any(|node| node.state_ref() != nodes[0].state_ref()) {
        return Err(denied(Denial::Conflict));
    }
    let world_ref = crate::preserves_rail::canonical_hash(&crate::fabric_simulation::extension_world_value(&world)?)?;
    let trace = crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "collaboration-simulation-run-v1",
        vec![
            crate::preserves_rail::u64_value(seed),
            crate::preserves_rail::string(world_ref),
            crate::preserves_rail::sequence(events),
            crate::preserves_rail::string("recorded-committed-log-not-a-live-consensus-or-disk-proof"),
        ],
    ))?;
    Ok(Run {
        states: nodes.into_iter().map(|node| node.state).collect(),
        trace,
    })
}
fn submit(
    network: &mut crate::fabric_simulation::SimulatedTransportState,
    command: &crate::fabric_consistency::raft::Command,
    sequence: usize,
) -> crate::error::Result<()> {
    for index in 0..3 {
        network
            .submit(crate::fabric_simulation::SimulatedTransmission {
                transmission_id: format!("scope-command-{sequence}-{index}"),
                destination: format!("scope-node-{index}"),
                port_id: "scope-ordered-commit-replay".to_string(),
                request_ref: command.request_ref.clone(),
                generation: 1,
                submitted_at_tick: 0,
                eligible_at_tick: 0,
            })
            .map_err(|_| denied(Denial::Bound))?;
    }
    Ok(())
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn same_application_world_replays_partition_crash_conflict_and_convergence() {
    for seed in 0..8 {
        let first = run(seed).expect("admitted simulation");
        let replay = run(seed).expect("deterministic replay");
        assert_eq!(first.trace, replay.trace);
        assert_eq!(first.states, replay.states);
        for state in &first.states {
            assert_eq!(state, &first.states[0]);
            assert_eq!(state.scopes[&fixtures::space_ref()].membership_epoch, 2);
            assert_eq!(state.scopes[&fixtures::space_ref()].members.len(), 2);
            assert_eq!(state.currentness.committed_index, 2);
            assert_eq!(state.bindings.len(), 1);
        }
    }
}
// r[verify molten.collaboration_scope.validation]
#[test]
fn actual_simulated_response_loss_quarantines_reloads_and_snapshot_admission() {
    let values = Values(crate::system_extension::shared_callback_value_port(
        crate::system_extension::InMemoryCallbackValuePort::empty(),
    ));
    let mut node = application(values).expect("application");
    let authority = fixtures::Signers::new();
    let replay = NonceLedger::new(store().expect("replay"));
    let original = node.state().clone();
    let admission = current(&node);
    let mutation = fixtures::add_member(node.state(), "uncertain-member").expect("request");
    let command = authority.stage(&mut node, admission, mutation, &replay).expect("actual signature");
    node.inject_simulated_fault(crate::fabric_durability::SimulatedFault::ResponseLostAfterCommit)
        .expect("actual storage fault");
    assert!(node.apply_batch(&[command]).is_err());
    assert_eq!(node.state(), &original);
    assert!(node.reload_durable().is_err());
    assert!(authority.snapshot(&node, current(&node), &replay).is_err());
}

#[path = "simulation/dispatch.rs"]
mod dispatch;
