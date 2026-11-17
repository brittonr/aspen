// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::*;

pub(super) struct Input<'a> {
    pub(super) manifest: &'a Manifest,
    pub(super) program: &'a MaterializedProgram,
    pub(super) input: &'a [i64],
    pub(super) grants: &'a ExecutionGrants,
}

pub(super) fn calculate(
    input: Input<'_>,
    meter: &mut super::meter::State<'_>,
    receipt: &mut Receipt,
) -> Result<Vec<Value>, Error> {
    let limits = &input.manifest.limits;
    meter.charge(
        Phase::Decode,
        (input.program.program.source.len() as u64).saturating_add(input.input.len() as u64),
        0,
    )?;
    if input.program.producer != PRODUCER_REVISION {
        return Err(Error::Bindings);
    }
    if u32::try_from(input.program.program.source.len()).map_err(|_| Error::Exhausted(Resource::Source))?
        > limits.max_source_bytes
    {
        return Err(Error::Exhausted(Resource::Source));
    }
    reserve_memory(limits, input.grants, meter)?;
    let extraction =
        crate::producer::checked(&input.program.program, limits, &mut |phase, units| meter.charge(phase, units, 0))?;
    meter.counters.extraction_work = extraction.stats.work_units;
    meter.counters.expanded_nodes = extraction.stats.expanded_nodes;
    let checked = &extraction.checked;
    let ir = noble_ngraph::format::graph::print(checked.graph());
    if ir.len() as u64 > limits.max_ir_bytes {
        return Err(Error::Exhausted(Resource::Ir));
    }
    let ir_id = identity("pure-net-eval/checked-source-ir/v1", &ir)?;
    if input.program.checked_ir != ir || input.program.ir != ir_id {
        return Err(Error::Bindings);
    }
    receipt.checked_ir = Some(ir_id);
    receipt.producer_graph_digest = Some(molten_batch_core::Id(checked.id()));
    let values = noble_ngraph::eval::i64_inputs(input.input);
    let output = match input.manifest.backend {
        Backend::IndependentReference => run_reference(checked, &values, &input, meter)?,
        Backend::SequentialNet => run_sequential(checked, &values, &input, meter, receipt)?,
    };
    if output.len() != checked.graph().outputs.len() {
        return Err(Error::Incomplete);
    }
    // Both lanes bound ordered-output conversion and canonical readback work.
    meter.charge(Phase::Readback, output.len() as u64, meter.counters.interactions)?;
    meter.charge(Phase::Complete, 0, meter.counters.interactions)?;
    Ok(output)
}

fn reserve_memory(limits: &Limits, grants: &ExecutionGrants, meter: &mut super::meter::State<'_>) -> Result<(), Error> {
    // Conservative live logical allocation reservation includes simultaneously
    // retained frontend submission, checked graph, candidate + validation rebuild,
    // canonical-net projection, fixed runtime arena/queue, readback and trace.
    let memory = u64::from(limits.max_source_nodes)
        .saturating_mul(1024)
        .saturating_add(limits.max_ir_nodes.saturating_mul(8192))
        .saturating_add(limits.max_agent_slots.saturating_mul(2048))
        .saturating_add(u64::from(limits.max_source_bytes).saturating_mul(256))
        .saturating_add(u64::from(limits.max_observations).saturating_mul(256))
        .saturating_add(limits.max_program_bytes.saturating_mul(8))
        .saturating_add(meter.counters.logical_memory_reservation);
    meter.counters.logical_memory_reservation = memory;
    if memory > limits.max_memory_bytes.min(grants.memory_bytes) {
        return Err(Error::Exhausted(Resource::Memory));
    }
    Ok(())
}

fn run_reference(
    checked: &noble_ngraph::check::CheckedGraph,
    values: &[noble_ngraph::sem::Value],
    input: &Input<'_>,
    meter: &mut super::meter::State<'_>,
) -> Result<Vec<Value>, Error> {
    let limits = &input.manifest.limits;
    let memory = limits.max_memory_bytes.min(input.grants.memory_bytes);
    meter.charge(Phase::Reference, 0, 0)?;
    // Scratch is charged independently; every normative node is hooked.
    if noble_ngraph::eval::reference_scratch_bytes(checked) > memory {
        return Err(Error::Exhausted(Resource::Memory));
    }
    let cell = std::cell::RefCell::new(&mut *meter);
    let report = noble_ngraph::eval::reference_with_hook(
        checked,
        values,
        None,
        &mut noble_ngraph::eval::Control {
            fuel: Some(limits.reference_fuel.min(input.grants.fuel)),
            cancel: None,
            scratch: None,
            forward_scratch: Some(memory),
            evaluated_nodes: None,
        },
        &|_| cell.borrow_mut().charge(Phase::Reference, 1, 0).is_err(),
        &|outputs| cell.borrow_mut().charge(Phase::Readback, outputs, 0).is_err(),
    );
    if let Some(error) = cell.borrow().stopped.clone() {
        return Err(error);
    }
    match report.obs {
        noble_ngraph::eval::Obs::Normal(values) => crate::producer::values(values),
        noble_ngraph::eval::Obs::Exhausted(_) => Err(Error::Exhausted(Resource::Reference)),
        noble_ngraph::eval::Obs::Cancelled => Err(Error::Cancelled),
        noble_ngraph::eval::Obs::Refused(d) => Err(crate::producer::diagnostic(&d)),
        noble_ngraph::eval::Obs::GuardFailed { .. } => Err(Error::Unsupported),
    }
}

