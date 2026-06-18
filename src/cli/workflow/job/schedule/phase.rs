use molten::error::Result;

use super::*;

pub(super) struct ApplyInput<'a> {
    pub(super) runtime: &'a mut molten::coordination::CoordinationRuntime,
    pub(super) input: &'a LocalInput<'a>,
    pub(super) refs: &'a CoordinationRefs,
    pub(super) request_ref: &'a str,
    pub(super) evidence_values: &'a mut super::super::core::Items<preserves::IOValue>,
    pub(super) receipt_refs: &'a mut super::super::core::Items<String>,
    pub(super) assertion_refs: &'a mut super::super::core::Items<String>,
    pub(super) diagnostics: &'a mut Vec<String>,
}

pub(super) struct LeaseInput<'a> {
    pub(super) apply: ApplyInput<'a>,
    pub(super) lease_key: &'a str,
}

pub(super) fn dequeue_once(input: ApplyInput<'_>) -> Result<Option<molten::coordination::CoordinationApplyResult>> {
    let request = coordination_request(RequestInput {
        service: molten::coordination::SERVICE_QUEUE,
        operation: molten::coordination::OP_DEQUEUE,
        key: input.input.queue_key,
        client_session: input.input.worker_session,
        operation_label: "worker-schedule-dequeue",
        request_ref: input.request_ref,
        payload: None,
        refs: input.refs,
    })?;
    let result = molten::coordination::apply_coordination_request(input.runtime, &request)?;
    push_result(&result, input.evidence_values, input.receipt_refs, input.assertion_refs)?;
    if result.receipt.decision != "pass" {
        input.diagnostics.extend(result.receipt.diagnostics.clone());
    }
    Ok(Some(result))
}

pub(super) fn lease_once(input: LeaseInput<'_>) -> Result<Option<molten::coordination::CoordinationApplyResult>> {
    let request = coordination_request(RequestInput {
        service: molten::coordination::SERVICE_LOCK,
        operation: molten::coordination::OP_ACQUIRE,
        key: input.lease_key,
        client_session: input.apply.input.worker_session,
        operation_label: "worker-schedule-lease",
        request_ref: input.apply.request_ref,
        payload: None,
        refs: input.apply.refs,
    })?;
    let result = molten::coordination::apply_coordination_request(input.apply.runtime, &request)?;
    push_result(&result, input.apply.evidence_values, input.apply.receipt_refs, input.apply.assertion_refs)?;
    if result.receipt.decision != "pass" {
        input.apply.diagnostics.extend(result.receipt.diagnostics.clone());
    }
    Ok(Some(result))
}

pub(super) struct RunOrReleaseInput<'a> {
    pub(super) runtime: &'a mut molten::coordination::CoordinationRuntime,
    pub(super) input: &'a LocalInput<'a>,
    pub(super) refs: &'a CoordinationRefs,
    pub(super) request_ref: &'a str,
    pub(super) lease_key: &'a str,
    pub(super) lease: Option<&'a molten::coordination::CoordinationApplyResult>,
    pub(super) evidence_values: &'a mut super::super::core::Items<preserves::IOValue>,
    pub(super) receipt_refs: &'a mut super::super::core::Items<String>,
    pub(super) assertion_refs: &'a mut super::super::core::Items<String>,
    pub(super) diagnostics: &'a mut Vec<String>,
    pub(super) release: &'a mut Option<molten::coordination::CoordinationApplyResult>,
    pub(super) worker: &'a mut Option<molten::job_dag::JobWorkerExecution>,
}

pub(super) fn run_or_release(input: RunOrReleaseInput<'_>) -> Result<()> {
    let Some(token) = input.lease.and_then(|result| result.token.as_ref()) else {
        input.diagnostics.push("coordination lease did not emit fencing token".to_string());
        return Ok(());
    };
    let effective_token = input.input.lease_token.unwrap_or(token.token);
    if effective_token != token.token {
        let result = release_once(ReleaseInput {
            runtime: input.runtime,
            input: input.input,
            refs: input.refs,
            request_ref: input.request_ref,
            lease_key: input.lease_key,
            token: effective_token,
        })?;
        push_result(&result, input.evidence_values, input.receipt_refs, input.assertion_refs)?;
        input.diagnostics.extend(result.receipt.diagnostics.clone());
        if input.diagnostics.is_empty() {
            input
                .diagnostics
                .push(format!("stale fencing token {effective_token}; current token is {}", token.token));
        }
        *input.release = Some(result);
        return Ok(());
    }
    let executed = worker_execution(input.input)?;
    if executed.result.decision != "pass" {
        input.diagnostics.extend(executed.result.diagnostics.clone());
    }
    *input.worker = Some(executed);
    let result = release_once(ReleaseInput {
        runtime: input.runtime,
        input: input.input,
        refs: input.refs,
        request_ref: input.request_ref,
        lease_key: input.lease_key,
        token: effective_token,
    })?;
    push_result(&result, input.evidence_values, input.receipt_refs, input.assertion_refs)?;
    if result.receipt.decision != "pass" {
        input.diagnostics.extend(result.receipt.diagnostics.clone());
    }
    *input.release = Some(result);
    Ok(())
}

struct ReleaseInput<'a> {
    runtime: &'a mut molten::coordination::CoordinationRuntime,
    input: &'a LocalInput<'a>,
    refs: &'a CoordinationRefs,
    request_ref: &'a str,
    lease_key: &'a str,
    token: u64,
}

fn release_once(input: ReleaseInput<'_>) -> Result<molten::coordination::CoordinationApplyResult> {
    let request = coordination_request(RequestInput {
        service: molten::coordination::SERVICE_LOCK,
        operation: molten::coordination::OP_RELEASE,
        key: input.lease_key,
        client_session: input.input.worker_session,
        operation_label: "worker-schedule-release",
        request_ref: input.request_ref,
        payload: Some(molten::preserves_rail::record("token", vec![molten::preserves_rail::u64_value(input.token)])),
        refs: input.refs,
    })?;
    molten::coordination::apply_coordination_request(input.runtime, &request)
}

fn worker_execution(input: &LocalInput<'_>) -> Result<molten::job_dag::JobWorkerExecution> {
    let worker_out = input.out.join("worker");
    super::super::worker::run_local_execution(super::super::worker::RunInput {
        request_value: input.request_value,
        target_registry: input.target_registry,
        storage_root: input.storage_root,
        cache_root: input.cache_root,
        chunk_root: input.chunk_root,
        admission_value: input.admission_value,
        execution_request_value: input.execution_request_value,
        transport_root: input.transport_root,
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        topic: input.topic,
        ledger_root: input.ledger_root,
        out: &worker_out,
    })
}
