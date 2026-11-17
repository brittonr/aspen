// SPDX-License-Identifier: AGPL-3.0-or-later

mod calculation;
mod meter;

use crate::*;

// r[impl molten.pure_net.producer]
// r[impl molten.pure_net.bounds]
// r[impl molten.pure_net.outcomes]
// r[impl molten.pure_net.replay]
/// Performs one synchronous calculation. Hook calls may hand off a permit, but
/// retain the same real producer arena and counters; cancellation is terminal.
/// Every backend independently parses/checks/extracts the source. The reference
/// lane runs the producer normative evaluator, never the net arithmetic path.
/// Returns the semantic report and its canonical frame so callers need not
/// repeat serialization outside the calculation's blocking-work owner.
pub fn execute(
    input: ExecutionInput<'_>,
    hook: &mut dyn FnMut(&Observation) -> Control,
) -> Result<(Receipt, Vec<u8>), Error> {
    if (hook)(&Observation {
        phase: Phase::Decode,
        work: 0,
        interactions: 0,
    }) == Control::Cancel
    {
        return Err(Error::Cancelled);
    }
    let Admission {
        manifest,
        program,
        inputs,
        mut receipt,
        decode_work,
        decode_memory,
    } = admit(&input)?;
    let mut meter = meter::State {
        limits: &manifest.limits,
        fuel: input.grants.fuel,
        hook,
        counters: Counters::default(),
        trace: Vec::with_capacity(
            usize::try_from(manifest.limits.max_observations).map_err(|_| Error::Exhausted(Resource::Observations))?,
        ),
        last_phase: None,
        last_work: 0,
        stopped: None,
    };
    meter.counters.logical_memory_reservation = decode_memory;
    let result = meter.charge(Phase::Decode, decode_work, 0).and_then(|()| {
        calculation::calculate(
            calculation::Input {
                manifest: &manifest,
                program: &program,
                input: &inputs.values,
                grants: input.grants,
            },
            &mut meter,
            &mut receipt,
        )
    });
    receipt.outcome = match result {
        Ok(values) => Outcome::Completed(values),
        Err(error) => Outcome::Rejected(error),
    };
    receipt.counters = meter.counters;
    receipt.observations = meter.trace;
    // A failed semantic report may still be retained separately by the lifecycle;
    // it is never a successful output admitted beyond the request output grant.
    let mut encoded = encode_receipt(&receipt)?;
    if matches!(receipt.outcome, Outcome::Completed(_)) && encoded.len() as u64 > input.grants.max_output_bytes {
        receipt.outcome = Outcome::Rejected(Error::Exhausted(Resource::Readback));
        encoded = encode_receipt(&receipt)?;
    }
    Ok((receipt, encoded))
}

struct Admission {
    manifest: Manifest,
    program: MaterializedProgram,
    inputs: OrderedInputs,
    receipt: Receipt,
    decode_work: u64,
    decode_memory: u64,
}

fn admit(input: &ExecutionInput<'_>) -> Result<Admission, Error> {
    let frame_bytes = (input.manifest_bytes.len() as u64)
        .saturating_add(input.program_bytes.len() as u64)
        .saturating_add(input.input_bytes.len() as u64);
    let decode_work = frame_bytes.saturating_mul(64);
    let decode_memory = decode_work.saturating_add(65_536);
    if decode_memory > input.grants.memory_bytes {
        return Err(Error::Exhausted(Resource::Memory));
    }
    if decode_work > input.grants.fuel {
        return Err(Error::Exhausted(Resource::TotalWork));
    }
    let manifest = decode_manifest(input.manifest_bytes)?;
    if &manifest.bindings.host != input.host {
        return Err(Error::Bindings);
    }
    if decode_memory > manifest.limits.max_memory_bytes {
        return Err(Error::Exhausted(Resource::Memory));
    }
    if decode_work > manifest.limits.max_atomic_work {
        return Err(Error::Exhausted(Resource::Decode));
    }
    if input.grants.max_attempts == 0
        || input.grants.max_attempts > manifest.limits.max_attempts
        || manifest
            .limits
            .max_total_work
            .checked_mul(u64::from(input.grants.max_attempts))
            .is_none_or(|v| v > manifest.limits.max_aggregate_attempt_work)
    {
        return Err(Error::Exhausted(Resource::Attempts));
    }
    let program: MaterializedProgram = decode(PROGRAM, input.program_bytes, manifest.limits.max_program_bytes)?;
    let inputs: OrderedInputs = decode(INPUTS, input.input_bytes, manifest.limits.max_input_bytes)?;
    let input_count = u32::try_from(inputs.values.len()).map_err(|_| Error::Exhausted(Resource::Inputs))?;
    if input_count > manifest.limits.max_inputs {
        return Err(Error::Exhausted(Resource::Inputs));
    }
    if input_count != program.program.input_arity {
        return Err(Error::Malformed);
    }
    let receipt = initial_receipt(input, &manifest, &program, &inputs)?;
    Ok(Admission {
        manifest,
        program,
        inputs,
        receipt,
        decode_work,
        decode_memory,
    })
}

fn initial_receipt(
    input: &ExecutionInput<'_>,
    manifest: &Manifest,
    program: &MaterializedProgram,
    inputs: &OrderedInputs,
) -> Result<Receipt, Error> {
    Ok(Receipt {
        version: "pure-net-eval/receipt/v1".into(),
        backend: manifest.backend,
        manifest: artifact_id(molten_batch_core::ContentKind::PureNetManifest, input.manifest_bytes)?,
        program: artifact_id(molten_batch_core::ContentKind::PureNetProgram, input.program_bytes)?,
        ordered_inputs: artifact_id(molten_batch_core::ContentKind::Data, input.input_bytes)?,
        manifest_semantics: identity(MANIFEST, manifest)?,
        program_semantics: identity(PROGRAM, program)?,
        ordered_input_semantics: identity(INPUTS, inputs)?,
        bindings: manifest.bindings.clone(),
        limits: manifest.limits.clone(),
        grants: input.grants.clone(),
        checked_ir: None,
        compiled_net: None,
        producer_graph_digest: None,
        producer_plan_stream_digest: None,
        outcome: Outcome::Rejected(Error::Malformed),
        counters: Counters::default(),
        observations: Vec::new(),
    })
}
