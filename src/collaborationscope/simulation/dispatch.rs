use super::*;

pub(super) fn deliver(
    scheduling: (
        &crate::fabric_simulation::AdmittedSimulatedWorld,
        &mut crate::fabric_simulation::SimulationSchedulerState,
    ),
    network: &mut crate::fabric_simulation::SimulatedTransportState,
    nodes: &mut [Application],
    input: (&crate::fabric_consistency::raft::Command, usize, u64),
    events: &mut Vec<preserves::IOValue>,
) -> crate::error::Result<()> {
    let (world, scheduler) = scheduling;
    let (command, sequence, seed) = input;
    let eligible = ready(world, scheduler, network)?;
    let candidates = eligible
        .iter()
        .map(|choice| candidate(world, choice, sequence, command))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let count = u64::try_from(candidates.len()).map_err(|_| denied(Denial::Bound))?;
    let position = seed
        .wrapping_add(scheduler.next_choice_position)
        .checked_rem(count)
        .ok_or_else(|| denied(Denial::Missing))?;
    let index = usize::try_from(position).map_err(|_| denied(Denial::Bound))?;
    let plan = crate::fabric_simulation::select_installed_workload_choice(
        world,
        scheduler,
        &candidates,
        Some(&candidates[index].choice.choice_id),
    )
    .map_err(|_| denied(Denial::Stale))?;
    if plan.workload.request_ref.as_str() != command.request_ref {
        return Err(denied(Denial::Domain));
    }
    let node = plan
        .workload
        .node_id
        .strip_prefix("scope-node-")
        .ok_or_else(|| denied(Denial::Domain))?
        .parse::<usize>()
        .map_err(|_| denied(Denial::Domain))?;
    let id = format!("scope-command-{sequence}-{node}");
    network.deliver(&id, plan.transition.next.virtual_tick).map_err(|_| denied(Denial::Partition))?;
    let before = domain_ref(nodes[node].state())?;
    let epoch = nodes[node].state().scopes[&fixtures::space_ref()].membership_epoch;
    let receipt = nodes[node].apply_batch(std::slice::from_ref(command))?;
    verify_domain(nodes[node].state(), &before, epoch, sequence)?;
    events.push(crate::preserves_rail::record("collaboration-simulation-event-v1", vec![
        crate::preserves_rail::string(id),
        crate::preserves_rail::u64_value(plan.transition.next.virtual_tick),
        crate::preserves_rail::string(nodes[node].state_ref().as_str()),
        crate::preserves_rail::string(receipt),
    ]));
    *scheduler = plan.transition.next;
    Ok(())
}
fn ready(
    world: &crate::fabric_simulation::AdmittedSimulatedWorld,
    scheduler: &mut crate::fabric_simulation::SimulationSchedulerState,
    network: &mut crate::fabric_simulation::SimulatedTransportState,
) -> crate::error::Result<Vec<crate::fabric_simulation::EligibleChoice>> {
    let eligible = network.eligible_deliveries(scheduler.virtual_tick);
    if !eligible.is_empty() {
        return Ok(eligible);
    }
    let timer = crate::fabric_simulation::EligibleChoice {
        kind: crate::fabric_simulation::SchedulerChoiceKind::TimerFire,
        choice_id: "heal-scope-partition".to_string(),
        node_id: "scope-node-2".to_string(),
        generation: 1,
        ready_at_tick: 5,
    };
    let transition = crate::fabric_simulation::select_simulation_choice(world, scheduler, &[timer], None)
        .map_err(|_| denied(Denial::Bound))?;
    *scheduler = transition.next;
    network.heal_ready_partitions(scheduler.virtual_tick);
    let eligible = network.eligible_deliveries(scheduler.virtual_tick);
    if eligible.is_empty() {
        return Err(denied(Denial::Partition));
    }
    Ok(eligible)
}
fn candidate(
    world: &crate::fabric_simulation::AdmittedSimulatedWorld,
    choice: &crate::fabric_simulation::EligibleChoice,
    sequence: usize,
    command: &crate::fabric_consistency::raft::Command,
) -> crate::error::Result<crate::fabric_simulation::InstalledEligibleChoice> {
    let installed = world.installed().ok_or_else(|| denied(Denial::Missing))?;
    let binding = installed
        .input()
        .nodes
        .iter()
        .find(|binding| binding.node_id == choice.node_id)
        .ok_or_else(|| denied(Denial::Domain))?;
    let node = choice
        .node_id
        .strip_prefix("scope-node-")
        .ok_or_else(|| denied(Denial::Domain))?
        .parse::<usize>()
        .map_err(|_| denied(Denial::Domain))?;
    Ok(crate::fabric_simulation::InstalledEligibleChoice {
        choice: choice.clone(),
        workload_sequence: (sequence * 3 + node) as u64,
        profile_ref: binding.profile_ref.clone(),
        request_ref: molten_core::nominal::ArtifactRef::new(command.request_ref.clone())
            .map_err(|_| denied(Denial::Domain))?,
    })
}
fn domain_ref(state: &State) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("collaboration-domain-state-v1", vec![
        crate::preserves_rail::sequence(state.scopes.values().map(space_value).collect()),
        crate::preserves_rail::sequence(state.bindings.values().map(binding_value).collect()),
    ]))
}
// Independent consumer-visible invariants, not the transition reducer's verdict.
fn verify_domain(state: &State, before: &str, epoch: u64, sequence: usize) -> crate::error::Result<()> {
    let expected = if sequence == 0 { epoch + 1 } else { epoch };
    if state.scopes[&fixtures::space_ref()].membership_epoch != expected {
        return Err(denied(Denial::Membership));
    }
    if sequence > 0 && domain_ref(state)? != before {
        return Err(denied(Denial::Conflict));
    }
    if state.bindings.len() > MAX_BINDINGS || state.operations.len() > MAX_OPERATIONS {
        return Err(denied(Denial::Bound));
    }
    for (resource, binding) in &state.bindings {
        let owner = state.scopes.get(&binding.owner_scope).ok_or_else(|| denied(Denial::Owner))?;
        if resource != &binding.resource || owner.owner != binding.owner {
            return Err(denied(Denial::Owner));
        }
    }
    if state.scopes.values().any(|scope| scope.members.len() > MAX_MEMBERS) {
        return Err(denied(Denial::Bound));
    }
    Ok(())
}
