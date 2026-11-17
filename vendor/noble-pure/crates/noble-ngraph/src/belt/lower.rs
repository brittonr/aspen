/// Lower a checked total-I64 graph to the bounded canonical belt program.
///
/// The output is untrusted until [`super::check`] accepts its bytes. A refusal
/// returns no artifact, even when the internal lowerer already emitted a prefix.
pub fn lower(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    belt_slots: u8,
    save_slots: u8,
) -> Result<crate::belt::types::Lowered, crate::belt::types::Refusal> {
    crate::belt::support::admit_graph(checked)?;
    validate_limits(belt_slots, save_slots)?;
    let mut lowerer = Lowerer::new(checked, belt_slots, save_slots)?;
    lowerer.definitions()?;
    lowerer.outputs()?;
    let instruction_count = lowerer.instruction_count;
    let bytes = crate::belt::codec::encode(
        ids,
        checked,
        belt_slots,
        save_slots,
        &lowerer.instruction_bytes,
    )?;
    Ok(crate::belt::types::Lowered {
        bytes,
        instruction_count,
        preservation_saves: lowerer.saves,
        preservation_loads: lowerer.loads,
    })
}

fn validate_limits(belt_slots: u8, save_slots: u8) -> Result<(), crate::belt::types::Refusal> {
    if !(super::MIN_CAPACITY..=super::MAX_CAPACITY).contains(&belt_slots) {
        return Err(crate::belt::types::Refusal::Capacity);
    }
    if save_slots > super::MAX_SAVE_SLOTS {
        return Err(crate::belt::types::Refusal::SaveSlots);
    }
    Ok(())
}

struct Lowerer<'a> {
    checked: &'a crate::check::CheckedGraph,
    belt_slots: usize,
    belt: alloc::vec::Vec<crate::ir::NodeId>,
    slots: alloc::vec::Vec<Option<crate::ir::NodeId>>,
    remaining: alloc::vec::Vec<u32>,
    instruction_bytes: alloc::vec::Vec<u8>,
    instruction_count: usize,
    saves: usize,
    loads: usize,
}

impl<'a> Lowerer<'a> {
    fn new(
        checked: &'a crate::check::CheckedGraph,
        belt_slots: u8,
        save_slots: u8,
    ) -> Result<Self, crate::belt::types::Refusal> {
        Ok(Self {
            checked,
            belt_slots: usize::from(belt_slots),
            belt: alloc::vec::Vec::with_capacity(usize::from(belt_slots)),
            slots: alloc::vec![None; usize::from(save_slots)],
            remaining: crate::belt::support::remaining_uses(checked)?,
            instruction_bytes: alloc::vec::Vec::new(),
            instruction_count: 0,
            saves: 0,
            loads: 0,
        })
    }

    fn definitions(&mut self) -> Result<(), crate::belt::types::Refusal> {
        let node_count = self.checked.graph().nodes.len();
        for index in 0..node_count {
            let id = crate::ir::NodeId(
                u32::try_from(index).map_err(|_| crate::belt::types::Refusal::GraphBound)?,
            );
            let definition = match &self.checked.graph().nodes[index].op {
                crate::ir::Op::Input(slot) => Definition::Input(*slot),
                crate::ir::Op::Const(crate::ir::Literal::I64(value)) => {
                    Definition::Constant(value.cast_unsigned())
                }
                crate::ir::Op::I64Add(left, right) => {
                    Definition::Operation(*left, *right, Operation::Add)
                }
                crate::ir::Op::I64Sub(left, right) => {
                    Definition::Operation(*left, *right, Operation::Sub)
                }
                crate::ir::Op::I64Mul(left, right) => {
                    Definition::Operation(*left, *right, Operation::Mul)
                }
                crate::ir::Op::I64Eq(left, right) => {
                    Definition::Operation(*left, *right, Operation::Eq)
                }
                _ => return Err(crate::belt::types::Refusal::UnsupportedGraph),
            };
            match definition {
                Definition::Input(slot) => {
                    self.define(id, crate::belt::types::Instruction::Input(slot))?
                }
                Definition::Constant(bits) => {
                    self.define(id, crate::belt::types::Instruction::Constant(bits))?;
                }
                Definition::Operation(left, right, operation) => {
                    self.operation(id, left, right, operation)?;
                }
            }
        }
        Ok(())
    }

    fn define(
        &mut self,
        id: crate::ir::NodeId,
        instruction: crate::belt::types::Instruction,
    ) -> Result<(), crate::belt::types::Refusal> {
        self.ensure_room()?;
        self.emit(instruction)?;
        self.insert(id);
        Ok(())
    }

    fn operation(
        &mut self,
        id: crate::ir::NodeId,
        left: crate::ir::NodeId,
        right: crate::ir::NodeId,
        operation: Operation,
    ) -> Result<(), crate::belt::types::Refusal> {
        self.ensure_available(left)?;
        self.ensure_available(right)?;
        if !self.belt.contains(&left) {
            self.ensure_available(left)?;
        }
        if !self.belt.contains(&right) {
            self.ensure_available(right)?;
        }
        let left_age = self.age(left)?;
        let right_age = self.age(right)?;
        self.consume(left)?;
        self.consume(right)?;
        self.ensure_room()?;
        self.emit(operation.instruction(left_age, right_age))?;
        self.insert(id);
        Ok(())
    }

    fn outputs(&mut self) -> Result<(), crate::belt::types::Refusal> {
        let output_count = self.checked.graph().outputs.len();
        for index in 0..output_count {
            let output = self.checked.graph().outputs[index];
            self.ensure_available(output)?;
            let age = self.age(output)?;
            self.emit(crate::belt::types::Instruction::Output(age))?;
            self.consume(output)?;
        }
        Ok(())
    }

