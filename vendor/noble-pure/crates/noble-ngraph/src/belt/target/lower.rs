/// Deterministic bounded list scheduler for the selected fixed target model.
///
/// It issues graph nodes in canonical order, waits for each latency, and inserts explicit
/// move-unit save/load operations whenever a live value would otherwise be evicted. It is an
/// untrusted schedule producer; [`super::check`] independently validates the result.
pub fn lower(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    machine: super::Machine,
) -> Result<super::Lowered, super::Refusal> {
    crate::belt::support::admit_graph(checked).map_err(|_| super::Refusal::UnsupportedGraph)?;
    let descriptor = machine.descriptor();
    if checked.graph().outputs.len() > usize::from(descriptor.belt_slots) {
        return Err(super::Refusal::OutputMismatch);
    }
    let mut scheduler = Scheduler::new(checked, machine);
    scheduler.schedule_nodes()?;
    scheduler.prepare_outputs()?;
    let outputs = checked
        .graph()
        .outputs
        .iter()
        .map(|node| super::OutputRead { node: node.0 })
        .collect::<alloc::vec::Vec<_>>();
    let bytes = super::codec::encode(ids, checked, machine, &scheduler.events, &outputs)?;
    let max_completion = scheduler
        .events
        .iter()
        .map(|event| {
            let class = descriptor
                .unit_class(event.unit)
                .unwrap_or(super::UnitClass::Move);
            event.cycle + u32::from(descriptor.completion_cycles(class)) - 1
        })
        .max()
        .unwrap_or(0);
    Ok(super::Lowered {
        bytes,
        event_count: scheduler.events.len(),
        modeled_cycles: max_completion.saturating_add(1),
        preservation_saves: scheduler.saves_count,
        preservation_loads: scheduler.loads,
        scheduling_work: scheduler.work,
    })
}

struct Scheduler<'a> {
    checked: &'a crate::check::CheckedGraph,
    machine: super::Machine,
    cycle: u32,
    belt: alloc::vec::Vec<crate::ir::NodeId>,
    saves: alloc::vec::Vec<Option<crate::ir::NodeId>>,
    remaining: alloc::vec::Vec<u32>,
    events: alloc::vec::Vec<super::Issue>,
    saves_count: usize,
    loads: usize,
    work: usize,
}

impl<'a> Scheduler<'a> {
    fn new(checked: &'a crate::check::CheckedGraph, machine: super::Machine) -> Self {
        let descriptor = machine.descriptor();
        let graph = checked.graph();
        let mut remaining = alloc::vec![0u32; graph.nodes.len()];
        for graph_node in &graph.nodes {
            graph_node.op.visit_operands(|node| {
                if let Some(count) = remaining.get_mut(node.0 as usize) {
                    *count = count.saturating_add(1);
                }
            });
        }
        for node in &graph.outputs {
            if let Some(count) = remaining.get_mut(node.0 as usize) {
                *count = count.saturating_add(1);
            }
        }
        Self {
            checked,
            machine,
            cycle: 0,
            belt: alloc::vec::Vec::with_capacity(usize::from(descriptor.belt_slots)),
            saves: alloc::vec![None; usize::from(descriptor.save_slots)],
            remaining,
            events: alloc::vec::Vec::new(),
            saves_count: 0,
            loads: 0,
            work: 0,
        }
    }

