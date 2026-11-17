pub(super) fn check_stream(
    checked: &crate::check::CheckedGraph,
    artifact: &crate::belt::types::ArtifactView<'_>,
) -> Result<(), crate::belt::types::Refusal> {
    let mut machine = Symbolic::new(checked, artifact)?;
    let mut cursor = 0usize;
    while cursor < artifact.instruction_bytes.len() {
        let instruction = crate::belt::codec::instructions::next_instruction(
            artifact.instruction_bytes,
            &mut cursor,
        )?;
        machine.step(instruction)?;
    }
    machine.finish()
}

struct Symbolic<'a> {
    checked: &'a crate::check::CheckedGraph,
    belt_slots: usize,
    belt: alloc::vec::Vec<crate::ir::NodeId>,
    slots: alloc::vec::Vec<Option<crate::ir::NodeId>>,
    remaining: alloc::vec::Vec<u32>,
    next_node: usize,
    next_output: usize,
    output_phase: bool,
}

impl<'a> Symbolic<'a> {
    fn new(
        checked: &'a crate::check::CheckedGraph,
        artifact: &crate::belt::types::ArtifactView<'_>,
    ) -> Result<Self, crate::belt::types::Refusal> {
        Ok(Self {
            checked,
            belt_slots: usize::from(artifact.belt_slots),
            belt: alloc::vec::Vec::with_capacity(usize::from(artifact.belt_slots)),
            slots: alloc::vec![None; usize::from(artifact.save_slots)],
            remaining: crate::belt::support::remaining_uses(checked)?,
            next_node: 0,
            next_output: 0,
            output_phase: false,
        })
    }

    fn step(
        &mut self,
        instruction: crate::belt::types::Instruction,
    ) -> Result<(), crate::belt::types::Refusal> {
        if self.complete() {
            return Err(crate::belt::types::Refusal::OutputMismatch);
        }
        match instruction {
            crate::belt::types::Instruction::Save(age, slot) => self.save(age, slot),
            crate::belt::types::Instruction::Load(slot) => self.load(slot),
            crate::belt::types::Instruction::Output(age) => self.output(age),
            other => self.define(other),
        }
    }

    fn define(
        &mut self,
        instruction: crate::belt::types::Instruction,
    ) -> Result<(), crate::belt::types::Refusal> {
        if self.output_phase {
            return Err(crate::belt::types::Refusal::BodyMismatch);
        }
        let node = self
            .checked
            .graph()
            .nodes
            .get(self.next_node)
            .ok_or(crate::belt::types::Refusal::IncompleteProgram)?;
        let node_id = crate::ir::NodeId(
            u32::try_from(self.next_node).map_err(|_| crate::belt::types::Refusal::GraphBound)?,
        );
        let operation = match (&node.op, instruction) {
            (crate::ir::Op::Input(expected), crate::belt::types::Instruction::Input(actual))
                if *expected == actual =>
            {
                None
            }
            (
                crate::ir::Op::Const(crate::ir::Literal::I64(expected)),
                crate::belt::types::Instruction::Constant(actual),
            ) if expected.cast_unsigned() == actual => None,
            (crate::ir::Op::I64Add(left, right), crate::belt::types::Instruction::Add(a, b)) => {
                Some((*left, *right, a, b))
            }
            (crate::ir::Op::I64Sub(left, right), crate::belt::types::Instruction::Sub(a, b)) => {
                Some((*left, *right, a, b))
            }
            (crate::ir::Op::I64Mul(left, right), crate::belt::types::Instruction::Mul(a, b)) => {
                Some((*left, *right, a, b))
            }
            (crate::ir::Op::I64Eq(left, right), crate::belt::types::Instruction::Eq(a, b)) => {
                Some((*left, *right, a, b))
            }
            _ => return Err(crate::belt::types::Refusal::BodyMismatch),
        };
        if let Some((left, right, left_age, right_age)) = operation {
            if self.at(left_age)? != left || self.at(right_age)? != right {
                return Err(crate::belt::types::Refusal::OperandMismatch);
            }
            crate::belt::support::consume(&mut self.remaining, left)?;
            crate::belt::support::consume(&mut self.remaining, right)?;
        }
        self.insert(node_id)?;
        self.next_node += 1;
        Ok(())
    }