    fn ensure_available(
        &mut self,
        node: crate::ir::NodeId,
    ) -> Result<(), crate::belt::types::Refusal> {
        if self.belt.contains(&node) {
            return Ok(());
        }
        let slot = self
            .slots
            .iter()
            .position(|value| *value == Some(node))
            .ok_or(crate::belt::types::Refusal::LiveValueLost)?;
        let slot = u8::try_from(slot).map_err(|_| crate::belt::types::Refusal::SlotBound)?;
        self.ensure_room()?;
        self.emit(crate::belt::types::Instruction::Load(slot))?;
        self.loads += 1;
        self.insert(node);
        Ok(())
    }

    fn ensure_room(&mut self) -> Result<(), crate::belt::types::Refusal> {
        if self.belt.len() < self.belt_slots {
            return Ok(());
        }
        let age = self.belt_slots - 1;
        let evicted = *self
            .belt
            .get(age)
            .ok_or(crate::belt::types::Refusal::StaleAge)?;
        if self.remaining[evicted.index()] == 0 || self.slots.contains(&Some(evicted)) {
            return Ok(());
        }
        let slot = self
            .reclaimable_slot()
            .ok_or(crate::belt::types::Refusal::LiveValueLost)?;
        let age = u8::try_from(age).map_err(|_| crate::belt::types::Refusal::Capacity)?;
        let slot_byte = u8::try_from(slot).map_err(|_| crate::belt::types::Refusal::SlotBound)?;
        self.emit(crate::belt::types::Instruction::Save(age, slot_byte))?;
        self.slots[slot] = Some(evicted);
        self.saves += 1;
        Ok(())
    }

    fn reclaimable_slot(&self) -> Option<usize> {
        self.slots.iter().enumerate().find_map(|(slot, value)| {
            let Some(old) = value else {
                return Some(slot);
            };
            if self.remaining[old.index()] == 0
                || self.belt.contains(old)
                || self
                    .slots
                    .iter()
                    .enumerate()
                    .any(|(other, entry)| other != slot && entry == &Some(*old))
            {
                Some(slot)
            } else {
                None
            }
        })
    }

    fn age(&self, node: crate::ir::NodeId) -> Result<u8, crate::belt::types::Refusal> {
        let age = self
            .belt
            .iter()
            .position(|value| *value == node)
            .ok_or(crate::belt::types::Refusal::LiveValueLost)?;
        u8::try_from(age).map_err(|_| crate::belt::types::Refusal::StaleAge)
    }

    fn consume(&mut self, node: crate::ir::NodeId) -> Result<(), crate::belt::types::Refusal> {
        crate::belt::support::consume(&mut self.remaining, node)
    }

    fn insert(&mut self, node: crate::ir::NodeId) {
        if self.belt.len() == self.belt_slots {
            self.belt.pop();
        }
        self.belt.insert(0, node);
    }

    fn emit(
        &mut self,
        instruction: crate::belt::types::Instruction,
    ) -> Result<(), crate::belt::types::Refusal> {
        self.instruction_count = self
            .instruction_count
            .checked_add(1)
            .ok_or(crate::belt::types::Refusal::InstructionBound)?;
        if self.instruction_count > crate::belt::MAX_INSTRUCTIONS {
            return Err(crate::belt::types::Refusal::InstructionBound);
        }
        match instruction {
            crate::belt::types::Instruction::Input(slot) => {
                self.instruction_bytes.push(0);
                self.instruction_bytes
                    .extend_from_slice(&slot.to_le_bytes());
            }
            crate::belt::types::Instruction::Constant(bits) => {
                self.instruction_bytes.push(1);
                self.instruction_bytes
                    .extend_from_slice(&bits.to_le_bytes());
            }
            crate::belt::types::Instruction::Add(left, right) => {
                self.instruction_bytes.extend_from_slice(&[2, left, right]);
            }
            crate::belt::types::Instruction::Sub(left, right) => {
                self.instruction_bytes.extend_from_slice(&[3, left, right]);
            }
            crate::belt::types::Instruction::Mul(left, right) => {
                self.instruction_bytes.extend_from_slice(&[4, left, right]);
            }
            crate::belt::types::Instruction::Eq(left, right) => {
                self.instruction_bytes.extend_from_slice(&[5, left, right]);
            }
            crate::belt::types::Instruction::Save(age, slot) => {
                self.instruction_bytes.extend_from_slice(&[6, age, slot]);
            }
            crate::belt::types::Instruction::Load(slot) => {
                self.instruction_bytes.extend_from_slice(&[7, slot]);
            }
            crate::belt::types::Instruction::Output(age) => {
                self.instruction_bytes.extend_from_slice(&[8, age]);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Definition {
    Input(u32),
    Constant(u64),
    Operation(crate::ir::NodeId, crate::ir::NodeId, Operation),
}

#[derive(Clone, Copy)]
enum Operation {
    Add,
    Sub,
    Mul,
    Eq,
}

impl Operation {
    fn instruction(self, left: u8, right: u8) -> crate::belt::types::Instruction {
        match self {
            Self::Add => crate::belt::types::Instruction::Add(left, right),
            Self::Sub => crate::belt::types::Instruction::Sub(left, right),
            Self::Mul => crate::belt::types::Instruction::Mul(left, right),
            Self::Eq => crate::belt::types::Instruction::Eq(left, right),
        }
    }
}
