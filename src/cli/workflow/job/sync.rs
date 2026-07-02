type AdmitLoopback = super::command::sync::AdmitLoopback;
type AdmitPlan = super::command::sync::AdmitPlan;
type ExecuteLoopback = super::command::sync::ExecuteLoopback;
type Loopback = super::command::sync::Loopback;
type MoltenError = molten::error::MoltenError;
type Plan = super::command::sync::Plan;
type Result<T> = molten::error::Result<T>;

#[path = "sync/input.rs"]
mod input;

pub(crate) fn plan(args: Plan) -> Result<()> {
    let request = input::request(&args.source_registry, &args.job, &args.stages, &args.target_peer, &[])?;
    let plan = molten::job_dag::sync_plan_value(&args.source_registry, &args.target_registry, &request)?;
    super::io::emit_job_analysis(&plan.value, args.out.as_ref())?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "job sync receipt", &plan.receipt_value)?;
    eprintln!(
        "job sync-plan ok job={} plan={} missing={}",
        plan.request.job_ref,
        plan.plan_ref,
        plan.missing_refs.len()
    );
    Ok(())
}

pub(crate) fn loopback(args: Loopback) -> Result<()> {
    let provenance_values = super::io::read_preserves_files(&args.provenance_paths)?;
    let build_verification_values = super::io::read_preserves_files(&args.build_verification_paths)?;
    let mut evidence_refs = super::io::values_canonical_refs(&provenance_values)?;
    evidence_refs.extend(super::io::values_canonical_refs(&build_verification_values)?);
    let request = input::request(&args.source_registry, &args.job, &args.stages, &args.target_peer, &evidence_refs)?;
    let synced = molten::job_dag::sync_loopback(molten::job_dag::SyncLoopbackInput {
        source_registry: &args.source_registry,
        target_registry: &args.target_registry,
        request_value: &request,
        provenance_values: &provenance_values,
        build_verification_values: &build_verification_values,
    })?;
    super::io::emit_job_analysis(&synced.plan.value, args.plan_out.as_ref())?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "job sync receipt", &synced.receipt_value)?;
    eprintln!(
        "job sync-loopback decision={} job={} installed={} already_present={}",
        synced.decision,
        synced.plan.request.job_ref,
        synced.installed_refs.len(),
        synced.already_present_refs.len()
    );
    Ok(())
}

pub(crate) fn admit_plan(args: AdmitPlan) -> Result<()> {
    let request = input::admission(input::AdmissionInput {
        target_registry: &args.target_registry,
        job: &args.job,
        sync_ref: args.sync_ref.as_deref(),
        stages: &args.stages,
        target_peer: &args.target_peer,
        policy_refs: args.policy_refs,
        capability_refs: args.capability_refs,
        evidence_refs: args.evidence_refs,
        resource_refs: args.resource_refs,
    })?;
    let plan = molten::job_dag::admission_plan_value(&args.target_registry, &request)?;
    super::io::emit_job_analysis(&plan.value, args.out.as_ref())?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "job admission receipt", &plan.receipt_value)?;
    eprintln!(
        "job admit-plan {} job={} plan={} stages={}",
        plan.decision,
        plan.request.job_ref,
        plan.plan_ref,
        plan.stage_order.len()
    );
    Ok(())
}

pub(crate) fn admit_loopback(args: AdmitLoopback) -> Result<()> {
    let request = input::admission(input::AdmissionInput {
        target_registry: &args.target_registry,
        job: &args.job,
        sync_ref: args.sync_ref.as_deref(),
        stages: &args.stages,
        target_peer: &args.target_peer,
        policy_refs: args.policy_refs,
        capability_refs: args.capability_refs,
        evidence_refs: args.evidence_refs,
        resource_refs: args.resource_refs,
    })?;
    let admitted = molten::job_dag::admission_loopback(&args.target_registry, &request)?;
    super::io::emit_job_analysis(&admitted.plan.value, args.plan_out.as_ref())?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "job admission receipt", &admitted.receipt_value)?;
    eprintln!(
        "job admit-loopback {} job={} receipt={} stages={}",
        admitted.plan.decision,
        admitted.plan.request.job_ref,
        admitted.receipt_ref,
        admitted.plan.stage_order.len()
    );
    Ok(())
}

pub(crate) fn execute_loopback(args: ExecuteLoopback) -> Result<()> {
    let admission_value = match super::io::read_preserves_file(&args.admission_receipt) {
        Ok(value) => value,
        Err(error) => return missing_admission(args, error),
    };
    let request = input::execution(input::ExecutionInput {
        target_registry: &args.target_registry,
        job: &args.job,
        admission_value: &admission_value,
        target_peer: &args.target_peer,
        stages: &args.stages,
        policy_refs: args.policy_refs,
        capability_refs: args.capability_refs,
        resource_refs: args.resource_refs,
    })?;
    emit_request(args.request_out.as_ref(), &request)?;
    let chunk_root = args.chunks.unwrap_or_else(|| args.target_registry.join("job-chunks"));
    let executed = molten::job_dag::execution_loopback(molten::job_dag::ExecutionLoopbackInput {
        target_registry: &args.target_registry,
        storage_root: &args.storage,
        cache_root: &args.cache,
        chunk_root: &chunk_root,
        admission_receipt_value: &admission_value,
        request_value: &request,
    })?;
    emit_optional_run(args.out.as_ref(), executed.run.as_ref())?;
    finish_execution(args.receipt_out.as_ref(), executed)
}

fn missing_admission(args: ExecuteLoopback, error: MoltenError) -> Result<()> {
    let request = input::from_admission_ref(input::ExecutionFromAdmissionInput {
        target_registry: &args.target_registry,
        job: &args.job,
        admission_ref: None,
        target_peer: &args.target_peer,
        stages: &args.stages,
        policy_refs: args.policy_refs,
        capability_refs: args.capability_refs,
        resource_refs: args.resource_refs,
    })?;
    emit_request(args.request_out.as_ref(), &request)?;
    let receipt = molten::job_dag::missing_admission_execution_receipt_value(&request, &error.to_string())?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "job execution receipt", &receipt)?;
    Err(error)
}

fn emit_request(path: Option<&std::path::PathBuf>, request: &preserves::IOValue) -> Result<()> {
    if let Some(path) = path {
        super::io::write_file(path, &molten::preserves_rail::to_text(request)?)?;
    }
    Ok(())
}

fn emit_optional_run(path: Option<&std::path::PathBuf>, run: Option<&molten::job_dag::JobRun>) -> Result<()> {
    if let Some(run) = run {
        super::io::write_optional_output(path, &molten::preserves_rail::to_text(&run.output_value)?)?;
    }
    Ok(())
}

fn finish_execution(
    receipt_out: Option<&std::path::PathBuf>,
    executed: molten::job_dag::JobExecutionLoopback,
) -> Result<()> {
    super::io::emit_named_receipt(receipt_out, "job execution receipt", &executed.receipt_value)?;
    if executed.decision == "pass" {
        eprintln!(
            "job execute-loopback pass job={} receipt={} outputs={}",
            executed.request.job_ref,
            executed.receipt_ref,
            executed.run.as_ref().map(|run| run.output_refs.len()).unwrap_or_default()
        );
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "job execute-loopback denied: {}",
            executed.diagnostics.join("; ")
        )))
    }
}
