use super::command::worker;

type CliError = molten::error::MoltenError;
type FsPath = std::path::Path;
type IoValue = preserves::IOValue;
type JobWorkerExecution = molten::job_dag::JobWorkerExecution;

struct RequestInput<'a> {
    admission_value: &'a IoValue,
    execution_request_value: &'a IoValue,
    sync_ref: Option<&'a str>,
    target_peer: &'a str,
    stages: &'a [String],
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    peer_bootstrap_refs: Vec<String>,
    node_identity_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

pub(super) struct RunInput<'a> {
    pub(super) request_value: &'a IoValue,
    pub(super) target_registry: &'a FsPath,
    pub(super) storage_root: &'a FsPath,
    pub(super) cache_root: &'a FsPath,
    pub(super) chunk_root: &'a FsPath,
    pub(super) admission_value: &'a IoValue,
    pub(super) execution_request_value: &'a IoValue,
    pub(super) transport_root: &'a FsPath,
    pub(super) from_peer: &'a str,
    pub(super) from_actor: &'a str,
    pub(super) topic: &'a str,
    pub(super) ledger_root: Option<&'a FsPath>,
    pub(super) out: &'a FsPath,
}

pub(crate) fn request(args: worker::Request) -> Result<(), CliError> {
    let admission_value = super::io::read_preserves_file(&args.admission_receipt)?;
    let execution_request_value = super::io::read_preserves_file(&args.execution_request)?;
    let request_value = request_value(RequestInput {
        admission_value: &admission_value,
        execution_request_value: &execution_request_value,
        sync_ref: args.sync_ref.as_deref(),
        target_peer: &args.target_peer,
        stages: &args.stages,
        authority_refs: args.authority_refs,
        resource_refs: args.resource_refs,
        peer_bootstrap_refs: args.peer_bootstrap_refs,
        node_identity_refs: args.node_identity_refs,
        evidence_refs: args.evidence_refs,
    })?;
    let parsed = molten::job_dag::parse_job_worker_request_value(&request_value)?;
    super::io::emit_job_analysis(&request_value, args.out.as_ref())?;
    eprintln!(
        "job worker-request ok job={} request={} target={} stages={}",
        parsed.job_ref,
        parsed.request_ref,
        parsed.target_peer,
        parsed.stage_ids.len()
    );
    Ok(())
}

pub(crate) fn run_local(args: worker::RunLocal) -> Result<(), CliError> {
    let request_value = super::io::read_preserves_file(&args.request)?;
    let admission_value = super::io::read_preserves_file(&args.admission_receipt)?;
    let execution_request_value = super::io::read_preserves_file(&args.execution_request)?;
    let chunk_root = args.chunks.unwrap_or_else(|| args.target_registry.join("job-chunks"));
    let executed = run_local_execution(RunInput {
        request_value: &request_value,
        target_registry: &args.target_registry,
        storage_root: &args.storage,
        cache_root: &args.cache,
        chunk_root: &chunk_root,
        admission_value: &admission_value,
        execution_request_value: &execution_request_value,
        transport_root: &args.transport_root,
        from_peer: &args.from_peer,
        from_actor: &args.from_actor,
        topic: &args.topic,
        ledger_root: args.ledger.as_deref(),
        out: &args.out,
    })?;
    eprintln!(
        "job worker-run-local {} job={} receipt={} result={} out={}",
        executed.result.decision,
        executed.result.job_ref,
        executed.receipt_ref,
        executed.result.result_ref,
        args.out.display()
    );
    if executed.result.decision == "pass" {
        Ok(())
    } else {
        Err(CliError::invalid_harness(format!(
            "job worker-run-local denied: {}",
            executed.result.diagnostics.join("; ")
        )))
    }
}

pub(crate) fn schedule_local(args: worker::ScheduleLocal) -> Result<(), CliError> {
    super::schedule::local(args)
}

fn request_value(input: RequestInput<'_>) -> Result<IoValue, CliError> {
    let admission = molten::job_dag::parse_job_admission_receipt_value(input.admission_value)?;
    let execution_request = molten::job_dag::parse_job_execution_request_value(input.execution_request_value)?;
    let admission_ref = molten::preserves_rail::canonical_hash(input.admission_value)?;
    let execution_request_ref = molten::preserves_rail::canonical_hash(input.execution_request_value)?;
    if execution_request.admission_ref != admission_ref {
        return Err(CliError::invalid_harness("job worker execution request does not bind admission receipt"));
    }
    if execution_request.job_ref != admission.job_ref {
        return Err(CliError::invalid_harness("job worker execution request job ref mismatches admission"));
    }
    let sync_ref = input.sync_ref.map(str::to_string).unwrap_or_else(|| admission.sync_ref.clone());
    let stage_ids = if input.stages.is_empty() {
        execution_request.stage_ids.clone()
    } else {
        input.stages.to_vec()
    };
    let authority_refs = select_refs(input.authority_refs.clone(), admission.authority_receipt_refs.clone());
    let resource_refs = select_refs(input.resource_refs.clone(), execution_request.resource_refs.clone());
    let evidence_refs = evidence_refs(&input, &sync_ref, &admission_ref, &execution_request_ref)?;
    molten::job_dag::job_worker_request_value(molten::job_dag::JobWorkerRequestValueInput {
        job_ref: &admission.job_ref,
        target_peer: input.target_peer,
        stage_ids: &stage_ids,
        sync_ref: &sync_ref,
        admission_ref: &admission_ref,
        execution_request_ref: &execution_request_ref,
        authority_refs: &authority_refs,
        resource_refs: &resource_refs,
        peer_bootstrap_refs: &input.peer_bootstrap_refs,
        node_identity_refs: &input.node_identity_refs,
        evidence_refs: &evidence_refs,
    })
}