    fn schedule_nodes(&mut self) -> Result<(), super::Refusal> {
        for index in 0..self.checked.node_count() {
            self.work = self
                .work
                .checked_add(1)
                .ok_or(super::Refusal::SchedulingExhausted)?;
            if self.work > super::MAX_CHECK_WORK {
                return Err(super::Refusal::SchedulingExhausted);
            }
            let node_id = crate::ir::NodeId(index as u32);
            let node_operation = node_operation(self.checked, index)?;
            let operation = match node_operation {
                NodeOperation::Input(slot) => super::Operation::Input {
                    node: node_id.0,
                    slot,
                },
                NodeOperation::Constant(bits) => super::Operation::Constant {
                    node: node_id.0,
                    bits,
                },
                NodeOperation::Add(left, right) => {
                    self.ensure_operand(left)?;
                    self.ensure_operand(right)?;
                    let left_age = self.age(left)?;
                    let right_age = self.age(right)?;
                    self.make_room()?;
                    super::Operation::Add {
                        node: node_id.0,
                        left_age,
                        right_age,
                    }
                }
                NodeOperation::Subtract(left, right) => {
                    self.ensure_operand(left)?;
                    self.ensure_operand(right)?;
                    let left_age = self.age(left)?;
                    let right_age = self.age(right)?;
                    self.make_room()?;
                    super::Operation::Sub {
                        node: node_id.0,
                        left_age,
                        right_age,
                    }
                }
                NodeOperation::Multiply(left, right) => {
                    self.ensure_operand(left)?;
                    self.ensure_operand(right)?;
                    let left_age = self.age(left)?;
                    let right_age = self.age(right)?;
                    self.make_room()?;
                    super::Operation::Multiply {
                        node: node_id.0,
                        left_age,
                        right_age,
                    }
                }
                NodeOperation::Equal(left, right) => {
                    self.ensure_operand(left)?;
                    self.ensure_operand(right)?;
                    let left_age = self.age(left)?;
                    let right_age = self.age(right)?;
                    self.make_room()?;
                    super::Operation::Equal {
                        node: node_id.0,
                        left_age,
                        right_age,
                    }
                }
            };
            if matches!(
                operation,
                super::Operation::Input { .. } | super::Operation::Constant { .. }
            ) {
                self.make_room()?;
            }
            self.issue(operation)?;
            match node_operation {
                NodeOperation::Add(left, right)
                | NodeOperation::Subtract(left, right)
                | NodeOperation::Multiply(left, right)
                | NodeOperation::Equal(left, right) => self.consume(left, right),
                NodeOperation::Input(_) | NodeOperation::Constant(_) => {}
            }
            let class = self.class(operation);
            self.cycle = self
                .cycle
                .checked_add(u32::from(
                    self.machine.descriptor().completion_cycles(class),
                ))
                .ok_or(super::Refusal::HorizonBound)?;
            self.insert(node_id)?;
        }
        Ok(())
    }

    fn prepare_outputs(&mut self) -> Result<(), super::Refusal> {
        for index in 0..self.checked.graph().outputs.len() {
            let node = self.checked.graph().outputs[index];
            if self.age(node).is_err() {
                self.ensure_operand(node)?;
                if self.age(node).is_err() {
                    return Err(super::Refusal::OutputMismatch);
                }
            }
        }
        Ok(())
    }

    fn ensure_operand(&mut self, node: crate::ir::NodeId) -> Result<(), super::Refusal> {
        if self.age(node).is_ok() {
            return Ok(());
        }
        let slot = self
            .saves
            .iter()
            .position(|saved| *saved == Some(node))
            .ok_or(super::Refusal::DependencyNotReady)?;
        self.make_room()?;
        let move_unit = self.unit(super::UnitClass::Move, 0)?;
        self.push_event(
            move_unit,
            super::Operation::Load {
                node: node.0,
                slot: slot as u8,
            },
        )?;
        self.loads += 1;
        self.cycle = self
            .cycle
            .checked_add(u32::from(self.machine.descriptor().move_cycles))
            .ok_or(super::Refusal::HorizonBound)?;
        self.insert(node)
    }

    fn make_room(&mut self) -> Result<(), super::Refusal> {
        let descriptor = self.machine.descriptor();
        if self.belt.len() < usize::from(descriptor.belt_slots) {
            return Ok(());
        }
        let victim = *self.belt.last().ok_or(super::Refusal::RuntimeFault)?;
        if self.live(victim) && !self.saves.contains(&Some(victim)) {
            let slot = self
                .saves
                .iter()
                .position(|saved| match saved {
                    None => true,
                    Some(old) => !self.live(*old),
                })
                .ok_or(super::Refusal::SchedulingExhausted)?;
            let age = self.age(victim)?;
            let unit = self.unit(super::UnitClass::Move, 0)?;
            self.push_event(
                unit,
                super::Operation::Save {
                    node: victim.0,
                    age,
                    slot: slot as u8,
                },
            )?;
            self.saves[slot] = Some(victim);
            self.saves_count += 1;
            self.cycle = self
                .cycle
                .checked_add(u32::from(descriptor.move_cycles))
                .ok_or(super::Refusal::HorizonBound)?;
        }
        Ok(())
    }

    fn issue(&mut self, operation: super::Operation) -> Result<(), super::Refusal> {
        let class = self.class(operation);
        let unit = self.unit(class, 0)?;
        self.push_event(unit, operation)
    }

