use std::path::Path;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_operator::*;
use molten_core::world_operator::*;

pub(super) fn write_plan(
    run: &WorldOperatorRun,
    plan_out: &Path,
    receipt_out: Option<&Path>,
    summary_out: Option<&Path>,
) -> Result<()> {
    std::fs::write(plan_out, &run.plan_record.bytes)?;
    if let Some(receipt_out) = receipt_out {
        std::fs::write(receipt_out, &run.receipt_record.bytes)?;
    }
    if let Some(summary_out) = summary_out {
        std::fs::write(summary_out, &run.summary_record.bytes)?;
    }
    println!("{}", run.rendered_summary);
    println!("plan_out={}", plan_out.display());
    println!("mutation_executed=false");
    Ok(())
}

pub(super) fn write_apply_denial(run: &WorldOperatorRun, submitted_plan_ref: &str, receipt_out: &Path) -> Result<()> {
    let operation = run
        .plan
        .operations
        .first()
        .ok_or_else(|| MoltenError::invalid_harness("world mutation plan has no operation"))?;
    let blocker = WorldWorkflowBlocker {
        operation_id: operation.operation_id.clone(),
        code: if submitted_plan_ref == run.plan.plan_ref {
            WorldWorkflowBlockerCode::HandlerUnavailable
        } else {
            WorldWorkflowBlockerCode::StalePlan
        },
        evidence_ref: canonical_ref(submitted_plan_ref),
    };
    let receipt = build_world_workflow_receipt(&run.plan, Vec::new(), Some(blocker))
        .map_err(|issues| MoltenError::invalid_harness(format!("world mutation denial receipt failed: {issues:?}")))?;
    let canonical = canonical_world_workflow_receipt(&receipt)?;
    std::fs::write(receipt_out, &canonical.bytes)?;
    println!("receipt_ref={}", receipt.receipt_ref);
    println!("receipt_out={}", receipt_out.display());
    println!("decision=denied");
    println!("mutation_executed=false");
    Err(MoltenError::invalid_harness(
        "world mutation requires an admitted component handler and fresh current-facts adapter",
    ))
}

fn canonical_ref(value: &str) -> Option<String> {
    molten::preserves_rail::validate_content_ref(value).ok().map(|()| value.to_string())
}
