#[derive(Clone, Copy)]
struct BeltValue {
    node: crate::ir::NodeId,
    value: crate::sem::Value,
}

#[derive(Clone, Copy)]
enum ResultValue {
    Insert(BeltValue),
    Save { slot: usize, value: BeltValue },
}

#[derive(Clone, Copy)]
struct Pending {
    completion_cycle: u32,
    issue_cycle: u32,
    unit: u8,
    value: ResultValue,
}

/// Check and execute a target schedule, publishing outputs only after successful admission.
pub fn execute(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    machine: super::Machine,
    bytes: &[u8],
    inputs: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<crate::sem::Value>, super::Refusal> {
    let accepted = super::check(ids, checked, machine, bytes)?;
    execute_checked(ids, checked, accepted, inputs)
}

/// Interpret a checker-issued schedule, rechecking its exact bound bytes first.
pub fn execute_checked(
    ids: &dyn crate::identity::IdentityScheme,
    checked: &crate::check::CheckedGraph,
    accepted: super::CheckedSchedule<'_>,
    inputs: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<crate::sem::Value>, super::Refusal> {
    let rechecked = super::check(ids, checked, accepted.machine, accepted.bytes)?;
    if rechecked != accepted {
        return Err(super::Refusal::PostCheckMutation);
    }
    let mut schedule = super::decode(ids, accepted.bytes)?;
    let slot_values =
        crate::eval::preflight(checked, inputs, None).map_err(|_| super::Refusal::Inputs)?;
    interpret(checked, &mut schedule, &slot_values)
}

fn interpret(
    checked: &crate::check::CheckedGraph,
    schedule: &mut super::DecodedSchedule,
    slot_values: &[crate::sem::Value],
) -> Result<alloc::vec::Vec<crate::sem::Value>, super::Refusal> {
    let graph = checked.graph();
    let descriptor = schedule.machine.descriptor();
    schedule
        .events
        .sort_unstable_by_key(|event| (event.cycle, event.lane, event.unit));
    let events = &schedule.events;
    let mut belt = alloc::vec::Vec::<BeltValue>::with_capacity(usize::from(descriptor.belt_slots));
    let mut saves = alloc::vec![None::<BeltValue>; usize::from(descriptor.save_slots)];
    let mut pending = alloc::vec::Vec::<Pending>::with_capacity(events.len());
    let max_cycle = events
        .iter()
        .map(|event| {
            event.cycle + u32::from(descriptor.completion_cycles(operation_class(event.operation)))
                - 1
        })
        .max()
        .unwrap_or(0);
    let mut cursor = 0usize;
    for cycle in 0..=max_cycle {
        let start = cursor;
        while cursor < events.len() && events[cursor].cycle == cycle {
            cursor += 1;
        }
        for event in &events[start..cursor] {
            let result = match event.operation {
                super::Operation::Input { node, slot } => {
                    let value = *slot_values
                        .get(slot as usize)
                        .ok_or(super::Refusal::RuntimeFault)?;
                    ResultValue::Insert(BeltValue {
                        node: crate::ir::NodeId(node),
                        value,
                    })
                }
                super::Operation::Constant { node, bits } => ResultValue::Insert(BeltValue {
                    node: crate::ir::NodeId(node),
                    value: crate::sem::Value::I64(i64::from_le_bytes(bits.to_le_bytes())),
                }),
                super::Operation::Add {
                    node,
                    left_age,
                    right_age,
                } => {
                    let (left, right) = operands(&belt, left_age, right_age)?;
                    let (crate::sem::Value::I64(left), crate::sem::Value::I64(right)) =
                        (left.value, right.value)
                    else {
                        return Err(super::Refusal::RuntimeFault);
                    };
                    ResultValue::Insert(BeltValue {
                        node: crate::ir::NodeId(node),
                        value: crate::sem::Value::I64(crate::sem::i64_add(left, right)),
                    })
                }
                super::Operation::Sub {
                    node,
                    left_age,
                    right_age,
                } => {
                    let (left, right) = operands(&belt, left_age, right_age)?;
                    let (crate::sem::Value::I64(left), crate::sem::Value::I64(right)) =
                        (left.value, right.value)
                    else {
                        return Err(super::Refusal::RuntimeFault);
                    };
                    ResultValue::Insert(BeltValue {
                        node: crate::ir::NodeId(node),
                        value: crate::sem::Value::I64(crate::sem::i64_sub(left, right)),
                    })
                }
                super::Operation::Multiply {
                    node,
                    left_age,
                    right_age,
                } => {
                    let (left, right) = operands(&belt, left_age, right_age)?;
                    let (crate::sem::Value::I64(left), crate::sem::Value::I64(right)) =
                        (left.value, right.value)
                    else {
                        return Err(super::Refusal::RuntimeFault);
                    };
                    ResultValue::Insert(BeltValue {
                        node: crate::ir::NodeId(node),
                        value: crate::sem::Value::I64(crate::sem::i64_mul(left, right)),
                    })
                }
                super::Operation::Equal {
                    node,
                    left_age,
                    right_age,
                } => {
                    let (left, right) = operands(&belt, left_age, right_age)?;
                    let (crate::sem::Value::I64(left), crate::sem::Value::I64(right)) =
                        (left.value, right.value)
                    else {
                        return Err(super::Refusal::RuntimeFault);
                    };
                    ResultValue::Insert(BeltValue {
                        node: crate::ir::NodeId(node),
                        value: crate::sem::Value::Bool(left == right),
                    })
                }
                super::Operation::Save { node, age, slot } => {
                    let value = *belt
                        .get(usize::from(age))
                        .ok_or(super::Refusal::RuntimeFault)?;
                    if value.node.0 != node {
                        return Err(super::Refusal::RuntimeFault);
                    }
                    ResultValue::Save {
                        slot: usize::from(slot),
                        value,
                    }
                }
                super::Operation::Load { node, slot } => {
                    let value = saves
                        .get(usize::from(slot))
                        .copied()
                        .flatten()
                        .ok_or(super::Refusal::RuntimeFault)?;
                    if value.node.0 != node {
                        return Err(super::Refusal::RuntimeFault);
                    }
                    ResultValue::Insert(value)
                }
            };
            let due = event.cycle
                + u32::from(descriptor.completion_cycles(operation_class(event.operation)))
                - 1;
            pending.push(Pending {
                completion_cycle: due,
                issue_cycle: event.cycle,
                unit: event.unit,
                value: result,
            });
        }
        pending
            .sort_unstable_by_key(|event| (event.completion_cycle, event.issue_cycle, event.unit));
        let mut done = 0usize;
        while done < pending.len() && pending[done].completion_cycle == cycle {
            match pending[done].value {
                ResultValue::Insert(value) => {
                    belt = insert(value, belt, descriptor.belt_slots)?;
                }
                ResultValue::Save { slot, value } => {
                    let target = saves.get_mut(slot).ok_or(super::Refusal::RuntimeFault)?;
                    *target = Some(value);
                }
            }
            done += 1;
        }
        pending.drain(..done);
    }
    if cursor != events.len() || !pending.is_empty() {
        return Err(super::Refusal::RuntimeFault);
    }
    let mut outputs = alloc::vec::Vec::with_capacity(schedule.outputs.len());
    for output in &schedule.outputs {
        let value = belt
            .iter()
            .find(|value| value.node.0 == output.node)
            .ok_or(super::Refusal::OutputMismatch)?;
        outputs.push(value.value);
    }
    if graph.outputs.len() != outputs.len() {
        return Err(super::Refusal::OutputMismatch);
    }
    Ok(outputs)
}

fn operands(
    belt: &[BeltValue],
    left_age: u8,
    right_age: u8,
) -> Result<(BeltValue, BeltValue), super::Refusal> {
    let left = *belt
        .get(usize::from(left_age))
        .ok_or(super::Refusal::RuntimeFault)?;
    let right = *belt
        .get(usize::from(right_age))
        .ok_or(super::Refusal::RuntimeFault)?;
    Ok((left, right))
}

fn insert(
    value: BeltValue,
    mut belt: alloc::vec::Vec<BeltValue>,
    belt_slot_count: u8,
) -> Result<alloc::vec::Vec<BeltValue>, super::Refusal> {
    if belt.len() == usize::from(belt_slot_count) {
        belt.pop().ok_or(super::Refusal::RuntimeFault)?;
    }
    belt.insert(0, value);
    Ok(belt)
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