fn run_sequential(
    checked: &noble_ngraph::check::CheckedGraph,
    values: &[noble_ngraph::sem::Value],
    input: &Input<'_>,
    meter: &mut super::meter::State<'_>,
    receipt: &mut Receipt,
) -> Result<Vec<Value>, Error> {
    let limits = &input.manifest.limits;
    let net_limits = noble_ngraph::net::Limits {
        max_agent_slots: limits.max_agent_slots,
        max_wires: limits.max_wires,
        interaction_fuel: limits.interaction_fuel.min(input.grants.fuel),
    };
    let candidate = lower_candidate(checked, values, &net_limits, meter)?;
    meter.charge(Phase::Lower, (candidate.agents.len() as u64).saturating_mul(16), 0)?;
    receipt.compiled_net = Some(crate::producer::net_identity(&candidate)?);
    meter.counters.initial_agents = candidate.agents.len() as u64;
    let cell = std::cell::RefCell::new(&mut *meter);
    let report = noble_ngraph::net::reduce_with_hook(
        &crate::producer::Ids,
        noble_ngraph::net::ReduceRequest {
            checked,
            candidate: &candidate,
            inputs: values,
            limits: &net_limits,
            plan: noble_ngraph::net::Plan {
                schedule: Some(noble_ngraph::net::Schedule::Lifo),
                interaction_fuel: None,
                cancel: None,
                storage_policy: None,
                max_plan_scratch_bytes: Some(limits.max_plan_scratch_bytes),
                max_worklist_entries: Some(limits.max_queue_entries),
                plan_observer: None,
            },
        },
        &|phase, work, interactions| cell.borrow_mut().charge(net_phase(phase), work, interactions).is_err(),
    );
    record_net_report(&report, &cell, receipt);
    if let Some(error) = cell.borrow().stopped.clone() {
        return Err(error);
    }
    match report.obs {
        noble_ngraph::net::Obs::Normal(values) => crate::producer::values(values),
        noble_ngraph::net::Obs::Refused(refusal) => Err(crate::producer::refusal(refusal)),
        noble_ngraph::net::Obs::Exhausted { limit, .. } => Err(crate::producer::exhausted(limit)),
        noble_ngraph::net::Obs::Cancelled => Err(Error::Cancelled),
        noble_ngraph::net::Obs::InternalInvalid(invalid) => Err(crate::producer::invalid_error(invalid)),
        noble_ngraph::net::Obs::GuardFailed { .. } => Err(Error::Unsupported),
    }
}

fn lower_candidate(
    checked: &noble_ngraph::check::CheckedGraph,
    values: &[noble_ngraph::sem::Value],
    limits: &noble_ngraph::net::Limits,
    meter: &mut super::meter::State<'_>,
) -> Result<noble_ngraph::net::InteractionGraph, Error> {
    let cell = std::cell::RefCell::new(&mut *meter);
    let candidate = noble_ngraph::net::lower_with_hook(&crate::producer::Ids, checked, values, limits, &|work| {
        cell.borrow_mut().charge(Phase::Lower, work, 0).is_err()
    });
    if let Some(error) = cell.borrow().stopped.clone() {
        return Err(error);
    }
    candidate.map_err(crate::producer::refusal)
}

fn net_phase(phase: noble_ngraph::net::WorkPhase) -> Phase {
    match phase {
        noble_ngraph::net::WorkPhase::Validation => Phase::Validate,
        noble_ngraph::net::WorkPhase::Setup => Phase::Setup,
        noble_ngraph::net::WorkPhase::Reduction => Phase::Reduce,
        noble_ngraph::net::WorkPhase::Readback => Phase::Readback,
    }
}

fn record_net_report(
    report: &noble_ngraph::net::Report,
    cell: &std::cell::RefCell<&mut super::meter::State<'_>>,
    receipt: &mut Receipt,
) {
    {
        let mut meter = cell.borrow_mut();
        meter.counters.interactions = report.interactions;
        meter.counters.peak_live_agents = report.peak_live_agents;
        meter.counters.arena_slots_high_water = report.plan_stats.arena_slots_high_water;
        meter.counters.queue_high_water = report.plan_stats.queue_high_water;
        meter.counters.allocated_slots = report.plan_stats.allocated_slots;
        meter.counters.reused_slots = report.plan_stats.reused_slots;
        meter.counters.freed_slots = report.plan_stats.freed_slots;
        meter.counters.plan_bytes = report.plan_stats.emitted_plan_bytes;
        meter.counters.plan_microsteps = report.plan_stats.microsteps;
        meter.counters.plan_scratch_bytes_high_water = report.plan_stats.scratch_bytes_high_water;
        meter.counters.transient_live_agents_high_water = report.plan_stats.transient_slots;
        meter.counters.initial_net_retained_bytes = report.retained_bytes;
    }
    receipt.producer_plan_stream_digest = Some(molten_batch_core::Id(report.plan_stats.plan_digest));
}
