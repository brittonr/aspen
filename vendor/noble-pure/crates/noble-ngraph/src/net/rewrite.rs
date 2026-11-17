//! Versioned, checked byte plans for revision-3 local interactions.
//!
//! The compiler and decoder/checker are intentionally separate: the checker
//! decodes bytes and reconstructs the selected rule from the live arena.

const MAGIC: [u8; 4] = *b"NPLN";
const VERSION: u8 = 1;
const COMMIT: u8 = 0xc7;
pub(super) const MAX_PLAN_BYTES: usize = 256;
const MAX_BOUNDARIES: usize = 2;
const MAX_RESULTS: usize = 2;
const MAX_WRITES: usize = 2;

/// Fixed-size storage means producing or decoding a plan does not allocate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanBytes {
    bytes: [u8; MAX_PLAN_BYTES],
    len: usize,
}

impl PlanBytes {
    /// Exact emitted bytes, excluding unused fixed-buffer capacity.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    #[cfg(test)]
    pub(super) fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.bytes[..self.len]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Boundary {
    source: super::PortRef,
    external: super::Endpoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResultAgent {
    slot: u32,
    fresh: bool,
    symbol: super::Symbol,
    donor_slot: u32,
    ports: [Option<u8>; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WireWrite {
    result: u8,
    port_index: super::PortIndex,
    boundary: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Active {
    first: u32,
    first_generation: u64,
    second: u32,
    second_generation: u64,
}

#[derive(Clone, Copy)]
struct RuleInstance {
    opcode: u8,
    action: [u8; 2],
    boundaries: [Option<Boundary>; MAX_BOUNDARIES],
    boundary_count: u8,
    results: [Option<ResultAgent>; MAX_RESULTS],
    result_count: u8,
    writes: [Option<WireWrite>; MAX_WRITES],
    write_count: u8,
    publications: [Option<Active>; MAX_RESULTS],
    publication_count: u8,
}

struct RuleContext<'a> {
    slots: &'a [super::Slot],
    outputs: &'a [super::PortRef],
    pair: super::Pair,
    policy: super::StoragePolicy,
}

struct ResultRequest {
    donor_slot: u32,
    reused_slot: Option<u32>,
    symbol: super::Symbol,
    ports: [Option<u8>; 3],
}

pub(super) struct CheckRequest<'a> {
    pub(super) bytes: &'a PlanBytes,
    pub(super) slots: &'a [super::Slot],
    pub(super) outputs: &'a [super::PortRef],
    pub(super) worklist: &'a super::ActiveWorklist,
    pub(super) pair: super::Pair,
    pub(super) policy: super::StoragePolicy,
}

#[derive(Clone, Copy)]
struct Image {
    mode: super::StoragePolicy,
    pair: super::Pair,
    opcode: u8,
    action: [u8; 2],
    boundaries: [Option<Boundary>; MAX_BOUNDARIES],
    boundary_count: u8,
    results: [Option<ResultAgent>; MAX_RESULTS],
    result_count: u8,
    writes: [Option<WireWrite>; MAX_WRITES],
    write_count: u8,
    external_writes: [Option<WireWrite>; MAX_WRITES],
    external_write_count: u8,
    arena_reservation: u32,
    scratch_reservation: u32,
    queue_reservation: u32,
    publications: [Option<Active>; MAX_RESULTS],
    publication_count: u8,
}

pub(super) struct CheckedPlan {
    image: Image,
    pub(super) arena_reservation: u32,
    pub(super) scratch_reservation: u32,
    pub(super) queue_reservation: u32,
    pub(super) reused_slots: u32,
    pub(super) freed_slots: u32,
    pub(super) live_decrease: u32,
}

pub(super) enum ExecuteError {
    Invalid(super::Invalid),
    Limited {
        limit: super::Limit,
        cap: u64,
        observed: u64,
    },
}

pub(super) struct Execution {
    pub(super) live_decrease: u32,
    pub(super) transient_live_nodes: u32,
    pub(super) reused_slots: u32,
    pub(super) freed_slots: u32,
    pub(super) allocated_slots: u32,
    pub(super) microsteps: u32,
}

#[derive(Clone, Copy)]
pub(super) struct ExecutionLimits {
    pub(super) arena_slots: u64,
    pub(super) scratch_bytes: u64,
    pub(super) worklist_entries: u64,
}

struct ExecutionHooks<'a> {
    interrupt_after: Option<&'a dyn Fn(u8) -> bool>,
    reservation_failure_after: Option<u8>,
}

pub(super) struct ExecutionState<'a> {
    pub(super) slots: &'a mut alloc::vec::Vec<super::Slot>,
    pub(super) outputs: &'a mut [super::PortRef],
    pub(super) worklist: &'a mut super::ActiveWorklist,
}

struct RollbackJournal {
    old_slot_len: usize,
    old_queue_len: usize,
    slot_backups: [Option<(usize, super::Slot)>; 4],
    slot_backup_count: usize,
    output_backups: [Option<(usize, super::PortRef)>; MAX_BOUNDARIES],
    output_backup_count: usize,
}

struct Writer {
    bytes: [u8; MAX_PLAN_BYTES],
    len: usize,
}

impl Writer {
    fn new() -> Self {
        Self {
            bytes: [0; MAX_PLAN_BYTES],
            len: 0,
        }
    }

    fn put(&mut self, value: u8) -> Result<(), super::Invalid> {
        let Some(target) = self.bytes.get_mut(self.len) else {
            return Err(super::Invalid::PlanEncoding);
        };
        *target = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(super::Invalid::CounterOverflow)?;
        Ok(())
    }

    fn put_u32(&mut self, value: u32) -> Result<(), super::Invalid> {
        for byte in value.to_le_bytes() {
            self.put(byte)?;
        }
        Ok(())
    }

    fn put_u64(&mut self, value: u64) -> Result<(), super::Invalid> {
        for byte in value.to_le_bytes() {
            self.put(byte)?;
        }
        Ok(())
    }

    fn put_endpoint(&mut self, endpoint: super::Endpoint) -> Result<(), super::Invalid> {
        match endpoint {
            super::Endpoint::Port(port) => {
                self.put(0)?;
                self.put_u32(port.agent)?;
                self.put(u8::from(port.port_index))
            }
            super::Endpoint::Output(output) => {
                self.put(1)?;
                self.put_u32(output)
            }
        }
    }

