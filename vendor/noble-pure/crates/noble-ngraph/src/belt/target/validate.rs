#[derive(Clone, Copy)]
enum Completion {
    Node(crate::ir::NodeId),
    Save {
        slot: usize,
        node: crate::ir::NodeId,
    },
}

#[derive(Clone, Copy)]
struct Pending {
    completion_cycle: u32,
    issue_cycle: u32,
    unit: u8,
    completion: Completion,
}

pub(super) fn check_stream(
    checked: &crate::check::CheckedGraph,
    schedule: &mut super::DecodedSchedule,
) -> Result<(), super::Refusal> {
    let graph = checked.graph();
    let descriptor = schedule.machine.descriptor();
    if schedule.events.len() > super::MAX_EVENTS {
        return Err(super::Refusal::EventBound);
    }
    if schedule.outputs.len() != graph.outputs.len()
        || schedule
            .outputs
            .iter()
            .zip(&graph.outputs)
            .any(|(actual, expected)| actual.node != expected.0)
    {
        return Err(super::Refusal::OutputMismatch);
    }
    if schedule.outputs.len() > usize::from(descriptor.belt_slots) {
        return Err(super::Refusal::OutputMismatch);
    }
    schedule
        .events
        .sort_unstable_by_key(|event| (event.cycle, event.lane, event.unit));
    let ordered = &schedule.events;
    if ordered
        .iter()
        .any(|event| event.cycle >= super::MAX_HORIZON)
    {
        return Err(super::Refusal::HorizonBound);
    }
    let units =
        usize::from(descriptor.alu_units + descriptor.multiply_units + descriptor.move_units);
    let mut lane_used = alloc::vec![false; usize::from(descriptor.issue_width)];
    let mut unit_used = alloc::vec![false; units];
    let mut last_issue = alloc::vec![None::<u32>; units];
    let mut issued = alloc::vec![false; graph.nodes.len()];
    let mut completed = alloc::vec![false; graph.nodes.len()];
    let mut remaining = use_counts(checked);
    let output_live = output_counts(checked);
    let mut belt = CheckedBelt::new(descriptor.belt_slots);
    let mut saves = alloc::vec![None::<crate::ir::NodeId>; usize::from(descriptor.save_slots)];
    let mut reserved_slots = alloc::vec![false; usize::from(descriptor.save_slots)];
    let mut pending = alloc::vec::Vec::with_capacity(ordered.len());
    let mut cursor = 0usize;
    let mut consumed_this_cycle =
        alloc::vec::Vec::with_capacity(usize::from(descriptor.issue_width) * 2);
    let mut last_completion_cycle_index = ordered.last().map_or(0, |event| event.cycle);
    for event in ordered {
        let class = operation_class(event.operation);
        let completion_cycles = u32::from(descriptor.completion_cycles(class));
        last_completion_cycle_index = last_completion_cycle_index.max(
            event
                .cycle
                .checked_add(completion_cycles - 1)
                .ok_or(super::Refusal::CompletionHorizon)?,
        );
    }
    if last_completion_cycle_index >= super::MAX_HORIZON {
        return Err(super::Refusal::CompletionHorizon);
    }
    let work_bound = ordered
        .len()
        .checked_add(last_completion_cycle_index as usize + 1)
        .ok_or(super::Refusal::CheckingExhausted)?;
    if work_bound > super::MAX_CHECK_WORK {
        return Err(super::Refusal::CheckingExhausted);
    }
    for cycle in 0..=last_completion_cycle_index {
        lane_used.fill(false);
        unit_used.fill(false);
        let start = cursor;
        while cursor < ordered.len() && ordered[cursor].cycle == cycle {
            cursor += 1;
        }
        if cursor.saturating_sub(start) > usize::from(descriptor.issue_width) {
            return Err(super::Refusal::IssueConflict);
        }
        consumed_this_cycle.clear();
        for event in &ordered[start..cursor] {
            check_issue_resources(
                *event,
                descriptor,
                &mut lane_used,
                &mut unit_used,
                &mut last_issue,
            )?;
            let node = event_node(event.operation);
            let completion = match event.operation {
                super::Operation::Input { slot, .. } => {
                    expect_input(graph, node, slot)?;
                    issue_graph_node(&mut issued, node)?;
                    Completion::Node(node)
                }
                super::Operation::Constant { bits, .. } => {
                    expect_constant(graph, node, bits)?;
                    issue_graph_node(&mut issued, node)?;
                    Completion::Node(node)
                }
                super::Operation::Add {
                    left_age,
                    right_age,
                    ..
                }
                | super::Operation::Sub {
                    left_age,
                    right_age,
                    ..
                }
                | super::Operation::Multiply {
                    left_age,
                    right_age,
                    ..
                }
                | super::Operation::Equal {
                    left_age,
                    right_age,
                    ..
                } => {
                    let (left, right) = graph_operands(graph, node, event.operation)?;
                    require_age(&belt.values, left_age, left)?;
                    require_age(&belt.values, right_age, right)?;
                    if !completed.get(left.0 as usize).copied().unwrap_or(false)
                        || !completed.get(right.0 as usize).copied().unwrap_or(false)
                    {
                        return Err(super::Refusal::DependencyNotReady);
                    }
                    issue_graph_node(&mut issued, node)?;
                    consumed_this_cycle.push(left);
                    consumed_this_cycle.push(right);
                    Completion::Node(node)
                }
                super::Operation::Save { age, slot, node } => {
                    let slot = usize::from(slot);
                    if slot >= saves.len() {
                        return Err(super::Refusal::SaveSlotBound);
                    }
                    require_age(&belt.values, age, crate::ir::NodeId(node))?;
                    if let Some(previous) = saves[slot] {
                        if is_live(previous, &remaining, &output_live)
                            && !belt.values.contains(&previous)
                            && !saves
                                .iter()
                                .enumerate()
                                .any(|(other, saved)| other != slot && *saved == Some(previous))
                        {
                            return Err(super::Refusal::LiveSaveOverwrite);
                        }
                        if is_live(previous, &remaining, &output_live) {
                            return Err(super::Refusal::LiveSaveOverwrite);
                        }
                    }
                    if reserved_slots[slot] {
                        return Err(super::Refusal::LiveSaveOverwrite);
                    }
                    reserved_slots[slot] = true;
                    Completion::Save {
                        slot,
                        node: crate::ir::NodeId(node),
                    }
                }
                super::Operation::Load { node, slot } => {
                    let slot = usize::from(slot);
                    let saved = saves
                        .get(slot)
                        .copied()
                        .flatten()
                        .ok_or(super::Refusal::UninitializedSave)?;
                    if saved.0 != node {
                        return Err(super::Refusal::UninitializedSave);
                    }
                    if !completed.get(node as usize).copied().unwrap_or(false) {
                        return Err(super::Refusal::DependencyNotReady);
                    }
                    Completion::Node(saved)
                }
            };
            let class = operation_class(event.operation);
            let completion_cycle = event
                .cycle
                .checked_add(u32::from(descriptor.completion_cycles(class)) - 1)
                .ok_or(super::Refusal::CompletionHorizon)?;
            pending.push(Pending {
                completion_cycle,
                issue_cycle: event.cycle,
                unit: event.unit,
                completion,
            });
        }
        for node in consumed_this_cycle.iter().copied() {
            if let Some(count) = remaining.get_mut(node.0 as usize) {
                *count = count.saturating_sub(1);
            }
        }
        pending.sort_unstable_by_key(|item| (item.completion_cycle, item.issue_cycle, item.unit));
        let mut completed_at_cycle = 0usize;
        while completed_at_cycle < pending.len()
            && pending[completed_at_cycle].completion_cycle == cycle
        {
            match pending[completed_at_cycle].completion {
                Completion::Node(node) => {
                    if !completed[node.0 as usize] {
                        completed[node.0 as usize] = true;
                    }
                    belt.insert_node(node, &saves, &remaining, &output_live)?;
                }
                Completion::Save { slot, node } => {
                    saves[slot] = Some(node);
                    reserved_slots[slot] = false;
                }
            }
            completed_at_cycle += 1;
        }
        pending.drain(..completed_at_cycle);
    }
    if cursor != ordered.len() {
        return Err(super::Refusal::CheckingExhausted);
    }
    if !pending.is_empty() {
        return Err(super::Refusal::CompletionHorizon);
    }
    if issued.iter().any(|value| !value) || completed.iter().any(|value| !value) {
        return Err(super::Refusal::MissingNode);
    }
    for output in &schedule.outputs {
        if !belt.values.iter().any(|node| node.0 == output.node) {
            return Err(super::Refusal::OutputMismatch);
        }
    }
    Ok(())
}

