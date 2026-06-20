use std::path::Path;

use molten::error::MoltenError;
use molten::error::Result;

use super::command::worker;
use super::io;
#[path = "schedule/output.rs"]
mod output;
#[path = "schedule/phase.rs"]
mod phase;
#[path = "schedule/run.rs"]
mod run;

type Apply = molten::coordination::CoordinationApplyResult;
type Items<T> = super::core::Items<T>;
type Request = molten::job_dag::JobWorkerRequest;
type Runtime = molten::coordination::CoordinationRuntime;
type Value = preserves::IOValue;
type Worker = molten::job_dag::JobWorkerExecution;

struct LocalInput<'a> {
    request_value: &'a Value,
    target_registry: &'a Path,
    storage_root: &'a Path,
    cache_root: &'a Path,
    chunk_root: &'a Path,
    admission_value: &'a Value,
    execution_request_value: &'a Value,
    transport_root: &'a Path,
    queue_key: &'a str,
    lease_key: Option<&'a str>,
    scheduler_session: &'a str,
    worker_session: &'a str,
    lease_token: Option<u64>,
    from_peer: &'a str,
    from_actor: &'a str,
    topic: &'a str,
    coordination_authority_refs: Vec<String>,
    coordination_resource_refs: Vec<String>,
    coordination_policy_refs: Vec<String>,
    ledger_root: Option<&'a Path>,
    out: &'a Path,
}

struct LocalResult {
    decision: String,
    receipt_ref: String,
    receipt_value: Value,
    worker: Option<Worker>,
}

struct CoordinationRefs {
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    policy_refs: Vec<String>,
}

struct RequestInput<'a> {
    service: &'a str,
    operation: &'a str,
    key: &'a str,
    client_session: &'a str,
    operation_label: &'a str,
    request_ref: &'a str,
    payload: Option<Value>,
    refs: &'a CoordinationRefs,
}

struct FinalizeInput<'a> {
    input: LocalInput<'a>,
    request: &'a Request,
    manifest_ref: &'a str,
    runtime: &'a Runtime,
    evidence_values: Items<Value>,
    receipt_refs: Items<String>,
    assertion_refs: Items<String>,
    enqueue: Option<&'a Apply>,
    enqueue_duplicate: Option<&'a Apply>,
    dequeue: Option<&'a Apply>,
    lease: Option<&'a Apply>,
    release: Option<&'a Apply>,
    worker: Option<&'a Worker>,
    diagnostics: Vec<String>,
    lease_key: &'a str,
}

pub(crate) fn local(args: worker::ScheduleLocal) -> Result<()> {
    let request_value = io::read_preserves_file(&args.request)?;
    let admission_value = io::read_preserves_file(&args.admission_receipt)?;
    let execution_request_value = io::read_preserves_file(&args.execution_request)?;
    let chunk_root = args.chunks.unwrap_or_else(|| args.target_registry.join("job-chunks"));
    let result = run::execute(LocalInput {
        request_value: &request_value,
        target_registry: &args.target_registry,
        storage_root: &args.storage,
        cache_root: &args.cache,
        chunk_root: &chunk_root,
        admission_value: &admission_value,
        execution_request_value: &execution_request_value,
        transport_root: &args.transport_root,
        queue_key: &args.queue_key,
        lease_key: args.lease_key.as_deref(),
        scheduler_session: &args.scheduler_session,
        worker_session: &args.worker_session,
        lease_token: args.lease_token,
        from_peer: &args.from_peer,
        from_actor: &args.from_actor,
        topic: &args.topic,
        coordination_authority_refs: args.coordination_authority_refs,
        coordination_resource_refs: args.coordination_resource_refs,
        coordination_policy_refs: args.coordination_policy_refs,
        ledger_root: args.ledger.as_deref(),
        out: &args.out,
    })?;
    eprintln!(
        "job worker-schedule-local {} receipt={} worker={} out={}",
        result.decision,
        result.receipt_ref,
        result.worker.as_ref().map(|worker| worker.receipt_ref.as_str()).unwrap_or("-"),
        args.out.display()
    );
    if result.decision == "pass" {
        Ok(())
    } else {
        let parsed = molten::job_dag::parse_job_worker_schedule_receipt_value(&result.receipt_value)?;
        Err(MoltenError::invalid_harness(format!(
            "job worker-schedule-local denied: {}",
            parsed.diagnostics.join("; ")
        )))
    }
}

fn coordination_refs(input: &LocalInput<'_>, request: &Request, request_ref: &str) -> Result<CoordinationRefs> {
    Ok(CoordinationRefs {
        authority_refs: select(input.coordination_authority_refs.clone(), request.authority_refs.clone()),
        resource_refs: select(input.coordination_resource_refs.clone(), request.resource_refs.clone()),
        policy_refs: if input.coordination_policy_refs.is_empty() {
            vec![io::synthetic_ref("worker-schedule-policy", request_ref)?]
        } else {
            input.coordination_policy_refs.clone()
        },
    })
}

fn select(input: Vec<String>, fallback: Vec<String>) -> Vec<String> {
    if input.is_empty() { fallback } else { input }
}

fn coordination_request(input: RequestInput<'_>) -> Result<Value> {
    molten::coordination::coordination_request_value(&molten::coordination::CoordinationRequestInput {
        service: input.service.to_string(),
        operation: input.operation.to_string(),
        key: input.key.to_string(),
        client_session: input.client_session.to_string(),
        operation_id_ref: io::synthetic_ref(input.operation_label, input.request_ref)?,
        payload: input.payload,
        authority_refs: input.refs.authority_refs.clone(),
        resource_refs: input.refs.resource_refs.clone(),
        policy_refs: input.refs.policy_refs.clone(),
    })
}

fn push_result(
    result: &Apply,
    evidence_values: &mut Items<Value>,
    receipt_refs: &mut Items<String>,
    assertion_refs: &mut Items<String>,
) -> Result<()> {
    receipt_refs.push(result.receipt.receipt_ref.clone())?;
    for assertion in &result.assertions {
        assertion_refs.push(assertion.assertion_ref.clone())?;
    }
    for value in &result.evidence_values {
        evidence_values.push(value.clone())?;
    }
    Ok(())
}
