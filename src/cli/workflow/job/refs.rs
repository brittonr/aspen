use molten::error::Result;

use super::io;

pub(crate) fn submit(args: super::command::refs::Submit) -> Result<()> {
    let executable = io::content_arg(&args.executable, "executable")?;
    let inputs = args.inputs.iter().map(|input| io::content_arg(input, "input")).collect::<Result<Vec<_>>>()?;
    let value = molten::job_dag::job_ref_submission_value(molten::job_dag::BlobRefJobSubmissionValueInput {
        job_id: &args.job_id,
        operation_id: &args.operation_id,
        executable,
        inputs,
        output_mode: &args.output_mode,
        input_schema_refs: &args.input_schema_refs,
        output_schema_refs: &args.output_schema_refs,
        effect_manifest_refs: &args.effect_manifest_refs,
        handler_profile: &args.handler_profile,
        authority_context_ref: &args.authority_context_ref,
        policy_refs: &args.policy_refs,
        provenance_refs: &args.provenance_refs,
        evidence_refs: &args.evidence_refs,
    })?;
    let submission = molten::job_dag::parse_job_ref_submission_value(&value)?;
    io::emit_job_analysis(&value, args.out.as_ref())?;
    eprintln!(
        "job ref-submit ok job={} submission={} inputs={}",
        submission.job_id,
        submission.submission_ref,
        submission.inputs.len()
    );
    Ok(())
}

pub(crate) fn execute(args: super::command::refs::Execute) -> Result<()> {
    let submission_value = io::read_preserves_file(&args.submission)?;
    let executed = molten::job_dag::execute_blob_ref_job(molten::job_dag::BlobRefJobExecuteInput {
        chunk_root: &args.chunks,
        submission_value: &submission_value,
        ledger_root: args.ledger.as_deref(),
    })?;
    io::emit_named_receipt(args.receipt_out.as_ref(), "job ref receipt", &executed.receipt_value)?;
    eprintln!(
        "job ref-execute {} job={} receipt={} output={}",
        executed.decision,
        executed.submission.job_id,
        executed.receipt_ref,
        executed.output_manifest_ref.as_deref().unwrap_or("none")
    );
    if executed.decision == "pass" {
        Ok(())
    } else {
        Err(molten::error::MoltenError::invalid_harness(format!(
            "job ref-execute denied: {}",
            executed.diagnostics.join("; ")
        )))
    }
}

pub(crate) fn status(args: super::command::refs::Status) -> Result<()> {
    for entry in molten::ledger::list_artifacts(&args.ledger)? {
        let value = match entry.artifact_kind.as_str() {
            "job-dag-receipt" | "job-ref-receipt" | "job-worker-receipt" | "job-worker-schedule-receipt" => {
                molten::ledger::read_artifact(&args.ledger, &entry.artifact_ref)?
            }
            _ => continue,
        };
        if maybe_print_schedule(&entry.artifact_ref, &value, args.job.as_deref())? {
            continue;
        }
        if maybe_print_worker(&entry.artifact_ref, &value, args.job.as_deref())? {
            continue;
        }
        print_receipt(&entry.artifact_ref, &value, args.job.as_deref())?;
    }
    Ok(())
}

pub(crate) fn receipt_show(args: super::command::refs::ReceiptShow) -> Result<()> {
    let value = molten::ledger::read_artifact(&args.ledger, &args.receipt_ref)?;
    println!("{}", molten::job_dag::receipt_summary(&value)?);
    println!("{}", molten::preserves_rail::to_text(&value)?);
    Ok(())
}

fn maybe_print_schedule(artifact_ref: &str, value: &preserves::IOValue, job: Option<&str>) -> Result<bool> {
    let Ok(schedule) = molten::job_dag::parse_job_worker_schedule_receipt_value(value) else {
        return Ok(false);
    };
    if job.is_none_or(|job_ref| schedule.job_ref == job_ref) {
        println!(
            "{} worker-schedule {} {} {}",
            artifact_ref,
            schedule.decision,
            schedule.job_ref,
            schedule.result_ref.unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(true)
}

fn maybe_print_worker(artifact_ref: &str, value: &preserves::IOValue, job: Option<&str>) -> Result<bool> {
    let Ok(worker) = molten::job_dag::parse_job_worker_receipt_value(value) else {
        return Ok(false);
    };
    if job.is_none_or(|job_ref| worker.job_ref.as_deref() == Some(job_ref)) {
        println!(
            "{} worker-execute {} {} {}",
            artifact_ref,
            worker.decision,
            worker.job_ref.unwrap_or_else(|| "-".to_string()),
            worker.result_ref
        );
    }
    Ok(true)
}

fn print_receipt(artifact_ref: &str, value: &preserves::IOValue, job: Option<&str>) -> Result<()> {
    let receipt = molten::job_dag::parse_job_receipt(value)
        .or_else(|_| molten::job_dag::parse_blob_ref_job_receipt_value(value))?;
    if job.is_none_or(|job_ref| receipt.job_ref.as_deref() == Some(job_ref)) {
        println!(
            "{} {} {} {} {}",
            artifact_ref,
            receipt.operation,
            receipt.decision,
            receipt.job_ref.unwrap_or_else(|| "-".to_string()),
            receipt.stage_id.unwrap_or_else(|| "-".to_string())
        );
    }
    Ok(())
}