fn check_issue_resources(
    event: super::Issue,
    descriptor: super::MachineDescriptor,
    lane_used: &mut [bool],
    unit_used: &mut [bool],
    last_issue: &mut [Option<u32>],
) -> Result<(), super::Refusal> {
    let lane = usize::from(event.lane);
    if lane >= lane_used.len() {
        return Err(super::Refusal::IssueLane);
    }
    let unit = usize::from(event.unit);
    let class = descriptor
        .unit_class(event.unit)
        .ok_or(super::Refusal::UnitBound)?;
    if class != operation_class(event.operation) {
        return Err(super::Refusal::UnitClass);
    }
    if lane_used[lane] {
        return Err(super::Refusal::IssueConflict);
    }
    if unit_used[unit] {
        return Err(super::Refusal::UnitConflict);
    }
    if last_issue[unit].is_some_and(|last| {
        event.cycle.saturating_sub(last) < u32::from(descriptor.issue_gap_cycles)
    }) {
        return Err(super::Refusal::InitiationInterval);
    }
    lane_used[lane] = true;
    unit_used[unit] = true;
    last_issue[unit] = Some(event.cycle);
    Ok(())
}

fn issue_graph_node(issued: &mut [bool], node: crate::ir::NodeId) -> Result<(), super::Refusal> {
    let present = issued
        .get_mut(node.0 as usize)
        .ok_or(super::Refusal::NodeMismatch)?;
    if *present {
        return Err(super::Refusal::DuplicateNode);
    }
    *present = true;
    Ok(())
}

