use std::path::Path;

use molten::error::MoltenError;
use molten::error::Result;

use super::*;

fn pass_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

pub(super) fn finalize(input: FinalizeInput<'_>) -> Result<LocalResult> {
    let mut input = input;
    let final_state_value = molten::coordination::coordination_state_snapshot_value(&input.runtime.state)?;
    let final_state_ref = molten::preserves_rail::canonical_hash(&final_state_value)?;
    input.evidence_values.push(final_state_value)?;
    let evidence_refs = input
        .evidence_values
        .as_slice()
        .iter()
        .map(molten::preserves_rail::canonical_hash)
        .collect::<Result<Vec<_>>>()?;
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let report_value =
        molten::coordination::coordination_apply_report_value(molten::coordination::ApplyReportValueInput {
            decision,
            manifest_ref: input.manifest_ref,
            final_state_ref: &final_state_ref,
            receipt_refs: input.receipt_refs.as_slice(),
            assertion_refs: input.assertion_refs.as_slice(),
            evidence_refs: &evidence_refs,
        })?;
    let report_ref = molten::preserves_rail::canonical_hash(&report_value)?;
    let refs = schedule_refs(&evidence_refs, input.worker);
    let receipt_value = receipt_value(&input, decision, &report_ref, &refs)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(&receipt_value)?;
    write_outputs(&input, input.evidence_values.as_slice(), &report_value, &receipt_value)?;
    if let Some(ledger_root) = input.input.ledger_root {
        molten::ledger::import_artifact(ledger_root, &report_value)?;
        molten::ledger::import_artifact(ledger_root, &receipt_value)?;
    }
    Ok(LocalResult {
        decision: decision.to_string(),
        receipt_ref,
        receipt_value,
        worker: input.worker.cloned(),
    })
}

fn schedule_refs(evidence_refs: &[String], worker: Option<&molten::job_dag::JobWorkerExecution>) -> Vec<String> {
    let mut refs = evidence_refs.to_vec();
    if let Some(worker) = worker {
        refs.push(worker.receipt_ref.clone());
        refs.push(worker.result.result_ref.clone());
    }
    refs
}

fn receipt_value(
    input: &FinalizeInput<'_>,
    decision: &str,
    report_ref: &str,
    refs: &[String],
) -> Result<preserves::IOValue> {
    let worker_receipt_ref = input.worker.map(|worker| worker.receipt_ref.as_str());
    let result_ref = input.worker.map(|worker| worker.result.result_ref.as_str());
    let token_ref = input.lease.and_then(|lease| lease.token.as_ref()).map(|token| token.token_ref.as_str());
    molten::job_dag::job_worker_schedule_receipt_value(molten::job_dag::JobWorkerScheduleReceiptValueInput {
        operation: "worker-schedule-local",
        decision,
        job_ref: &input.request.job_ref,
        request_ref: &input.request.request_ref,
        queue_key: input.input.queue_key,
        lease_key: input.lease_key,
        worker_session: input.input.worker_session,
        coordination_report_ref: report_ref,
        enqueue_receipt_ref: input.enqueue.map(|result| result.receipt.receipt_ref.as_str()),
        enqueue_duplicate_receipt_ref: input.enqueue_duplicate.map(|result| result.receipt.receipt_ref.as_str()),
        dequeue_receipt_ref: input.dequeue.map(|result| result.receipt.receipt_ref.as_str()),
        lease_receipt_ref: input.lease.map(|result| result.receipt.receipt_ref.as_str()),
        release_receipt_ref: input.release.map(|result| result.receipt.receipt_ref.as_str()),
        token_ref,
        worker_receipt_ref,
        result_ref,
        diagnostics: &input.diagnostics,
        refs,
        checks: &[
            ("duplicate-operation-replay", pass_fail(duplicate_replayed(input))),
            ("lease-checked-before-worker", pass_fail(input.worker.is_some() || !input.diagnostics.is_empty())),
            (
                "worker-result-bound",
                pass_fail(input.worker.is_some_and(|worker| worker.result.decision == "pass")),
            ),
        ],
    })
}

fn duplicate_replayed(input: &FinalizeInput<'_>) -> bool {
    input.enqueue_duplicate.is_some_and(|duplicate| {
        input.enqueue.is_some_and(|enqueue| duplicate.receipt.receipt_ref == enqueue.receipt.receipt_ref)
    })
}

fn write_outputs(
    input: &FinalizeInput<'_>,
    evidence_values: &[preserves::IOValue],
    report_value: &preserves::IOValue,
    receipt_value: &preserves::IOValue,
) -> Result<()> {
    std::fs::create_dir_all(input.input.out).map_err(MoltenError::from)?;
    io::write_file(
        &input.input.out.join("schedule-receipt.preserves"),
        &molten::preserves_rail::to_text(receipt_value)?,
    )?;
    let coordination_out = input.input.out.join("coordination");
    std::fs::create_dir_all(&coordination_out).map_err(MoltenError::from)?;
    io::write_file(
        &coordination_out.join("manifest.preserves"),
        &molten::preserves_rail::to_text(&evidence_values[0])?,
    )?;
    io::write_file(&coordination_out.join("report.preserves"), &molten::preserves_rail::to_text(report_value)?)?;
    io::write_indexed_values(&coordination_out, "evidence", evidence_values)?;
    write_optional_receipts(input, &coordination_out)
}

fn write_optional_receipts(input: &FinalizeInput<'_>, coordination_out: &Path) -> Result<()> {
    if let Some(result) = input.enqueue {
        io::write_file(
            &coordination_out.join("enqueue-receipt.preserves"),
            &molten::preserves_rail::to_text(&result.receipt.value)?,
        )?;
    }
    if let Some(result) = input.enqueue_duplicate {
        io::write_file(
            &coordination_out.join("enqueue-duplicate-receipt.preserves"),
            &molten::preserves_rail::to_text(&result.receipt.value)?,
        )?;
    }
    if let Some(result) = input.dequeue {
        io::write_file(
            &coordination_out.join("dequeue-receipt.preserves"),
            &molten::preserves_rail::to_text(&result.receipt.value)?,
        )?;
    }
    if let Some(result) = input.lease {
        io::write_file(
            &coordination_out.join("lease-receipt.preserves"),
            &molten::preserves_rail::to_text(&result.receipt.value)?,
        )?;
        if let Some(token) = &result.token {
            io::write_file(
                &coordination_out.join("fencing-token.preserves"),
                &molten::preserves_rail::to_text(&token.value)?,
            )?;
        }
    }
    if let Some(result) = input.release {
        io::write_file(
            &coordination_out.join("release-receipt.preserves"),
            &molten::preserves_rail::to_text(&result.receipt.value)?,
        )?;
    }
    Ok(())
}
