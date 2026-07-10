use super::*;

struct State<'a> {
    input: LocalInput<'a>,
    request: Request,
    request_ref: String,
    lease_key: String,
    refs: CoordinationRefs,
    manifest_ref: String,
    runtime: Runtime,
    evidence_values: Items<Value>,
    receipt_refs: Items<String>,
    assertion_refs: Items<String>,
}

struct EnqueueResults {
    enqueue: Apply,
    enqueue_duplicate: Apply,
}

struct Outcome {
    dequeue: Option<Apply>,
    lease: Option<Apply>,
    release: Option<Apply>,
    worker: Option<Worker>,
    diagnostics: Vec<String>,
}

impl Outcome {
    fn new(diagnostics: Vec<String>) -> Self {
        Self {
            dequeue: None,
            lease: None,
            release: None,
            worker: None,
            diagnostics,
        }
    }
}

pub(super) fn execute(input: LocalInput<'_>) -> Result<LocalResult> {
    let mut state = prepare(input)?;
    let enqueue = enqueue_pair(&mut state)?;
    let diagnostics = enqueue_diagnostics(&enqueue.enqueue, &enqueue.enqueue_duplicate);
    let steps = advance(&mut state, diagnostics)?;
    finish(state, enqueue, steps)
}

fn prepare<'a>(input: LocalInput<'a>) -> Result<State<'a>> {
    let request = molten::job_dag::parse_job_worker_request_value(input.request_value)?;
    let request_ref = request.request_ref.clone();
    let lease_key = input.lease_key.map(str::to_string).unwrap_or_else(|| format!("lock:job-worker:{request_ref}"));
    let refs = coordination_refs(&input, &request, &request_ref)?;
    let manifest_value = molten::coordination::coordination_fixture_manifest_value()?;
    let runtime = molten::coordination::new_coordination_runtime(&manifest_value)?;
    let manifest_ref = runtime.manifest.manifest_ref.clone();
    let mut evidence_values =
        Items::new(super::super::COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "job worker schedule evidence");
    let receipt_refs = Items::new(super::super::COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule receipts");
    let assertion_refs = Items::new(super::super::COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule assertions");
    evidence_values.push(manifest_value)?;
    Ok(State {
        input,
        request,
        request_ref,
        lease_key,
        refs,
        manifest_ref,
        runtime,
        evidence_values,
        receipt_refs,
        assertion_refs,
    })
}

fn enqueue_pair(state: &mut State<'_>) -> Result<EnqueueResults> {
    let enqueue_request = coordination_request(RequestInput {
        service: molten::coordination::SERVICE_QUEUE,
        operation: molten::coordination::OP_ENQUEUE,
        key: state.input.queue_key,
        client_session: state.input.scheduler_session,
        operation_label: "worker-schedule-enqueue",
        request_ref: &state.request_ref,
        payload: Some(molten::preserves_rail::record("item", vec![molten::preserves_rail::string(&state.request_ref)])),
        refs: &state.refs,
    })?;
    let enqueue = molten::coordination::apply_coordination_request(&mut state.runtime, &enqueue_request)?;
    push_result(&enqueue, &mut state.evidence_values, &mut state.receipt_refs, &mut state.assertion_refs)?;
    let enqueue_duplicate = molten::coordination::apply_coordination_request(&mut state.runtime, &enqueue_request)?;
    push_result(&enqueue_duplicate, &mut state.evidence_values, &mut state.receipt_refs, &mut state.assertion_refs)?;
    Ok(EnqueueResults {
        enqueue,
        enqueue_duplicate,
    })
}

fn advance(state: &mut State<'_>, diagnostics: Vec<String>) -> Result<Outcome> {
    let mut steps = Outcome::new(diagnostics);
    steps.dequeue = maybe_dequeue(state, &mut steps)?;
    steps.lease = maybe_lease(state, &mut steps)?;
    maybe_execute(state, &mut steps)?;
    Ok(steps)
}

fn maybe_dequeue(state: &mut State<'_>, steps: &mut Outcome) -> Result<Option<Apply>> {
    if !steps.diagnostics.is_empty() {
        return Ok(None);
    }
    phase::dequeue_once(phase::ApplyInput {
        runtime: &mut state.runtime,
        input: &state.input,
        refs: &state.refs,
        request_ref: &state.request_ref,
        evidence_values: &mut state.evidence_values,
        receipt_refs: &mut state.receipt_refs,
        assertion_refs: &mut state.assertion_refs,
        diagnostics: &mut steps.diagnostics,
    })
}

fn maybe_lease(state: &mut State<'_>, steps: &mut Outcome) -> Result<Option<Apply>> {
    if !steps.diagnostics.is_empty() {
        return Ok(None);
    }
    phase::lease_once(phase::LeaseInput {
        apply: phase::ApplyInput {
            runtime: &mut state.runtime,
            input: &state.input,
            refs: &state.refs,
            request_ref: &state.request_ref,
            evidence_values: &mut state.evidence_values,
            receipt_refs: &mut state.receipt_refs,
            assertion_refs: &mut state.assertion_refs,
            diagnostics: &mut steps.diagnostics,
        },
        lease_key: &state.lease_key,
    })
}

fn maybe_execute(state: &mut State<'_>, steps: &mut Outcome) -> Result<()> {
    if !steps.diagnostics.is_empty() {
        return Ok(());
    }
    phase::run_or_release(phase::WorkerInput {
        runtime: &mut state.runtime,
        input: &state.input,
        refs: &state.refs,
        request_ref: &state.request_ref,
        lease_key: &state.lease_key,
        lease: steps.lease.as_ref(),
        evidence_values: &mut state.evidence_values,
        receipt_refs: &mut state.receipt_refs,
        assertion_refs: &mut state.assertion_refs,
        diagnostics: &mut steps.diagnostics,
        release: &mut steps.release,
        worker: &mut steps.worker,
    })
}

fn finish(state: State<'_>, enqueue: EnqueueResults, steps: Outcome) -> Result<LocalResult> {
    let State {
        input,
        request,
        lease_key,
        manifest_ref,
        runtime,
        evidence_values,
        receipt_refs,
        assertion_refs,
        ..
    } = state;
    let EnqueueResults {
        enqueue,
        enqueue_duplicate,
    } = enqueue;
    let Outcome {
        dequeue,
        lease,
        release,
        worker,
        diagnostics,
    } = steps;
    output::finalize(FinalizeInput {
        input,
        request: &request,
        manifest_ref: &manifest_ref,
        runtime: &runtime,
        evidence_values,
        receipt_refs,
        assertion_refs,
        enqueue: Some(&enqueue),
        enqueue_duplicate: Some(&enqueue_duplicate),
        dequeue: dequeue.as_ref(),
        lease: lease.as_ref(),
        release: release.as_ref(),
        worker: worker.as_ref(),
        diagnostics,
        lease_key: &lease_key,
    })
}

fn enqueue_diagnostics(enqueue: &Apply, duplicate: &Apply) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if enqueue.receipt.decision != "pass" {
        diagnostics.extend(enqueue.receipt.diagnostics.clone());
    } else if !duplicate_enqueue_replayed(enqueue, duplicate) {
        diagnostics.push("coordination duplicate enqueue did not replay prior receipt".to_string());
    }
    diagnostics
}