fn expect_input(
    graph: &crate::ir::Graph,
    node: crate::ir::NodeId,
    slot: u32,
) -> Result<(), super::Refusal> {
    if graph
        .nodes
        .get(node.0 as usize)
        .is_some_and(|entry| entry.op == crate::ir::Op::Input(slot))
    {
        Ok(())
    } else {
        Err(super::Refusal::NodeMismatch)
    }
}

fn expect_constant(
    graph: &crate::ir::Graph,
    node: crate::ir::NodeId,
    bits: u64,
) -> Result<(), super::Refusal> {
    let expected = graph
        .nodes
        .get(node.0 as usize)
        .ok_or(super::Refusal::NodeMismatch)?;
    if let crate::ir::Op::Const(crate::ir::Literal::I64(value)) = expected.op {
        if value as u64 == bits {
            Ok(())
        } else {
            Err(super::Refusal::NodeMismatch)
        }
    } else {
        Err(super::Refusal::NodeMismatch)
    }
}

fn graph_operands(
    graph: &crate::ir::Graph,
    node: crate::ir::NodeId,
    operation: super::Operation,
) -> Result<(crate::ir::NodeId, crate::ir::NodeId), super::Refusal> {
    let graph_node = graph
        .nodes
        .get(node.0 as usize)
        .ok_or(super::Refusal::NodeMismatch)?;
    let (left, right) = match (&graph_node.op, operation) {
        (crate::ir::Op::I64Add(left, right), super::Operation::Add { .. })
        | (crate::ir::Op::I64Sub(left, right), super::Operation::Sub { .. })
        | (crate::ir::Op::I64Mul(left, right), super::Operation::Multiply { .. })
        | (crate::ir::Op::I64Eq(left, right), super::Operation::Equal { .. }) => (*left, *right),
        _ => return Err(super::Refusal::NodeMismatch),
    };
    Ok((left, right))
}