fn select_refs(input: Vec<String>, fallback: Vec<String>) -> Vec<String> {
    if input.is_empty() { fallback } else { input }
}

fn evidence_refs(
    input: &RequestInput<'_>,
    sync_ref: &str,
    admission_ref: &str,
    execution_request_ref: &str,
) -> Result<Vec<String>, CliError> {
    let mut refs = super::core::Items::new(super::JOB_WORKER_CLI_REF_LIMIT, "job worker evidence refs");
    for reference in &input.evidence_refs {
        refs.push_unique(reference.clone())?;
    }
    for reference in [sync_ref, admission_ref, execution_request_ref] {
        refs.push_unique(reference.to_string())?;
    }
    for reference in &input.peer_bootstrap_refs {
        refs.push_unique(reference.clone())?;
    }
    for reference in &input.node_identity_refs {
        refs.push_unique(reference.clone())?;
    }
    Ok(refs.into_vec())
}

pub(super) fn run_local_execution(input: RunInput<'_>) -> Result<JobWorkerExecution, CliError> {
    let request = molten::job_dag::parse_job_worker_request_value(input.request_value)?;
    let envelope = molten::job_dag::job_worker_envelope(molten::job_dag::JobWorkerEnvelopeInput {
        from_peer: input.from_peer,
        from_actor: input.from_actor,
        to_peer: &request.target_peer,
        topic: input.topic,
        request_value: input.request_value,
    })?;
    let published = molten::remote_dataspace::publish_local_gossip(input.transport_root, &envelope, input.from_peer)?;
    let delivery = molten::remote_dataspace::deliver_local_gossip(
        input.transport_root,
        input.topic,
        &envelope.envelope_ref,
        &request.target_peer,
    )?;
    let delivery_log = molten::remote_dataspace::delivery_log(std::slice::from_ref(&delivery), true)?;
    let executed = molten::job_dag::execute_worker_delivery(molten::job_dag::JobWorkerExecuteInput {
        target_registry: input.target_registry,
        storage_root: input.storage_root,
        cache_root: input.cache_root,
        chunk_root: input.chunk_root,
        delivery: &delivery,
        delivery_log: Some(&delivery_log),
        admission_receipt_value: input.admission_value,
        execution_request_value: input.execution_request_value,
        ledger_root: input.ledger_root,
    })?;
    write_execution(WriteInput {
        run: input,
        envelope: &envelope,
        published: &published,
        delivery: &delivery,
        delivery_log: &delivery_log,
        executed: &executed,
    })?;
    Ok(executed)
}

struct WriteInput<'a> {
    run: RunInput<'a>,
    envelope: &'a molten::remote_dataspace::RemoteDataspaceEnvelope,
    published: &'a molten::remote_dataspace::RemoteDataspaceExchange,
    delivery: &'a molten::remote_dataspace::RemoteDataspaceDelivery,
    delivery_log: &'a molten::remote_dataspace::RemoteDeliveryLog,
    executed: &'a molten::job_dag::JobWorkerExecution,
}

fn write_execution(input: WriteInput<'_>) -> Result<(), CliError> {
    std::fs::create_dir_all(input.run.out).map_err(CliError::from)?;
    super::io::write_file(
        &input.run.out.join("request.preserves"),
        &molten::preserves_rail::to_text(input.run.request_value)?,
    )?;
    super::io::write_file(
        &input.run.out.join("envelope.preserves"),
        &molten::preserves_rail::to_text(&input.envelope.value)?,
    )?;
    super::io::write_file(
        &input.run.out.join("publish-receipt.preserves"),
        &molten::preserves_rail::to_text(&input.published.receipt_value)?,
    )?;
    super::io::write_file(
        &input.run.out.join("delivery-receipt.preserves"),
        &molten::preserves_rail::to_text(&input.delivery.receipt_value)?,
    )?;
    super::io::write_file(
        &input.run.out.join("delivery-log.preserves"),
        &molten::preserves_rail::to_text(&input.delivery_log.value)?,
    )?;
    super::io::write_file(
        &input.run.out.join("assignment.preserves"),
        &molten::preserves_rail::to_text(&input.executed.assignment_value)?,
    )?;
    super::io::write_indexed_values(input.run.out, "status", &input.executed.status_values)?;
    super::io::write_file(
        &input.run.out.join("result.preserves"),
        &molten::preserves_rail::to_text(&input.executed.result.value)?,
    )?;
    super::io::write_file(
        &input.run.out.join("worker-receipt.preserves"),
        &molten::preserves_rail::to_text(&input.executed.receipt_value)?,
    )?;
    if let Some(execution) = input.executed.execution.as_ref() {
        super::io::write_file(
            &input.run.out.join("execution-receipt.preserves"),
            &molten::preserves_rail::to_text(&execution.receipt_value)?,
        )?;
        if let Some(run) = execution.run.as_ref() {
            super::io::write_file(
                &input.run.out.join("output.preserves"),
                &molten::preserves_rail::to_text(&run.output_value)?,
            )?;
        }
    }
    Ok(())
}
