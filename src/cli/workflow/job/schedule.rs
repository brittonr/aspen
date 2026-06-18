use std::path::Path;

use molten::error::MoltenError;
use molten::error::Result;

use super::command::worker;
use super::io;
#[path = "schedule/output.rs"]
mod output;
#[path = "schedule/phase.rs"]
mod phase;

struct LocalInput<'a> {
    request_value: &'a preserves::IOValue,
    target_registry: &'a Path,
    storage_root: &'a Path,
    cache_root: &'a Path,
    chunk_root: &'a Path,
    admission_value: &'a preserves::IOValue,
    execution_request_value: &'a preserves::IOValue,
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
    receipt_value: preserves::IOValue,
    worker: Option<molten::job_dag::JobWorkerExecution>,
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
    payload: Option<preserves::IOValue>,
    refs: &'a CoordinationRefs,
}

struct FinalizeInput<'a> {
    input: LocalInput<'a>,
    request: &'a molten::job_dag::JobWorkerRequest,
    manifest_ref: &'a str,
    runtime: &'a molten::coordination::CoordinationRuntime,
    evidence_values: super::core::Items<preserves::IOValue>,
    receipt_refs: super::core::Items<String>,
    assertion_refs: super::core::Items<String>,
    enqueue: Option<&'a molten::coordination::CoordinationApplyResult>,
    enqueue_duplicate: Option<&'a molten::coordination::CoordinationApplyResult>,
    dequeue: Option<&'a molten::coordination::CoordinationApplyResult>,
    lease: Option<&'a molten::coordination::CoordinationApplyResult>,
    release: Option<&'a molten::coordination::CoordinationApplyResult>,
    worker: Option<&'a molten::job_dag::JobWorkerExecution>,
    diagnostics: Vec<String>,
    lease_key: &'a str,
}

pub(crate) fn local(args: worker::ScheduleLocal) -> Result<()> {
    let request_value = io::read_preserves_file(&args.request)?;
    let admission_value = io::read_preserves_file(&args.admission_receipt)?;
    let execution_request_value = io::read_preserves_file(&args.execution_request)?;
    let chunk_root = args.chunks.unwrap_or_else(|| args.target_registry.join("job-chunks"));
    let result = execute(LocalInput {
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

fn execute(input: LocalInput<'_>) -> Result<LocalResult> {
    let request = molten::job_dag::parse_job_worker_request_value(input.request_value)?;
    let request_ref = request.request_ref.clone();
    let lease_key = input
        .lease_key
        .map(str::to_string)
        .unwrap_or_else(|| format!("lock:job-worker:{}", request.request_ref));
    let refs = coordination_refs(&input, &request, &request_ref)?;
    let manifest_value = molten::coordination::coordination_fixture_manifest_value()?;
    let mut runtime = molten::coordination::new_coordination_runtime(&manifest_value)?;
    let manifest_ref = runtime.manifest.manifest_ref.clone();
    let mut evidence_values =
        super::core::Items::new(super::COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "job worker schedule evidence");
    let mut receipt_refs =
        super::core::Items::new(super::COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule receipts");
    let mut assertion_refs =
        super::core::Items::new(super::COORDINATION_CLI_BATCH_REF_LIMIT, "job worker schedule assertions");
    evidence_values.push(manifest_value.clone())?;
    let enqueue_request = coordination_request(RequestInput {
        service: molten::coordination::SERVICE_QUEUE,
        operation: molten::coordination::OP_ENQUEUE,
        key: input.queue_key,
        client_session: input.scheduler_session,
        operation_label: "worker-schedule-enqueue",
        request_ref: &request_ref,
        payload: Some(molten::preserves_rail::record("item", vec![molten::preserves_rail::string(&request_ref)])),
        refs: &refs,
    })?;
    let enqueue = molten::coordination::apply_coordination_request(&mut runtime, &enqueue_request)?;
    push_result(&enqueue, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
    let enqueue_duplicate = molten::coordination::apply_coordination_request(&mut runtime, &enqueue_request)?;
    push_result(&enqueue_duplicate, &mut evidence_values, &mut receipt_refs, &mut assertion_refs)?;
    let mut diagnostics = enqueue_diagnostics(&enqueue, &enqueue_duplicate);
    let mut dequeue = None;
    let mut lease = None;
    let mut release = None;
    let mut worker = None;
    if diagnostics.is_empty() {
        dequeue = phase::dequeue_once(phase::ApplyInput {
            runtime: &mut runtime,
            input: &input,
            refs: &refs,
            request_ref: &request_ref,
            evidence_values: &mut evidence_values,
            receipt_refs: &mut receipt_refs,
            assertion_refs: &mut assertion_refs,
            diagnostics: &mut diagnostics,
        })?;
    }
    if diagnostics.is_empty() {
        lease = phase::lease_once(phase::LeaseInput {
            apply: phase::ApplyInput {
                runtime: &mut runtime,
                input: &input,
                refs: &refs,
                request_ref: &request_ref,
                evidence_values: &mut evidence_values,
                receipt_refs: &mut receipt_refs,
                assertion_refs: &mut assertion_refs,
                diagnostics: &mut diagnostics,
            },
            lease_key: &lease_key,
        })?;
    }
    if diagnostics.is_empty() {
        phase::run_or_release(phase::RunOrReleaseInput {
            runtime: &mut runtime,
            input: &input,
            refs: &refs,
            request_ref: &request_ref,
            lease_key: &lease_key,
            lease: lease.as_ref(),
            evidence_values: &mut evidence_values,
            receipt_refs: &mut receipt_refs,
            assertion_refs: &mut assertion_refs,
            diagnostics: &mut diagnostics,
            release: &mut release,
            worker: &mut worker,
        })?;
    }
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

fn coordination_refs(
    input: &LocalInput<'_>,
    request: &molten::job_dag::JobWorkerRequest,
    request_ref: &str,
) -> Result<CoordinationRefs> {
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

fn coordination_request(input: RequestInput<'_>) -> Result<preserves::IOValue> {
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
    result: &molten::coordination::CoordinationApplyResult,
    evidence_values: &mut super::core::Items<preserves::IOValue>,
    receipt_refs: &mut super::core::Items<String>,
    assertion_refs: &mut super::core::Items<String>,
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

fn enqueue_diagnostics(
    enqueue: &molten::coordination::CoordinationApplyResult,
    duplicate: &molten::coordination::CoordinationApplyResult,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if enqueue.receipt.decision != "pass" {
        diagnostics.extend(enqueue.receipt.diagnostics.clone());
    } else if duplicate.receipt.receipt_ref != enqueue.receipt.receipt_ref {
        diagnostics.push("coordination duplicate enqueue did not replay prior receipt".to_string());
    }
    diagnostics
}