    fn save(&mut self, age: u8, slot: u8) -> Result<(), crate::belt::types::Refusal> {
        if self.output_phase && self.next_output >= self.checked.graph().outputs.len() {
            return Err(crate::belt::types::Refusal::OutputMismatch);
        }
        let source = self.at(age)?;
        let slot_index = usize::from(slot);
        let old = *self
            .slots
            .get(slot_index)
            .ok_or(crate::belt::types::Refusal::SlotBound)?;
        if let Some(old_node) = old {
            let is_duplicated = self.belt.contains(&old_node)
                || self
                    .slots
                    .iter()
                    .enumerate()
                    .any(|(other, value)| other != slot_index && *value == Some(old_node));
            if self.remaining[old_node.index()] != 0 && !is_duplicated {
                return Err(crate::belt::types::Refusal::LiveSlotOverwrite);
            }
        }
        self.slots[slot_index] = Some(source);
        Ok(())
    }

    fn load(&mut self, slot: u8) -> Result<(), crate::belt::types::Refusal> {
        if self.output_phase && self.next_output >= self.checked.graph().outputs.len() {
            return Err(crate::belt::types::Refusal::OutputMismatch);
        }
        let node = self
            .slots
            .get(usize::from(slot))
            .copied()
            .ok_or(crate::belt::types::Refusal::SlotBound)?
            .ok_or(crate::belt::types::Refusal::UninitializedSlot)?;
        if self.remaining[node.index()] == 0 {
            return Err(crate::belt::types::Refusal::LiveValueLost);
        }
        self.insert(node)
    }

    fn output(&mut self, age: u8) -> Result<(), crate::belt::types::Refusal> {
        if self.next_node != self.checked.graph().nodes.len() {
            return Err(crate::belt::types::Refusal::OutputMismatch);
        }
        self.output_phase = true;
        let expected = *self
            .checked
            .graph()
            .outputs
            .get(self.next_output)
            .ok_or(crate::belt::types::Refusal::OutputMismatch)?;
        if self.at(age)? != expected {
            return Err(crate::belt::types::Refusal::OutputMismatch);
        }
        crate::belt::support::consume(&mut self.remaining, expected)?;
        self.next_output += 1;
        Ok(())
    }

    fn insert(&mut self, node: crate::ir::NodeId) -> Result<(), crate::belt::types::Refusal> {
        if self.belt.len() == self.belt_slots {
            let evicted = *self
                .belt
                .last()
                .ok_or(crate::belt::types::Refusal::StaleAge)?;
            if self.remaining[evicted.index()] != 0 && !self.slots.contains(&Some(evicted)) {
                return Err(crate::belt::types::Refusal::LiveValueLost);
            }
            self.belt.pop();
        }
        self.belt.insert(0, node);
        Ok(())
    }

    fn at(&self, age: u8) -> Result<crate::ir::NodeId, crate::belt::types::Refusal> {
        self.belt
            .get(usize::from(age))
            .copied()
            .ok_or(crate::belt::types::Refusal::StaleAge)
    }

    fn complete(&self) -> bool {
        self.next_node == self.checked.graph().nodes.len()
            && self.next_output == self.checked.graph().outputs.len()
    }

    fn finish(&self) -> Result<(), crate::belt::types::Refusal> {
        if self.next_node != self.checked.graph().nodes.len() {
            return Err(crate::belt::types::Refusal::IncompleteProgram);
        }
        if self.next_output != self.checked.graph().outputs.len() {
            return Err(crate::belt::types::Refusal::OutputMismatch);
        }
        Ok(())
    }
}
