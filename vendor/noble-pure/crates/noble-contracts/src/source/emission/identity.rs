/// Definition specialization identity includes immutable adapter selection.
/// The module's nominal id remains independent of this linked-program recipe.
/// Consumers also compare the full adapter identity, so a u64 fold collision
/// cannot silently substitute a previously compiled body.
pub(super) fn specialization(
    identity: u64,
    environment: &noble_kernel::contracts::Env,
    selected: &[u32],
) -> u64 {
    if selected.is_empty() {
        return identity;
    }
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ identity;
    let mut at = 0usize;
    while at < selected.len() {
        hash = hash_selected_slot(hash, environment, selected[at]);
        at += 1;
    }
    hash
}

fn hash_selected_slot(mut hash: u64, environment: &noble_kernel::contracts::Env, slot: u32) -> u64 {
    let mut adapter_at = 0usize;
    let mut is_found = false;
    while adapter_at < environment.bound_adapters.len() && !is_found {
        let adapter = &environment.bound_adapters[adapter_at];
        if adapter.adapter_slot == slot {
            hash = hash_adapter(hash, adapter);
            is_found = true;
        }
        adapter_at += 1;
    }
    hash
}

fn hash_adapter(hash: u64, adapter: &noble_kernel::contracts::BoundAdapter) -> u64 {
    let slot_bytes = adapter.adapter_slot.to_le_bytes();
    let hash = fold_bytes(hash, &slot_bytes);
    let hash = fold_bytes(hash, adapter.adapter_identity.as_bytes());
    (hash ^ 0xff).wrapping_mul(0x100_0000_01b3)
}

fn fold_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    let mut at = 0usize;
    while at < bytes.len() {
        hash = (hash ^ u64::from(bytes[at])).wrapping_mul(0x100_0000_01b3);
        at += 1;
    }
    hash
}

pub(super) fn selected_slots(
    state: &crate::source::inference::State,
    environment: &noble_kernel::contracts::Env,
) -> alloc::vec::Vec<u32> {
    let mut selected = alloc::vec::Vec::new();
    let mut body_at = 0usize;
    while body_at < state.bodies.len() {
        selected = append_selected_nodes(selected, &state.bodies[body_at].nodes, environment);
        body_at += 1;
    }
    // Distinct slot permutations must have the same specialization identity.
    selected.sort_unstable();
    selected
}

fn append_selected_nodes(
    mut selected: alloc::vec::Vec<u32>,
    nodes: &[crate::source::inference::Draft],
    environment: &noble_kernel::contracts::Env,
) -> alloc::vec::Vec<u32> {
    let mut node_at = 0usize;
    while node_at < nodes.len() {
        if let Some(slot) = selected_slot(&nodes[node_at], environment) {
            if !selected.contains(&slot) {
                selected.push(slot);
            }
        }
        node_at += 1;
    }
    selected
}

fn selected_slot(
    node: &crate::source::inference::Draft,
    environment: &noble_kernel::contracts::Env,
) -> Option<u32> {
    if let crate::source::inference::DraftKind::Invocation(definition) = &node.kind {
        if let Some(
            noble_kernel::contracts::Behavior::BoundEmit(slot)
            | noble_kernel::contracts::Behavior::BoundClock(slot)
            | noble_kernel::contracts::Behavior::BoundCodeExport(slot)
            | noble_kernel::contracts::Behavior::BoundSend(slot),
        ) = environment.kind(*definition)
        {
            return Some(slot);
        }
    }
    None
}
