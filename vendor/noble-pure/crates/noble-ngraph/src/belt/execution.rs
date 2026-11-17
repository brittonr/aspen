/// Check and execute an artifact, publishing values only after the entire run succeeds.
pub fn execute(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    bytes: &[u8],
    inputs: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<crate::sem::Value>, crate::belt::types::Refusal> {
    let accepted = super::validate::check(ids, checked, bytes)?;
    execute_checked(ids, checked, accepted, inputs)
}

/// Execute a checker-issued token after rechecking its artifact digest and binding.
pub fn execute_checked(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    accepted: crate::belt::types::CheckedArtifact<'_>,
    inputs: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<crate::sem::Value>, crate::belt::types::Refusal> {
    super::support::admit_graph(checked)?;
    let view = super::codec::decode(ids, accepted.bytes)?;
    if view.graph_id != accepted.graph_id || view.graph_id != checked.id() {
        return Err(crate::belt::types::Refusal::GraphIdentity);
    }
    if view.belt_slots != accepted.belt_slots || view.save_slots != accepted.save_slots {
        return Err(crate::belt::types::Refusal::ArtifactBinding);
    }
    if view.instruction_count != accepted.instruction_count
        || view.instruction_digest != accepted.instruction_digest
    {
        return Err(crate::belt::types::Refusal::InstructionDigest);
    }
    if view.artifact_binding != accepted.artifact_binding {
        return Err(crate::belt::types::Refusal::ArtifactBinding);
    }
    let slot_values = crate::eval::preflight(checked, inputs, None)
        .map_err(|_| crate::belt::types::Refusal::Inputs)?;
    interpret(checked, &view, &slot_values)
}

fn interpret(
    checked: &crate::check::CheckedGraph,
    artifact: &crate::belt::types::ArtifactView<'_>,
    inputs: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<crate::sem::Value>, crate::belt::types::Refusal> {
    let mut belt = RuntimeValues::new(usize::from(artifact.belt_slots));
    let mut slots = alloc::vec![None; usize::from(artifact.save_slots)];
    let mut outputs = alloc::vec::Vec::with_capacity(checked.graph().outputs.len());
    let mut cursor = 0usize;
    while cursor < artifact.instruction_bytes.len() {
        let instruction = crate::belt::codec::instructions::next_instruction(
            artifact.instruction_bytes,
            &mut cursor,
        )?;
        match instruction {
            crate::belt::types::Instruction::Input(slot) => {
                let index =
                    usize::try_from(slot).map_err(|_| crate::belt::types::Refusal::RuntimeFault)?;
                let value = *inputs
                    .get(index)
                    .ok_or(crate::belt::types::Refusal::RuntimeFault)?;
                belt.insert(value);
            }
            crate::belt::types::Instruction::Constant(bits) => {
                let value = i64::from_le_bytes(bits.to_le_bytes());
                belt.insert(crate::sem::Value::I64(value));
            }
            crate::belt::types::Instruction::Add(left, right) => {
                let (a, b) = operands(&belt, left, right)?;
                belt.insert(crate::sem::Value::I64(a.wrapping_add(b)));
            }
            crate::belt::types::Instruction::Sub(left, right) => {
                let (a, b) = operands(&belt, left, right)?;
                belt.insert(crate::sem::Value::I64(a.wrapping_sub(b)));
            }
            crate::belt::types::Instruction::Mul(left, right) => {
                let (a, b) = operands(&belt, left, right)?;
                belt.insert(crate::sem::Value::I64(a.wrapping_mul(b)));
            }
            crate::belt::types::Instruction::Eq(left, right) => {
                let (a, b) = operands(&belt, left, right)?;
                belt.insert(crate::sem::Value::Bool(a == b));
            }
            crate::belt::types::Instruction::Save(age, slot) => {
                let value = *belt
                    .get(age)
                    .ok_or(crate::belt::types::Refusal::RuntimeFault)?;
                let target = slots
                    .get_mut(usize::from(slot))
                    .ok_or(crate::belt::types::Refusal::RuntimeFault)?;
                *target = Some(value);
            }
            crate::belt::types::Instruction::Load(slot) => {
                let value = slots
                    .get(usize::from(slot))
                    .copied()
                    .ok_or(crate::belt::types::Refusal::RuntimeFault)?
                    .ok_or(crate::belt::types::Refusal::RuntimeFault)?;
                belt.insert(value);
            }
            crate::belt::types::Instruction::Output(age) => outputs.push(
                *belt
                    .get(age)
                    .ok_or(crate::belt::types::Refusal::RuntimeFault)?,
            ),
        }
    }
    Ok(outputs)
}

fn operands(
    belt: &RuntimeValues,
    left: u8,
    right: u8,
) -> Result<(i64, i64), crate::belt::types::Refusal> {
    let left = belt
        .get(left)
        .copied()
        .ok_or(crate::belt::types::Refusal::RuntimeFault)?;
    let right = belt
        .get(right)
        .copied()
        .ok_or(crate::belt::types::Refusal::RuntimeFault)?;
    match (left, right) {
        (crate::sem::Value::I64(a), crate::sem::Value::I64(b)) => Ok((a, b)),
        _ => Err(crate::belt::types::Refusal::RuntimeFault),
    }
}

struct RuntimeValues {
    values: alloc::vec::Vec<crate::sem::Value>,
    belt_slots: usize,
}

impl RuntimeValues {
    fn new(belt_slots: usize) -> Self {
        Self {
            values: alloc::vec::Vec::with_capacity(belt_slots),
            belt_slots,
        }
    }

    fn get(&self, age: u8) -> Option<&crate::sem::Value> {
        self.values.get(usize::from(age))
    }

    fn insert(&mut self, value: crate::sem::Value) {
        if self.values.len() == self.belt_slots {
            self.values.pop();
        }
        self.values.insert(0, value);
    }
}