fn require_age(
    belt: &[crate::ir::NodeId],
    age: u8,
    expected: crate::ir::NodeId,
) -> Result<(), super::Refusal> {
    if belt.get(usize::from(age)) == Some(&expected) {
        Ok(())
    } else {
        Err(super::Refusal::OperandMismatch)
    }
}

struct CheckedBelt {
    values: alloc::vec::Vec<crate::ir::NodeId>,
    slot_count: usize,
}

impl CheckedBelt {
    fn new(belt_slot_count: u8) -> Self {
        Self {
            values: alloc::vec::Vec::with_capacity(usize::from(belt_slot_count)),
            slot_count: usize::from(belt_slot_count),
        }
    }

    fn insert_node(
        &mut self,
        node: crate::ir::NodeId,
        saves: &[Option<crate::ir::NodeId>],
        remaining: &[u32],
        outputs: &[u32],
    ) -> Result<(), super::Refusal> {
        if self.values.len() == self.slot_count {
            let evicted = self.values.pop().ok_or(super::Refusal::RuntimeFault)?;
            // Liveness is checked after all issues in this cycle; remaining uses encode every
            // not-yet-issued graph dependency and every declared output remains live to final read.
            if is_live(evicted, remaining, outputs) && !saves.contains(&Some(evicted)) {
                return Err(super::Refusal::LiveValueLost);
            }
        }
        self.values.insert(0, node);
        Ok(())
    }
}

fn use_counts(checked: &crate::check::CheckedGraph) -> alloc::vec::Vec<u32> {
    let graph = checked.graph();
    let mut counts = alloc::vec![0u32; graph.nodes.len()];
    for node in &graph.nodes {
        node.op.visit_operands(|operand| {
            if let Some(count) = counts.get_mut(operand.0 as usize) {
                *count = count.saturating_add(1);
            }
        });
    }
    counts
}

fn output_counts(checked: &crate::check::CheckedGraph) -> alloc::vec::Vec<u32> {
    let graph = checked.graph();
    let mut counts = alloc::vec![0u32; graph.nodes.len()];
    for output in &graph.outputs {
        if let Some(count) = counts.get_mut(output.0 as usize) {
            *count = count.saturating_add(1);
        }
    }
    counts
}

fn is_live(node: crate::ir::NodeId, remaining: &[u32], outputs: &[u32]) -> bool {
    remaining.get(node.0 as usize).copied().unwrap_or(0) != 0
        || outputs.get(node.0 as usize).copied().unwrap_or(0) != 0
}

fn operation_class(operation: super::Operation) -> super::UnitClass {
    match operation {
        super::Operation::Input { .. }
        | super::Operation::Constant { .. }
        | super::Operation::Save { .. }
        | super::Operation::Load { .. } => super::UnitClass::Move,
        super::Operation::Multiply { .. } => super::UnitClass::Multiply,
        super::Operation::Add { .. }
        | super::Operation::Sub { .. }
        | super::Operation::Equal { .. } => super::UnitClass::Alu,
    }
}

fn event_node(operation: super::Operation) -> crate::ir::NodeId {
    crate::ir::NodeId(match operation {
        super::Operation::Input { node, .. }
        | super::Operation::Constant { node, .. }
        | super::Operation::Add { node, .. }
        | super::Operation::Sub { node, .. }
        | super::Operation::Multiply { node, .. }
        | super::Operation::Equal { node, .. }
        | super::Operation::Save { node, .. }
        | super::Operation::Load { node, .. } => node,
    })
}