    fn push_event(&mut self, unit: u8, operation: super::Operation) -> Result<(), super::Refusal> {
        if self.events.len() >= super::MAX_EVENTS || self.cycle >= super::MAX_HORIZON {
            return Err(super::Refusal::SchedulingExhausted);
        }
        self.events.push(super::Issue {
            cycle: self.cycle,
            lane: 0,
            unit,
            operation,
        });
        Ok(())
    }

    fn insert(&mut self, node: crate::ir::NodeId) -> Result<(), super::Refusal> {
        if self.belt.len() == usize::from(self.machine.descriptor().belt_slots) {
            let removed = self.belt.pop().ok_or(super::Refusal::RuntimeFault)?;
            if self.live(removed) && !self.saves.contains(&Some(removed)) {
                return Err(super::Refusal::SchedulingExhausted);
            }
        }
        self.belt.insert(0, node);
        Ok(())
    }

    fn age(&self, node: crate::ir::NodeId) -> Result<u8, super::Refusal> {
        self.belt
            .iter()
            .position(|value| *value == node)
            .and_then(|age| u8::try_from(age).ok())
            .ok_or(super::Refusal::DependencyNotReady)
    }

    fn live(&self, node: crate::ir::NodeId) -> bool {
        self.remaining
            .get(node.0 as usize)
            .is_some_and(|remaining| *remaining != 0)
    }

    fn consume(&mut self, left: crate::ir::NodeId, right: crate::ir::NodeId) {
        for node in [left, right] {
            if let Some(remaining) = self.remaining.get_mut(node.0 as usize) {
                *remaining = remaining.saturating_sub(1);
            }
        }
    }

    fn class(&self, operation: super::Operation) -> super::UnitClass {
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

    fn unit(&self, class: super::UnitClass, index: u8) -> Result<u8, super::Refusal> {
        let descriptor = self.machine.descriptor();
        let id = match class {
            super::UnitClass::Alu if index < descriptor.alu_units => index,
            super::UnitClass::Multiply if index < descriptor.multiply_units => {
                descriptor.alu_units + index
            }
            super::UnitClass::Move if index < descriptor.move_units => {
                descriptor.alu_units + descriptor.multiply_units + index
            }
            _ => return Err(super::Refusal::UnitBound),
        };
        Ok(id)
    }
}

#[derive(Clone, Copy)]
enum NodeOperation {
    Input(u32),
    Constant(u64),
    Add(crate::ir::NodeId, crate::ir::NodeId),
    Subtract(crate::ir::NodeId, crate::ir::NodeId),
    Multiply(crate::ir::NodeId, crate::ir::NodeId),
    Equal(crate::ir::NodeId, crate::ir::NodeId),
}

fn node_operation(
    checked: &crate::check::CheckedGraph,
    index: usize,
) -> Result<NodeOperation, super::Refusal> {
    let graph_node = checked
        .graph()
        .nodes
        .get(index)
        .ok_or(super::Refusal::NodeMismatch)?;
    match &graph_node.op {
        crate::ir::Op::Input(slot) => Ok(NodeOperation::Input(*slot)),
        crate::ir::Op::Const(crate::ir::Literal::I64(value)) => {
            Ok(NodeOperation::Constant(*value as u64))
        }
        crate::ir::Op::I64Add(left, right) => Ok(NodeOperation::Add(*left, *right)),
        crate::ir::Op::I64Sub(left, right) => Ok(NodeOperation::Subtract(*left, *right)),
        crate::ir::Op::I64Mul(left, right) => Ok(NodeOperation::Multiply(*left, *right)),
        crate::ir::Op::I64Eq(left, right) => Ok(NodeOperation::Equal(*left, *right)),
        crate::ir::Op::Const(_)
        | crate::ir::Op::Param(_)
        | crate::ir::Op::F32Add(..)
        | crate::ir::Op::F32Sub(..)
        | crate::ir::Op::F32Mul(..)
        | crate::ir::Op::F32Affine { .. }
        | crate::ir::Op::F32Relu(_)
        | crate::ir::Op::F32Reglu { .. }
        | crate::ir::Op::F32Step(_)
        | crate::ir::Op::F32Select { .. }
        | crate::ir::Op::F32Index { .. }
        | crate::ir::Op::Guard { .. } => Err(super::Refusal::UnsupportedGraph),
    }
}
