#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world CLI converts explicit request documents into the pure workflow core"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "world operator protocol names remain explicit at the product boundary"
)]

use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_operator::plan_world_operator_request;
use molten_core::world_operator::*;

#[path = "world/document.rs"]
mod document;
#[path = "world/output.rs"]
mod output;

use document::read_world_workflow_request;
use output::write_apply_denial;
use output::write_plan;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldCommand {
    Plan(WorldPlanArgs),
    Inspect(WorldReadArgs),
    Checkpoint(WorldMutationArgs),
    Branch(WorldMutationArgs),
    Run(WorldMutationArgs),
    Diff(WorldReadArgs),
    Conflicts(WorldReadArgs),
    Replay(WorldReadArgs),
    Simulate(WorldReadArgs),
    Verify(WorldReadArgs),
    Promote(WorldMutationArgs),
    Export(WorldReadArgs),
    Import(WorldMutationArgs),
    #[command(name = "gc-plan")]
    GarbageCollectionPlan(WorldReadArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct WorldPlanArgs {
    #[arg(long)]
    request: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    receipt_out: Option<PathBuf>,
    #[arg(long)]
    summary_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct WorldReadArgs {
    #[arg(long)]
    request: PathBuf,
    #[arg(long)]
    plan_out: PathBuf,
    #[arg(long)]
    receipt_out: Option<PathBuf>,
    #[arg(long)]
    summary_out: Option<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct WorldMutationArgs {
    #[arg(long)]
    request: PathBuf,
    #[arg(long)]
    plan_out: PathBuf,
    #[arg(long)]
    summary_out: Option<PathBuf>,
    #[arg(long)]
    apply_plan_ref: Option<String>,
    #[arg(long)]
    receipt_out: Option<PathBuf>,
}

// r[impl molten.world_operator.commands]
pub(crate) fn run_world_command(command: WorldCommand) -> Result<()> {
    match command {
        WorldCommand::Plan(args) => plan_complete(args),
        WorldCommand::Inspect(args) => plan_one(WorldOperationKind::Inspect, args),
        WorldCommand::Checkpoint(args) => plan_mutation(WorldOperationKind::Checkpoint, args),
        WorldCommand::Branch(args) => plan_mutation(WorldOperationKind::Branch, args),
        WorldCommand::Run(args) => plan_mutation(WorldOperationKind::Run, args),
        WorldCommand::Diff(args) => plan_one(WorldOperationKind::Diff, args),
        WorldCommand::Conflicts(args) => plan_one(WorldOperationKind::Conflicts, args),
        WorldCommand::Replay(args) => plan_one(WorldOperationKind::Replay, args),
        WorldCommand::Simulate(args) => plan_one(WorldOperationKind::Simulate, args),
        WorldCommand::Verify(args) => plan_one(WorldOperationKind::Verify, args),
        WorldCommand::Promote(args) => plan_mutation(WorldOperationKind::Promote, args),
        WorldCommand::Export(args) => plan_one(WorldOperationKind::Export, args),
        WorldCommand::Import(args) => plan_mutation(WorldOperationKind::Import, args),
        WorldCommand::GarbageCollectionPlan(args) => plan_one(WorldOperationKind::GarbageCollectionPlan, args),
    }
}

fn plan_complete(args: WorldPlanArgs) -> Result<()> {
    let request = read_world_workflow_request(&args.request)?;
    let run = plan_world_operator_request(&request)?;
    write_plan(&run, &args.out, args.receipt_out.as_deref(), args.summary_out.as_deref())
}

fn plan_one(kind: WorldOperationKind, args: WorldReadArgs) -> Result<()> {
    let request = read_world_workflow_request(&args.request)?;
    require_one_operation(&request, kind)?;
    let run = plan_world_operator_request(&request)?;
    write_plan(&run, &args.plan_out, args.receipt_out.as_deref(), args.summary_out.as_deref())
}

fn plan_mutation(kind: WorldOperationKind, args: WorldMutationArgs) -> Result<()> {
    let request = read_world_workflow_request(&args.request)?;
    require_one_operation(&request, kind)?;
    let run = plan_world_operator_request(&request)?;
    write_plan(&run, &args.plan_out, args.receipt_out.as_deref(), args.summary_out.as_deref())?;
    let Some(submitted_plan_ref) = args.apply_plan_ref else {
        return Ok(());
    };
    let receipt_out = args
        .receipt_out
        .ok_or_else(|| MoltenError::invalid_harness("world mutation apply requires an explicit receipt output"))?;
    write_apply_denial(&run, &submitted_plan_ref, &receipt_out)
}

fn require_one_operation(request: &WorldWorkflowRequest, expected: WorldOperationKind) -> Result<()> {
    let Some(operation) = request.operations.first() else {
        return Err(MoltenError::invalid_harness("typed world command requires exactly one operation"));
    };
    if request.operations.len() != 1 || operation.kind != expected {
        return Err(MoltenError::invalid_harness("typed world command does not match its request operation"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "world/tests.rs"]
mod tests;