    fn finish(self) -> PlanBytes {
        PlanBytes {
            bytes: self.bytes,
            len: self.len,
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn get(&mut self) -> Result<u8, super::Invalid> {
        let value = *self
            .bytes
            .get(self.cursor)
            .ok_or(super::Invalid::PlanEncoding)?;
        self.cursor = self
            .cursor
            .checked_add(1)
            .ok_or(super::Invalid::CounterOverflow)?;
        Ok(value)
    }

    fn get_u32(&mut self) -> Result<u32, super::Invalid> {
        let mut bytes = [0; 4];
        for byte in &mut bytes {
            *byte = self.get()?;
        }
        Ok(u32::from_le_bytes(bytes))
    }

    fn get_u64(&mut self) -> Result<u64, super::Invalid> {
        let mut bytes = [0; 8];
        for byte in &mut bytes {
            *byte = self.get()?;
        }
        Ok(u64::from_le_bytes(bytes))
    }

    fn get_endpoint(&mut self) -> Result<super::Endpoint, super::Invalid> {
        match self.get()? {
            0 => Ok(super::Endpoint::Port(super::PortRef {
                agent: self.get_u32()?,
                port_index: super::PortIndex::from(self.get()?),
            })),
            1 => Ok(super::Endpoint::Output(self.get_u32()?)),
            _ => Err(super::Invalid::PlanEncoding),
        }
    }
}

fn op_tag(op: super::Op) -> u8 {
    match op {
        super::Op::Add => 0,
        super::Op::Sub => 1,
        super::Op::Mul => 2,
        super::Op::Eq => 3,
    }
}

fn decode_op(tag: u8) -> Result<super::Op, super::Invalid> {
    match tag {
        0 => Ok(super::Op::Add),
        1 => Ok(super::Op::Sub),
        2 => Ok(super::Op::Mul),
        3 => Ok(super::Op::Eq),
        _ => Err(super::Invalid::PlanEncoding),
    }
}

fn symbol_tag(symbol: super::Symbol) -> (u8, u64) {
    match symbol {
        super::Symbol::Num(value) => (0, value),
        super::Symbol::Op2(op) => (1 + op_tag(op), 0),
        super::Symbol::Op1(op, first) => (5 + op_tag(op), first),
        super::Symbol::Dup => (9, 0),
        super::Symbol::Era => (10, 0),
    }
}

fn decode_symbol(tag: u8, payload: u64) -> Result<super::Symbol, super::Invalid> {
    match tag {
        0 => Ok(super::Symbol::Num(payload)),
        1..=4 if payload == 0 => Ok(super::Symbol::Op2(decode_op(tag - 1)?)),
        5..=8 => Ok(super::Symbol::Op1(decode_op(tag - 5)?, payload)),
        9 if payload == 0 => Ok(super::Symbol::Dup),
        10 if payload == 0 => Ok(super::Symbol::Era),
        _ => Err(super::Invalid::PlanEncoding),
    }
}

fn slot_agent(slots: &[super::Slot], index: u32) -> Result<super::RuntimeAgent, super::Invalid> {
    slots
        .get(index as usize)
        .and_then(|slot| slot.agent)
        .ok_or(super::Invalid::StalePlanSlot)
}

fn active_pair(
    slots: &[super::Slot],
    pair: super::Pair,
) -> Result<(super::RuntimeAgent, super::RuntimeAgent), super::Invalid> {
    let first = slots
        .get(pair.first as usize)
        .ok_or(super::Invalid::StalePlanSlot)?;
    let second = slots
        .get(pair.second as usize)
        .ok_or(super::Invalid::StalePlanSlot)?;
    if first.generation != pair.first_generation || second.generation != pair.second_generation {
        return Err(super::Invalid::StalePlanSlot);
    }
    let a = first.agent.ok_or(super::Invalid::StalePlanSlot)?;
    let b = second.agent.ok_or(super::Invalid::StalePlanSlot)?;
    if a.ports[0]
        != Some(super::Endpoint::Port(super::PortRef {
            agent: pair.second,
            port_index: super::PortIndex::from(0),
        }))
        || b.ports[0]
            != Some(super::Endpoint::Port(super::PortRef {
                agent: pair.first,
                port_index: super::PortIndex::from(0),
            }))
    {
        return Err(super::Invalid::StalePlanSlot);
    }
    Ok((a, b))
}

fn boundary(
    context: &RuleContext<'_>,
    owner: u32,
    port_index: super::PortIndex,
    instance: &mut RuleInstance,
) -> Result<(), super::Invalid> {
    let slots = context.slots;
    let outputs = context.outputs;
    let pair = context.pair;
    let source_agent = slot_agent(slots, owner)?;
    let external = source_agent
        .ports
        .get(usize::from(port_index))
        .copied()
        .flatten()
        .ok_or(super::Invalid::PortArity)?;
    match external {
        super::Endpoint::Port(target) => {
            if target.agent == pair.first || target.agent == pair.second {
                return Err(super::Invalid::PlanBoundaryAlias);
            }
            let target_agent = slot_agent(slots, target.agent)?;
            if usize::from(target.port_index) >= target_agent.symbol.port_count()
                || target_agent.ports[usize::from(target.port_index)]
                    != Some(super::Endpoint::Port(super::PortRef {
                        agent: owner,
                        port_index,
                    }))
            {
                return Err(super::Invalid::NonReciprocal);
            }
        }
        super::Endpoint::Output(index) => {
            if outputs.get(index as usize)
                != Some(&super::PortRef {
                    agent: owner,
                    port_index,
                })
            {
                return Err(super::Invalid::OutputBoundary);
            }
        }
    }
    for prior in instance.boundaries.iter().flatten() {
        if prior.external == external {
            return Err(super::Invalid::PlanBoundaryAlias);
        }
    }
    let index = usize::from(instance.boundary_count);
    *instance
        .boundaries
        .get_mut(index)
        .ok_or(super::Invalid::PlanEncoding)? = Some(Boundary {
        source: super::PortRef {
            agent: owner,
            port_index,
        },
        external,
    });
    instance.boundary_count = instance
        .boundary_count
        .checked_add(1)
        .ok_or(super::Invalid::CounterOverflow)?;
    Ok(())
}

fn classify(context: &RuleContext<'_>) -> Result<RuleInstance, super::Invalid> {
    let slots = context.slots;
    let pair = context.pair;
    let policy = context.policy;
    let (first, second) = active_pair(slots, pair)?;
    let mut instance = RuleInstance {
        opcode: 0,
        action: [0; 2],
        boundaries: [None; MAX_BOUNDARIES],
        boundary_count: 0,
        results: [None; MAX_RESULTS],
        result_count: 0,
        writes: [None; MAX_WRITES],
        write_count: 0,
        publications: [None; MAX_RESULTS],
        publication_count: 0,
    };
    let (rule_slot, value_slot, opcode, payload) = match (first.symbol, second.symbol) {
        (super::Symbol::Op2(_), super::Symbol::Num(value)) => (pair.first, pair.second, 1, value),
        (super::Symbol::Num(value), super::Symbol::Op2(_)) => (pair.second, pair.first, 1, value),
        (super::Symbol::Op1(_, _), super::Symbol::Num(second_value)) => {
            (pair.first, pair.second, 2, second_value)
        }
        (super::Symbol::Num(second_value), super::Symbol::Op1(_, _)) => {
            (pair.second, pair.first, 2, second_value)
        }
        (super::Symbol::Dup, super::Symbol::Num(value)) => (pair.first, pair.second, 3, value),
        (super::Symbol::Num(value), super::Symbol::Dup) => (pair.second, pair.first, 3, value),
        (super::Symbol::Era, super::Symbol::Num(_)) => (pair.first, pair.second, 4, 0),
        (super::Symbol::Num(_), super::Symbol::Era) => (pair.second, pair.first, 4, 0),
        _ => return Err(super::Invalid::StuckPair),
    };
    instance.opcode = opcode;
    let result_slots = match policy {
        super::StoragePolicy::Reuse => [Some(rule_slot), None],
        super::StoragePolicy::Allocate => [None, None],
    };
    match opcode {
        1 => {
            boundary(context, rule_slot, super::PortIndex::from(1), &mut instance)?;
            boundary(context, rule_slot, super::PortIndex::from(2), &mut instance)?;
            let op = match slot_agent(slots, rule_slot)?.symbol {
                super::Symbol::Op2(op) => op,
                _ => return Err(super::Invalid::PlanMismatch),
            };
            add_result(
                &mut instance,
                context,
                ResultRequest {
                    donor_slot: rule_slot,
                    reused_slot: result_slots[0],
                    symbol: super::Symbol::Op1(op, payload),
                    ports: [Some(0), Some(1), None],
                },
            )?;
            instance.action = consumed_actions(pair, [rule_slot], policy)?;
        }
        2 => {
            boundary(context, rule_slot, super::PortIndex::from(1), &mut instance)?;
            let (op, first_value) = match slot_agent(slots, rule_slot)?.symbol {
                super::Symbol::Op1(op, first_value) => (op, first_value),
                _ => return Err(super::Invalid::PlanMismatch),
            };
            let second_value = match slot_agent(slots, value_slot)?.symbol {
                super::Symbol::Num(value) => value,
                _ => return Err(super::Invalid::PlanMismatch),
            };
            add_result(
                &mut instance,
                context,
                ResultRequest {
                    donor_slot: rule_slot,
                    reused_slot: result_slots[0],
                    symbol: super::Symbol::Num(apply(op, first_value, second_value)),
                    ports: [Some(0), None, None],
                },
            )?;
            instance.action = consumed_actions(pair, [rule_slot], policy)?;
        }
        3 => {
            boundary(context, rule_slot, super::PortIndex::from(1), &mut instance)?;
            boundary(context, rule_slot, super::PortIndex::from(2), &mut instance)?;
            let second_destination = match policy {
                super::StoragePolicy::Reuse => Some(value_slot),
                super::StoragePolicy::Allocate => None,
            };
            add_result(
                &mut instance,
                context,
                ResultRequest {
                    donor_slot: rule_slot,
                    reused_slot: result_slots[0],
                    symbol: super::Symbol::Num(payload),
                    ports: [Some(0), None, None],
                },
            )?;
            add_result(
                &mut instance,
                context,
                ResultRequest {
                    donor_slot: value_slot,
                    reused_slot: second_destination,
                    symbol: super::Symbol::Num(payload),
                    ports: [Some(1), None, None],
                },
            )?;
            instance.action = consumed_actions(pair, [rule_slot, value_slot], policy)?;
        }
        4 => {
            instance.action = [0; 2];
        }
        _ => return Err(super::Invalid::PlanEncoding),
    }
    populate_writes_and_publications(slots, pair, &mut instance)?;
    Ok(instance)
}

fn consumed_actions<const N: usize>(
    pair: super::Pair,
    reused: [u32; N],
    policy: super::StoragePolicy,
) -> Result<[u8; 2], super::Invalid> {
    let mut actions = [0; 2];
    for id in reused {
        let index = if pair.first == id {
            0
        } else if pair.second == id {
            1
        } else {
            return Err(super::Invalid::PlanMismatch);
        };
        actions[index] = u8::from(policy == super::StoragePolicy::Reuse);
    }
    Ok(actions)
}

fn add_result(
    instance: &mut RuleInstance,
    context: &RuleContext<'_>,
    request: ResultRequest,
) -> Result<(), super::Invalid> {
    let slots = context.slots;
    let policy = context.policy;
    let ResultRequest {
        donor_slot,
        reused_slot,
        symbol,
        ports,
    } = request;
    let index = usize::from(instance.result_count);
    let destination = match reused_slot {
        Some(slot) => slot,
        None => {
            let ordinal = u32::from(instance.result_count);
            u32::try_from(slots.len())
                .map_err(|_| super::Invalid::CounterOverflow)?
                .checked_add(ordinal)
                .ok_or(super::Invalid::CounterOverflow)?
        }
    };
    let result = ResultAgent {
        slot: destination,
        fresh: policy == super::StoragePolicy::Allocate,
        symbol,
        donor_slot,
        ports,
    };
    *instance
        .results
        .get_mut(index)
        .ok_or(super::Invalid::PlanEncoding)? = Some(result);
    instance.result_count = instance
        .result_count
        .checked_add(1)
        .ok_or(super::Invalid::CounterOverflow)?;
    Ok(())
}

fn populate_writes_and_publications(
    slots: &[super::Slot],
    pair: super::Pair,
    instance: &mut RuleInstance,
) -> Result<(), super::Invalid> {
    for result_index in 0..usize::from(instance.result_count) {
        let result = instance.results[result_index].ok_or(super::Invalid::PlanEncoding)?;
        for (port_index, boundary_index) in result.ports.iter().enumerate() {
            let Some(boundary_index) = boundary_index else {
                continue;
            };
            let write_index = usize::from(instance.write_count);
            let write = WireWrite {
                result: u8::try_from(result_index).map_err(|_| super::Invalid::CounterOverflow)?,
                port_index: super::PortIndex::from(
                    u8::try_from(port_index).map_err(|_| super::Invalid::CounterOverflow)?,
                ),
                boundary: *boundary_index,
            };
            *instance
                .writes
                .get_mut(write_index)
                .ok_or(super::Invalid::PlanEncoding)? = Some(write);
            instance.write_count = instance
                .write_count
                .checked_add(1)
                .ok_or(super::Invalid::CounterOverflow)?;
            if port_index == 0 {
                maybe_publish_active(slots, pair, result, *boundary_index, instance)?;
            }
        }
    }
    Ok(())
}

fn maybe_publish_active(
    slots: &[super::Slot],
    pair: super::Pair,
    result: ResultAgent,
    boundary_index: u8,
    instance: &mut RuleInstance,
) -> Result<(), super::Invalid> {
    let Some(super::Endpoint::Port(target)) = instance
        .boundaries
        .get(usize::from(boundary_index))
        .and_then(|boundary| *boundary)
        .map(|boundary| boundary.external)
    else {
        return Ok(());
    };
    if target.port_index != super::PortIndex::from(0) {
        return Ok(());
    }
    if target.agent == pair.first || target.agent == pair.second {
        return Ok(());
    }
    let target_slot = slots
        .get(target.agent as usize)
        .ok_or(super::Invalid::StalePlanSlot)?;
    let destination_generation = if result.fresh {
        0
    } else {
        slots
            .get(result.slot as usize)
            .ok_or(super::Invalid::StalePlanSlot)?
            .generation
            .checked_add(1)
            .ok_or(super::Invalid::CounterOverflow)?
    };
    let (first, first_generation, second, second_generation) = if result.slot < target.agent {
        (
            result.slot,
            destination_generation,
            target.agent,
            target_slot.generation,
        )
    } else {
        (
            target.agent,
            target_slot.generation,
            result.slot,
            destination_generation,
        )
    };
    let publication_index = usize::from(instance.publication_count);
    *instance
        .publications
        .get_mut(publication_index)
        .ok_or(super::Invalid::PlanEncoding)? = Some(Active {
        first,
        first_generation,
        second,
        second_generation,
    });
    instance.publication_count = instance
        .publication_count
        .checked_add(1)
        .ok_or(super::Invalid::CounterOverflow)?;
    Ok(())
}

/// Compile the selected active pair into actual versioned bytes.
pub(super) fn compile(
    slots: &[super::Slot],
    outputs: &[super::PortRef],
    pair: super::Pair,
    policy: super::StoragePolicy,
) -> Result<PlanBytes, super::Invalid> {
    let context = RuleContext {
        slots,
        outputs,
        pair,
        policy,
    };
    let instance = classify(&context)?;
    let mut writer = Writer::new();
    for byte in MAGIC {
        writer.put(byte)?;
    }
    writer.put(VERSION)?;
    writer.put(match policy {
        super::StoragePolicy::Reuse => 0,
        super::StoragePolicy::Allocate => 1,
    })?;
    writer.put(instance.opcode)?;
    writer.put_u32(pair.first)?;
    writer.put_u64(pair.first_generation)?;
    writer.put_u32(pair.second)?;
    writer.put_u64(pair.second_generation)?;
    writer.put(instance.action[0])?;
    writer.put(instance.action[1])?;
    writer.put(instance.boundary_count)?;
    for boundary in instance
        .boundaries
        .iter()
        .take(usize::from(instance.boundary_count))
    {
        let boundary = boundary.ok_or(super::Invalid::PlanEncoding)?;
        writer.put_u32(boundary.source.agent)?;
        writer.put(u8::from(boundary.source.port_index))?;
        writer.put_endpoint(boundary.external)?;
    }
    writer.put(instance.result_count)?;
    let mut fresh = 0u32;
    for result in instance
        .results
        .iter()
        .take(usize::from(instance.result_count))
    {
        let result = result.ok_or(super::Invalid::PlanEncoding)?;
        writer.put(u8::from(result.fresh))?;
        writer.put_u32(result.slot)?;
        let (tag, payload) = symbol_tag(result.symbol);
        writer.put(tag)?;
        writer.put_u64(payload)?;
        writer.put_u32(result.donor_slot)?;
        for boundary in result.ports {
            writer.put(boundary.unwrap_or(u8::MAX))?;
        }
        if result.fresh {
            fresh = fresh
                .checked_add(1)
                .ok_or(super::Invalid::CounterOverflow)?;
        }
    }
    writer.put(instance.write_count)?;
    for write in instance
        .writes
        .iter()
        .take(usize::from(instance.write_count))
    {
        let write = write.ok_or(super::Invalid::PlanEncoding)?;
        writer.put(write.result)?;
        writer.put(u8::from(write.port_index))?;
        writer.put(write.boundary)?;
    }
    writer.put(instance.write_count)?;
    for write in instance
        .writes
        .iter()
        .take(usize::from(instance.write_count))
    {
        let write = write.ok_or(super::Invalid::PlanEncoding)?;
        writer.put(write.result)?;
        writer.put(u8::from(write.port_index))?;
        writer.put(write.boundary)?;
    }
    writer.put_u32(fresh)?;
    writer.put_u32(u32::try_from(MAX_PLAN_BYTES).map_err(|_| super::Invalid::CounterOverflow)?)?;
    writer.put_u32(u32::from(instance.publication_count))?;
    writer.put(instance.publication_count)?;
    for active in instance
        .publications
        .iter()
        .take(usize::from(instance.publication_count))
    {
        let active = active.ok_or(super::Invalid::PlanEncoding)?;
        writer.put_u32(active.first)?;
        writer.put_u64(active.first_generation)?;
        writer.put_u32(active.second)?;
        writer.put_u64(active.second_generation)?;
    }
    writer.put(COMMIT)?;
    Ok(writer.finish())
}

fn decode(bytes: &[u8]) -> Result<Image, super::Invalid> {
    if bytes.len() > MAX_PLAN_BYTES {
        return Err(super::Invalid::PlanEncoding);
    }
    let mut reader = Reader::new(bytes);
    for expected in MAGIC {
        if reader.get()? != expected {
            return Err(super::Invalid::PlanEncoding);
        }
    }
    if reader.get()? != VERSION {
        return Err(super::Invalid::PlanVersion);
    }
    let mode = match reader.get()? {
        0 => super::StoragePolicy::Reuse,
        1 => super::StoragePolicy::Allocate,
        _ => return Err(super::Invalid::PlanEncoding),
    };
    let opcode = reader.get()?;
    let pair = super::Pair {
        first: reader.get_u32()?,
        first_generation: reader.get_u64()?,
        second: reader.get_u32()?,
        second_generation: reader.get_u64()?,
    };
    let action = [reader.get()?, reader.get()?];
    if action.iter().any(|value| *value > 1) {
        return Err(super::Invalid::PlanEncoding);
    }
    let boundary_count = reader.get()?;
    if usize::from(boundary_count) > MAX_BOUNDARIES {
        return Err(super::Invalid::PlanEncoding);
    }
    let mut boundaries = [None; MAX_BOUNDARIES];
    for boundary in boundaries.iter_mut().take(usize::from(boundary_count)) {
        *boundary = Some(Boundary {
            source: super::PortRef {
                agent: reader.get_u32()?,
                port_index: super::PortIndex::from(reader.get()?),
            },
            external: reader.get_endpoint()?,
        });
    }
    let result_count = reader.get()?;
    if usize::from(result_count) > MAX_RESULTS {
        return Err(super::Invalid::PlanEncoding);
    }
    let mut results = [None; MAX_RESULTS];
    for result in results.iter_mut().take(usize::from(result_count)) {
        let is_fresh = match reader.get()? {
            0 => false,
            1 => true,
            _ => return Err(super::Invalid::PlanEncoding),
        };
        let slot = reader.get_u32()?;
        let symbol = decode_symbol(reader.get()?, reader.get_u64()?)?;
        let donor_slot = reader.get_u32()?;
        let mut ports = [None; 3];
        for port in &mut ports {
            let value = reader.get()?;
            *port = if value == u8::MAX { None } else { Some(value) };
        }
        *result = Some(ResultAgent {
            slot,
            fresh: is_fresh,
            symbol,
            donor_slot,
            ports,
        });
    }
    let write_count = reader.get()?;
    if usize::from(write_count) > MAX_WRITES {
        return Err(super::Invalid::PlanEncoding);
    }
    let mut writes = [None; MAX_WRITES];
    for write in writes.iter_mut().take(usize::from(write_count)) {
        *write = Some(WireWrite {
            result: reader.get()?,
            port_index: super::PortIndex::from(reader.get()?),
            boundary: reader.get()?,
        });
    }
    let external_write_count = reader.get()?;
    if usize::from(external_write_count) > MAX_WRITES {
        return Err(super::Invalid::PlanEncoding);
    }
    let mut external_writes = [None; MAX_WRITES];
    for write in external_writes
        .iter_mut()
        .take(usize::from(external_write_count))
    {
        *write = Some(WireWrite {
            result: reader.get()?,
            port_index: super::PortIndex::from(reader.get()?),
            boundary: reader.get()?,
        });
    }
    let arena_reservation = reader.get_u32()?;
    let scratch_reservation = reader.get_u32()?;
    let queue_reservation = reader.get_u32()?;
    let publication_count = reader.get()?;
    if usize::from(publication_count) > MAX_RESULTS {
        return Err(super::Invalid::PlanEncoding);
    }
    let mut publications = [None; MAX_RESULTS];
    for active in publications.iter_mut().take(usize::from(publication_count)) {
        *active = Some(Active {
            first: reader.get_u32()?,
            first_generation: reader.get_u64()?,
            second: reader.get_u32()?,
            second_generation: reader.get_u64()?,
        });
    }
    if reader.get()? != COMMIT || reader.cursor != bytes.len() {
        return Err(super::Invalid::PlanEncoding);
    }
    Ok(Image {
        mode,
        pair,
        opcode,
        action,
        boundaries,
        boundary_count,
        results,
        result_count,
        writes,
        write_count,
        external_writes,
        external_write_count,
        arena_reservation,
        scratch_reservation,
        queue_reservation,
        publications,
        publication_count,
    })
}

fn expected_symbol(
    opcode: u8,
    slots: &[super::Slot],
    pair: super::Pair,
) -> Result<(u32, u32, super::Symbol), super::Invalid> {
    let (left, right) = active_pair(slots, pair)?;
    match (left.symbol, right.symbol) {
        (super::Symbol::Op2(op), super::Symbol::Num(value)) => {
            Ok((pair.first, pair.second, super::Symbol::Op1(op, value)))
        }
        (super::Symbol::Num(value), super::Symbol::Op2(op)) => {
            Ok((pair.second, pair.first, super::Symbol::Op1(op, value)))
        }
        (super::Symbol::Op1(op, first), super::Symbol::Num(second)) if opcode == 2 => Ok((
            pair.first,
            pair.second,
            super::Symbol::Num(apply(op, first, second)),
        )),
        (super::Symbol::Num(second), super::Symbol::Op1(op, first)) if opcode == 2 => Ok((
            pair.second,
            pair.first,
            super::Symbol::Num(apply(op, first, second)),
        )),
        (super::Symbol::Dup, super::Symbol::Num(value)) => {
            Ok((pair.first, pair.second, super::Symbol::Num(value)))
        }
        (super::Symbol::Num(value), super::Symbol::Dup) => {
            Ok((pair.second, pair.first, super::Symbol::Num(value)))
        }
        (super::Symbol::Era, super::Symbol::Num(_)) => {
            Ok((pair.first, pair.second, super::Symbol::Num(0)))
        }
        (super::Symbol::Num(_), super::Symbol::Era) => {
            Ok((pair.second, pair.first, super::Symbol::Num(0)))
        }
        _ => Err(super::Invalid::PlanMismatch),
    }
}

fn validate_image(
    image: &Image,
    slots: &[super::Slot],
    outputs: &[super::PortRef],
    pair: super::Pair,
    policy: super::StoragePolicy,
) -> Result<CheckedPlan, super::Invalid> {
    if image.pair != pair || image.mode != policy {
        return Err(super::Invalid::StalePlanSlot);
    }
    let (first, second) = active_pair(slots, pair)?;
    let rule = match (first.symbol, second.symbol) {
        (super::Symbol::Op2(_), super::Symbol::Num(_))
        | (super::Symbol::Num(_), super::Symbol::Op2(_)) => 1,
        (super::Symbol::Op1(_, _), super::Symbol::Num(_))
        | (super::Symbol::Num(_), super::Symbol::Op1(_, _)) => 2,
        (super::Symbol::Dup, super::Symbol::Num(_))
        | (super::Symbol::Num(_), super::Symbol::Dup) => 3,
        (super::Symbol::Era, super::Symbol::Num(_))
        | (super::Symbol::Num(_), super::Symbol::Era) => 4,
        _ => return Err(super::Invalid::StuckPair),
    };
    if image.opcode != rule {
        return Err(super::Invalid::PlanMismatch);
    }
    let (rule_slot, value_slot, result_symbol) = expected_symbol(rule, slots, pair)?;
    let expected_boundary_ports: &[(u32, u8)] = match rule {
        1 => &[(rule_slot, 1), (rule_slot, 2)],
        2 => &[(rule_slot, 1)],
        3 => &[(rule_slot, 1), (rule_slot, 2)],
        4 => &[],
        _ => return Err(super::Invalid::PlanMismatch),
    };
    if usize::from(image.boundary_count) != expected_boundary_ports.len() {
        return Err(super::Invalid::PlanMismatch);
    }
    for (index, (owner, port)) in expected_boundary_ports.iter().enumerate() {
        let expected = Boundary {
            source: super::PortRef {
                agent: *owner,
                port_index: super::PortIndex::from(*port),
            },
            external: slot_agent(slots, *owner)?.ports[usize::from(*port)]
                .ok_or(super::Invalid::PortArity)?,
        };
        if image.boundaries.get(index).copied().flatten() != Some(expected) {
            return Err(super::Invalid::PlanMismatch);
        }
        validate_boundary_independent(slots, outputs, pair, expected)?;
    }
    for prior_index in 0..usize::from(image.boundary_count) {
        let prior = image.boundaries[prior_index].ok_or(super::Invalid::PlanEncoding)?;
        for later in image
            .boundaries
            .iter()
            .skip(prior_index + 1)
            .take(usize::from(image.boundary_count) - prior_index - 1)
            .flatten()
        {
            if prior.external == later.external {
                return Err(super::Invalid::PlanBoundaryAlias);
            }
        }
    }
    let expected_results = match (rule, policy) {
        (1, super::StoragePolicy::Reuse) | (2, super::StoragePolicy::Reuse) => 1,
        (1, super::StoragePolicy::Allocate) | (2, super::StoragePolicy::Allocate) => 1,
        (3, _) => 2,
        (4, _) => 0,
        _ => return Err(super::Invalid::PlanMismatch),
    };
    if usize::from(image.result_count) != expected_results {
        return Err(super::Invalid::PlanMismatch);
    }
    let mut consumed_reused = [false; 2];
    let mut fresh_count = 0u32;
    for (index, result) in image.results.iter().take(expected_results).enumerate() {
        let result = result.ok_or(super::Invalid::PlanEncoding)?;
        if result.symbol != result_symbol {
            return Err(super::Invalid::PlanMismatch);
        }
        let expected_donor = if rule == 3 && index == 1 {
            value_slot
        } else {
            rule_slot
        };
        if result.donor_slot != expected_donor {
            return Err(super::Invalid::PlanMismatch);
        }
        let expected_ports: [Option<u8>; 3] = match (rule, index) {
            (1, 0) => [Some(0), Some(1), None],
            (2, 0) => [Some(0), None, None],
            (3, 0) => [Some(0), None, None],
            (3, 1) => [Some(1), None, None],
            _ => return Err(super::Invalid::PlanMismatch),
        };
        if result.ports != expected_ports {
            return Err(super::Invalid::PlanMismatch);
        }
        if result.fresh {
            if policy != super::StoragePolicy::Allocate
                || result.slot
                    != u32::try_from(slots.len())
                        .map_err(|_| super::Invalid::CounterOverflow)?
                        .checked_add(
                            u32::try_from(index).map_err(|_| super::Invalid::CounterOverflow)?,
                        )
                        .ok_or(super::Invalid::CounterOverflow)?
            {
                return Err(super::Invalid::PlanMismatch);
            }
            fresh_count = fresh_count
                .checked_add(1)
                .ok_or(super::Invalid::CounterOverflow)?;
        } else {
            if policy != super::StoragePolicy::Reuse {
                return Err(super::Invalid::PlanMismatch);
            }
            let consumed_index = if result.slot == pair.first {
                0
            } else if result.slot == pair.second {
                1
            } else {
                return Err(super::Invalid::PlanMismatch);
            };
            if consumed_reused[consumed_index] {
                return Err(super::Invalid::DuplicateFree);
            }
            consumed_reused[consumed_index] = true;
            let expected_slot = if rule == 3 && index == 1 {
                value_slot
            } else {
                rule_slot
            };
            if result.slot != expected_slot {
                return Err(super::Invalid::PlanMismatch);
            }
        }
    }
    for (index, reused) in consumed_reused.iter().enumerate() {
        let expected_action = u8::from(*reused);
        if image.action[index] != expected_action {
            return Err(super::Invalid::PlanMismatch);
        }
    }
    let mut expected_writes = 0usize;
    for result in image.results.iter().take(expected_results) {
        let result = result.ok_or(super::Invalid::PlanEncoding)?;
        expected_writes = expected_writes
            .checked_add(result.ports.iter().filter(|port| port.is_some()).count())
            .ok_or(super::Invalid::CounterOverflow)?;
    }
    if usize::from(image.write_count) != expected_writes
        || usize::from(image.external_write_count) != expected_writes
    {
        return Err(super::Invalid::PlanMismatch);
    }
    let mut write_index = 0usize;
    for (result_index, result) in image.results.iter().take(expected_results).enumerate() {
        let result = result.ok_or(super::Invalid::PlanEncoding)?;
        for (port_index, boundary_index) in result.ports.iter().enumerate() {
            let Some(boundary_index) = boundary_index else {
                continue;
            };
            if usize::from(*boundary_index) >= usize::from(image.boundary_count) {
                return Err(super::Invalid::PlanMismatch);
            }
            let expected_write = WireWrite {
                result: u8::try_from(result_index).map_err(|_| super::Invalid::CounterOverflow)?,
                port_index: super::PortIndex::from(
                    u8::try_from(port_index).map_err(|_| super::Invalid::CounterOverflow)?,
                ),
                boundary: *boundary_index,
            };
            if image.writes.get(write_index).copied().flatten() != Some(expected_write)
                || image.external_writes.get(write_index).copied().flatten() != Some(expected_write)
            {
                return Err(super::Invalid::PlanMismatch);
            }
            write_index = write_index
                .checked_add(1)
                .ok_or(super::Invalid::CounterOverflow)?;
        }
    }
    let expected_published = expected_publications(image, slots, pair)?;
    if usize::from(image.publication_count) != expected_published.0
        || image.publications[..expected_published.0]
            != expected_published.1[..expected_published.0]
    {
        return Err(super::Invalid::PlanMismatch);
    }
    let expected_arena = fresh_count;
    let expected_queue =
        u32::try_from(expected_published.0).map_err(|_| super::Invalid::CounterOverflow)?;
    if image.arena_reservation != expected_arena
        || image.scratch_reservation as usize != MAX_PLAN_BYTES
        || image.queue_reservation != expected_queue
    {
        return Err(super::Invalid::PlanMismatch);
    }
    let reused_slots = u32::try_from(consumed_reused.iter().filter(|reused| **reused).count())
        .map_err(|_| super::Invalid::CounterOverflow)?;
    let freed_slots = 2u32
        .checked_sub(reused_slots)
        .ok_or(super::Invalid::CounterOverflow)?;
    let live_decrease = 2u32
        .checked_sub(u32::try_from(expected_results).map_err(|_| super::Invalid::CounterOverflow)?)
        .ok_or(super::Invalid::CounterOverflow)?;
    Ok(CheckedPlan {
        image: *image,
        arena_reservation: expected_arena,
        scratch_reservation: image.scratch_reservation,
        queue_reservation: expected_queue,
        reused_slots,
        freed_slots,
        live_decrease,
    })
}

fn validate_boundary_independent(
    slots: &[super::Slot],
    outputs: &[super::PortRef],
    pair: super::Pair,
    boundary: Boundary,
) -> Result<(), super::Invalid> {
    if boundary.source.agent != pair.first && boundary.source.agent != pair.second {
        return Err(super::Invalid::PlanMismatch);
    }
    let source = slot_agent(slots, boundary.source.agent)?;
    if source
        .ports
        .get(usize::from(boundary.source.port_index))
        .copied()
        .flatten()
        != Some(boundary.external)
    {
        return Err(super::Invalid::PlanMismatch);
    }
    match boundary.external {
        super::Endpoint::Port(target) => {
            if target.agent == pair.first || target.agent == pair.second {
                return Err(super::Invalid::PlanBoundaryAlias);
            }
            let opposite = slot_agent(slots, target.agent)?;
            if usize::from(target.port_index) >= opposite.symbol.port_count()
                || opposite.ports[usize::from(target.port_index)]
                    != Some(super::Endpoint::Port(boundary.source))
            {
                return Err(super::Invalid::NonReciprocal);
            }
        }
        super::Endpoint::Output(index) => {
            if outputs.get(index as usize) != Some(&boundary.source) {
                return Err(super::Invalid::OutputBoundary);
            }
        }
    }
    Ok(())
}

fn expected_publications(
    image: &Image,
    slots: &[super::Slot],
    pair: super::Pair,
) -> Result<(usize, [Option<Active>; MAX_RESULTS]), super::Invalid> {
    let mut result: [Option<Active>; MAX_RESULTS] = [None; MAX_RESULTS];
    let mut count = 0usize;
    for agent in image
        .results
        .iter()
        .take(usize::from(image.result_count))
        .flatten()
    {
        let Some(boundary_index) = agent.ports[0] else {
            continue;
        };
        let boundary =
            image.boundaries[usize::from(boundary_index)].ok_or(super::Invalid::PlanEncoding)?;
        if let super::Endpoint::Port(target) = boundary.external {
            if target.port_index == super::PortIndex::from(0)
                && target.agent != pair.first
                && target.agent != pair.second
            {
                let target_slot = slots
                    .get(target.agent as usize)
                    .ok_or(super::Invalid::StalePlanSlot)?;
                let generation = if agent.fresh {
                    0
                } else {
                    slots
                        .get(agent.slot as usize)
                        .ok_or(super::Invalid::StalePlanSlot)?
                        .generation
                        .checked_add(1)
                        .ok_or(super::Invalid::CounterOverflow)?
                };
                let active = if agent.slot < target.agent {
                    Active {
                        first: agent.slot,
                        first_generation: generation,
                        second: target.agent,
                        second_generation: target_slot.generation,
                    }
                } else {
                    Active {
                        first: target.agent,
                        first_generation: target_slot.generation,
                        second: agent.slot,
                        second_generation: generation,
                    }
                };
                if result
                    .iter()
                    .flatten()
                    .any(|prior| prior.first == active.first && prior.second == active.second)
                {
                    return Err(super::Invalid::DoubleEnqueue);
                }
                result[count] = Some(active);
                count = count
                    .checked_add(1)
                    .ok_or(super::Invalid::CounterOverflow)?;
            }
        }
    }
    Ok((count, result))
}

fn validate_worklist(
    slots: &[super::Slot],
    worklist: &super::ActiveWorklist,
    selected: super::Pair,
) -> Result<(), super::Invalid> {
    if worklist.entries.len() > slots.len() {
        return Err(super::Invalid::WorklistBound);
    }
    for (index, queued) in worklist.entries.iter().enumerate() {
        if queued.first >= queued.second {
            return Err(super::Invalid::PlanEncoding);
        }
        if queued.second as usize >= slots.len() {
            return Err(super::Invalid::StalePlanSlot);
        }
        if *queued == selected {
            return Err(super::Invalid::DoubleEnqueue);
        }
        if worklist
            .entries
            .iter()
            .take(index)
            .any(|prior| prior == queued)
        {
            return Err(super::Invalid::DoubleEnqueue);
        }
        active_pair(slots, *queued)?;
        let first = slots
            .get(queued.first as usize)
            .ok_or(super::Invalid::StalePlanSlot)?;
        let second = slots
            .get(queued.second as usize)
            .ok_or(super::Invalid::StalePlanSlot)?;
        if first.queued_generation != Some(queued.first_generation)
            || second.queued_generation != Some(queued.second_generation)
        {
            return Err(super::Invalid::WorklistBound);
        }
        for prior in worklist.entries.iter().take(index) {
            let is_first_shared = prior.first == queued.first || prior.second == queued.first;
            let is_second_shared = prior.first == queued.second || prior.second == queued.second;
            if is_first_shared || is_second_shared {
                return Err(super::Invalid::DoubleEnqueue);
            }
        }
    }
    for (index, slot) in slots.iter().enumerate() {
        if let Some(generation) = slot.queued_generation {
            if generation != slot.generation {
                return Err(super::Invalid::WorklistBound);
            }
            let owner = u32::try_from(index).map_err(|_| super::Invalid::CounterOverflow)?;
            let references = worklist
                .entries
                .iter()
                .filter(|queued| {
                    (queued.first == owner && queued.first_generation == generation)
                        || (queued.second == owner && queued.second_generation == generation)
                })
                .count();
            if references != 1 {
                return Err(super::Invalid::WorklistBound);
            }
        }
    }
    for (index, slot) in slots.iter().enumerate() {
        let Some(agent) = slot.agent else { continue };
        let Some(super::Endpoint::Port(target)) = agent.ports[0] else {
            continue;
        };
        if target.port_index != super::PortIndex::from(0) || index >= target.agent as usize {
            continue;
        }
        let source = u32::try_from(index).map_err(|_| super::Invalid::CounterOverflow)?;
        let Some(other) = slots.get(target.agent as usize) else {
            return Err(super::Invalid::StalePlanSlot);
        };
        if other.agent.is_none_or(|other_agent| {
            other_agent.ports[0]
                != Some(super::Endpoint::Port(super::PortRef {
                    agent: source,
                    port_index: super::PortIndex::from(0),
                }))
        }) {
            continue;
        }
        let pair = super::Pair {
            first: source,
            first_generation: slot.generation,
            second: target.agent,
            second_generation: other.generation,
        };
        let count = worklist
            .entries
            .iter()
            .filter(|queued| **queued == pair)
            .count();
        if pair == selected {
            if count != 0 {
                return Err(super::Invalid::DoubleEnqueue);
            }
        } else if count != 1 {
            return Err(if count > 1 {
                super::Invalid::DoubleEnqueue
            } else {
                super::Invalid::WorklistBound
            });
        }
    }
    Ok(())
}

/// Independently decode and check emitted bytes against the current arena.
#[cfg(test)]
pub(super) fn check(
    bytes: &PlanBytes,
    slots: &[super::Slot],
    outputs: &[super::PortRef],
    pair: super::Pair,
    policy: super::StoragePolicy,
) -> Result<CheckedPlan, super::Invalid> {
    let empty = super::ActiveWorklist {
        entries: alloc::vec::Vec::new(),
    };
    check_with_worklist(CheckRequest {
        bytes,
        slots,
        outputs,
        worklist: &empty,
        pair,
        policy,
    })
}

/// Independently decode and check emitted bytes against the arena and queue.
pub(super) fn check_with_worklist(
    request: CheckRequest<'_>,
) -> Result<CheckedPlan, super::Invalid> {
    validate_worklist(request.slots, request.worklist, request.pair)?;
    let image = decode(request.bytes.as_bytes())?;
    validate_image(
        &image,
        request.slots,
        request.outputs,
        request.pair,
        request.policy,
    )
}

/// Reserve and execute a checked plan as one unobservable transaction.
pub(super) fn execute(
    checked: CheckedPlan,
    state: &mut ExecutionState<'_>,
    limits: ExecutionLimits,
) -> Result<Execution, ExecuteError> {
    execute_with_hooks(
        checked,
        state,
        limits,
        ExecutionHooks {
            interrupt_after: None,
            reservation_failure_after: None,
        },
    )
}

#[cfg(test)]
pub(super) fn execute_with_interrupt(
    checked: CheckedPlan,
    state: &mut ExecutionState<'_>,
    limits: ExecutionLimits,
    interrupt_after: Option<&dyn Fn(u8) -> bool>,
) -> Result<Execution, ExecuteError> {
    execute_with_hooks(
        checked,
        state,
        limits,
        ExecutionHooks {
            interrupt_after,
            reservation_failure_after: None,
        },
    )
}

#[cfg(test)]
pub(super) fn execute_with_reservation_failure(
    checked: CheckedPlan,
    state: &mut ExecutionState<'_>,
    limits: ExecutionLimits,
    fail_after_reservation: u8,
) -> Result<Execution, ExecuteError> {
    execute_with_hooks(
        checked,
        state,
        limits,
        ExecutionHooks {
            interrupt_after: None,
            reservation_failure_after: Some(fail_after_reservation),
        },
    )
}

fn execute_with_hooks(
    checked: CheckedPlan,
    state: &mut ExecutionState<'_>,
    limits: ExecutionLimits,
    hooks: ExecutionHooks<'_>,
) -> Result<Execution, ExecuteError> {
    let slots = &mut *state.slots;
    let outputs = &mut *state.outputs;
    let worklist = &mut *state.worklist;
    if u64::from(checked.scratch_reservation) > limits.scratch_bytes {
        return Err(ExecuteError::Limited {
            limit: super::Limit::PlanScratch,
            cap: limits.scratch_bytes,
            observed: u64::from(checked.scratch_reservation),
        });
    }
    let required_slots = u64::try_from(slots.len())
        .map_err(|_| ExecuteError::Invalid(super::Invalid::CounterOverflow))?
        .checked_add(u64::from(checked.arena_reservation))
        .ok_or(ExecuteError::Invalid(super::Invalid::CounterOverflow))?;
    if required_slots > limits.arena_slots {
        return Err(ExecuteError::Limited {
            limit: super::Limit::ArenaReservation,
            cap: limits.arena_slots,
            observed: required_slots,
        });
    }
    let required_queue = u64::try_from(worklist.entries.len())
        .map_err(|_| ExecuteError::Invalid(super::Invalid::CounterOverflow))?
        .checked_add(u64::from(checked.queue_reservation))
        .ok_or(ExecuteError::Invalid(super::Invalid::CounterOverflow))?;
    if required_queue > limits.worklist_entries {
        return Err(ExecuteError::Limited {
            limit: super::Limit::WorklistReservation,
            cap: limits.worklist_entries,
            observed: required_queue,
        });
    }
    let reserved_slots_before = slots.capacity();
    let reserved_entries_before = worklist.entries.capacity();
    if hooks.reservation_failure_after == Some(0) {
        return Err(ExecuteError::Invalid(
            super::Invalid::PlanReservationFailure,
        ));
    }
    let image = checked.image;
    let mut journal = RollbackJournal {
        old_slot_len: slots.len(),
        old_queue_len: worklist.entries.len(),
        slot_backups: [None; 4],
        slot_backup_count: 0,
        output_backups: [None; MAX_BOUNDARIES],
        output_backup_count: 0,
    };
    let live_before = u32::try_from(slots.iter().filter(|slot| slot.agent.is_some()).count())
        .map_err(|_| ExecuteError::Invalid(super::Invalid::CounterOverflow))?;
    let transient_live_nodes = live_before
        .checked_add(checked.arena_reservation)
        .ok_or(ExecuteError::Invalid(super::Invalid::CounterOverflow))?;
    for id in [image.pair.first, image.pair.second] {
        save_slot(slots, id as usize, &mut journal)?;
    }
    for boundary in image
        .boundaries
        .iter()
        .take(usize::from(image.boundary_count))
        .flatten()
    {
        match boundary.external {
            super::Endpoint::Port(target) => save_slot(slots, target.agent as usize, &mut journal)?,
            super::Endpoint::Output(output) => {
                let index = output as usize;
                if !journal
                    .output_backups
                    .iter()
                    .take(journal.output_backup_count)
                    .any(|saved| saved.is_some_and(|(saved_index, _)| saved_index == index))
                {
                    let old = *outputs
                        .get(index)
                        .ok_or(ExecuteError::Invalid(super::Invalid::OutputBoundary))?;
                    journal.output_backups[journal.output_backup_count] = Some((index, old));
                    journal.output_backup_count = journal
                        .output_backup_count
                        .checked_add(1)
                        .ok_or(ExecuteError::Invalid(super::Invalid::CounterOverflow))?;
                }
            }
        }
    }
    if slots
        .try_reserve(
            usize::try_from(checked.arena_reservation)
                .map_err(|_| ExecuteError::Invalid(super::Invalid::CounterOverflow))?,
        )
        .is_err()
    {
        return Err(ExecuteError::Invalid(
            super::Invalid::PlanReservationFailure,
        ));
    }
    if hooks.reservation_failure_after == Some(1) {
        slots.shrink_to(reserved_slots_before);
        return Err(ExecuteError::Invalid(
            super::Invalid::PlanReservationFailure,
        ));
    }
    if worklist
        .entries
        .try_reserve(
            usize::try_from(checked.queue_reservation)
                .map_err(|_| ExecuteError::Invalid(super::Invalid::CounterOverflow))?,
        )
        .is_err()
    {
        slots.shrink_to(reserved_slots_before);
        worklist.entries.shrink_to(reserved_entries_before);
        return Err(ExecuteError::Invalid(
            super::Invalid::PlanReservationFailure,
        ));
    }
    if hooks.reservation_failure_after == Some(2) {
        slots.shrink_to(reserved_slots_before);
        worklist.entries.shrink_to(reserved_entries_before);
        return Err(ExecuteError::Invalid(
            super::Invalid::PlanReservationFailure,
        ));
    }
    if let Err(invalid) = apply_image(state, &image, hooks.interrupt_after) {
        rollback(state, &journal);
        state.slots.shrink_to(reserved_slots_before);
        state.worklist.entries.shrink_to(reserved_entries_before);
        return Err(ExecuteError::Invalid(invalid));
    }
    Ok(Execution {
        live_decrease: checked.live_decrease,
        transient_live_nodes,
        reused_slots: checked.reused_slots,
        freed_slots: checked.freed_slots,
        allocated_slots: checked.arena_reservation,
        microsteps: 6,
    })
}

fn save_slot(
    slots: &[super::Slot],
    index: usize,
    journal: &mut RollbackJournal,
) -> Result<(), ExecuteError> {
    if journal
        .slot_backups
        .iter()
        .take(journal.slot_backup_count)
        .any(|saved| saved.is_some_and(|(saved_index, _)| saved_index == index))
    {
        return Ok(());
    }
    let slot = *slots
        .get(index)
        .ok_or(ExecuteError::Invalid(super::Invalid::StalePlanSlot))?;
    *journal
        .slot_backups
        .get_mut(journal.slot_backup_count)
        .ok_or(ExecuteError::Invalid(super::Invalid::PlanBoundaryAlias))? = Some((index, slot));
    journal.slot_backup_count = journal
        .slot_backup_count
        .checked_add(1)
        .ok_or(ExecuteError::Invalid(super::Invalid::CounterOverflow))?;
    Ok(())
}

fn rollback(state: &mut ExecutionState<'_>, journal: &RollbackJournal) {
    let slots = &mut *state.slots;
    let outputs = &mut *state.outputs;
    let worklist = &mut *state.worklist;
    for (index, slot) in journal
        .slot_backups
        .iter()
        .take(journal.slot_backup_count)
        .flatten()
    {
        if let Some(destination) = slots.get_mut(*index) {
            *destination = *slot;
        }
    }
    for (index, output) in journal
        .output_backups
        .iter()
        .take(journal.output_backup_count)
        .flatten()
    {
        if let Some(destination) = outputs.get_mut(*index) {
            *destination = *output;
        }
    }
    slots.truncate(journal.old_slot_len);
    worklist.entries.truncate(journal.old_queue_len);
}

fn apply_image(
    state: &mut ExecutionState<'_>,
    image: &Image,
    interrupt_after: Option<&dyn Fn(u8) -> bool>,
) -> Result<(), super::Invalid> {
    let slots = &mut *state.slots;
    let outputs = &mut *state.outputs;
    let worklist = &mut *state.worklist;
    let mut changed = [None; 2];
    for (index, result) in image
        .results
        .iter()
        .take(usize::from(image.result_count))
        .enumerate()
    {
        let result = result.ok_or(super::Invalid::PlanEncoding)?;
        let donor = slot_agent(slots, result.donor_slot)?;
        let replacement = super::RuntimeAgent {
            origin: donor.origin,
            symbol: result.symbol,
            ports: [None; 3],
        };
        if result.fresh {
            if result.slot as usize != slots.len() {
                return Err(super::Invalid::PlanMismatch);
            }
            slots.push(super::Slot {
                agent: Some(replacement),
                generation: 0,
                queued_generation: None,
            });
        } else {
            let slot = slots
                .get_mut(result.slot as usize)
                .ok_or(super::Invalid::StalePlanSlot)?;
            slot.generation = slot
                .generation
                .checked_add(1)
                .ok_or(super::Invalid::CounterOverflow)?;
            slot.queued_generation = None;
            slot.agent = Some(replacement);
        }
        changed[index] = Some(result.slot);
    }
    interrupt(interrupt_after, 1)?;
    for result in image
        .results
        .iter()
        .take(usize::from(image.result_count))
        .flatten()
    {
        let agent = slots
            .get_mut(result.slot as usize)
            .and_then(|slot| slot.agent.as_mut())
            .ok_or(super::Invalid::StalePlanSlot)?;
        for (port_index, boundary_index) in result.ports.iter().enumerate() {
            if let Some(boundary_index) = boundary_index {
                agent.ports[port_index] = Some(
                    image.boundaries[usize::from(*boundary_index)]
                        .ok_or(super::Invalid::PlanEncoding)?
                        .external,
                );
            }
        }
    }
    interrupt(interrupt_after, 2)?;
    for write in image
        .external_writes
        .iter()
        .take(usize::from(image.external_write_count))
        .flatten()
    {
        let result =
            image.results[usize::from(write.result)].ok_or(super::Invalid::PlanEncoding)?;
        let target =
            image.boundaries[usize::from(write.boundary)].ok_or(super::Invalid::PlanEncoding)?;
        let new_port = super::PortRef {
            agent: result.slot,
            port_index: write.port_index,
        };
        match target.external {
            super::Endpoint::Port(peer) => {
                let slot = slots
                    .get_mut(peer.agent as usize)
                    .ok_or(super::Invalid::StalePlanSlot)?;
                if peer.port_index == super::PortIndex::from(0) {
                    slot.queued_generation = None;
                }
                let agent = slot.agent.as_mut().ok_or(super::Invalid::StalePlanSlot)?;
                let endpoint = agent
                    .ports
                    .get_mut(usize::from(peer.port_index))
                    .ok_or(super::Invalid::PortArity)?;
                if *endpoint != Some(super::Endpoint::Port(target.source)) {
                    return Err(super::Invalid::NonReciprocal);
                }
                *endpoint = Some(super::Endpoint::Port(new_port));
            }
            super::Endpoint::Output(output) => {
                let entry = outputs
                    .get_mut(output as usize)
                    .ok_or(super::Invalid::OutputBoundary)?;
                if *entry != target.source {
                    return Err(super::Invalid::OutputBoundary);
                }
                *entry = new_port;
            }
        }
    }
    interrupt(interrupt_after, 3)?;
    for id in [image.pair.first, image.pair.second] {
        let is_retained = image
            .results
            .iter()
            .take(usize::from(image.result_count))
            .flatten()
            .any(|result| !result.fresh && result.slot == id);
        if !is_retained {
            let slot = slots
                .get_mut(id as usize)
                .ok_or(super::Invalid::StalePlanSlot)?;
            slot.generation = slot
                .generation
                .checked_add(1)
                .ok_or(super::Invalid::CounterOverflow)?;
            slot.queued_generation = None;
            slot.agent = None;
        }
    }
    interrupt(interrupt_after, 4)?;
    let start_queue_len = worklist.entries.len();
    for changed_slot in changed.into_iter().flatten() {
        super::enqueue_active(slots, worklist, changed_slot as usize)?;
    }
    let published = worklist
        .entries
        .len()
        .checked_sub(start_queue_len)
        .ok_or(super::Invalid::CounterOverflow)?;
    if published != usize::from(image.publication_count) {
        return Err(super::Invalid::DoubleEnqueue);
    }
    for (actual, expected) in worklist.entries.iter().skip(start_queue_len).zip(
        image
            .publications
            .iter()
            .take(usize::from(image.publication_count)),
    ) {
        let expected = expected.ok_or(super::Invalid::PlanEncoding)?;
        if *actual
            != (super::Pair {
                first: expected.first,
                first_generation: expected.first_generation,
                second: expected.second,
                second_generation: expected.second_generation,
            })
        {
            return Err(super::Invalid::DoubleEnqueue);
        }
    }
    interrupt(interrupt_after, 5)?;
    interrupt(interrupt_after, 6)?;
    Ok(())
}

fn interrupt(hook: Option<&dyn Fn(u8) -> bool>, step: u8) -> Result<(), super::Invalid> {
    if hook.is_some_and(|should_interrupt| should_interrupt(step)) {
        Err(super::Invalid::PlanInterrupted)
    } else {
        Ok(())
    }
}

fn apply(op: super::Op, first: u64, second: u64) -> u64 {
    match op {
        super::Op::Add => first.wrapping_add(second),
        super::Op::Sub => first.wrapping_sub(second),
        super::Op::Mul => first.wrapping_mul(second),
        super::Op::Eq => u64::from(first == second),
    }
}
