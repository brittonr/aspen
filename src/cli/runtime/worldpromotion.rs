#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world promotion CLI composes explicit request documents and capability-backed readback"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "operator commands retain the public world promotion protocol spelling"
)]

use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_promotion::LocalWorldPromotionStore;
use molten::world_promotion::WorldPromotionTransactionPort;
use molten::world_promotion::canonical_promotion_plan;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::WorldBranchClass;
use molten_core::world_head::WorldBranchId;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_promotion::*;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;
use serde::Deserialize;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldPromotionCommand {
    Plan {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Promote {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        request: PathBuf,
    },
    OutboxInspect {
        #[arg(long)]
        state_root: PathBuf,
    },
    RetryPlan {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        reservation_ref: String,
        #[arg(long)]
        attempt_ref: String,
        #[arg(long)]
        next_attempt_ref: String,
        #[arg(long)]
        acknowledge_duplicate_risk: bool,
    },
    Reconcile {
        #[arg(long)]
        state_root: PathBuf,
    },
    Deny {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        reservation_ref: String,
    },
    Abandon {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        attempt_ref: String,
        #[arg(long)]
        acknowledge_unknown_outcome: bool,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromotionDocument {
    operation_ref: String,
    branch_id: String,
    expected_head: String,
    candidate_head: String,
    expected_generation: u64,
    policy_ref: String,
    authority_ref: String,
    authority_admitted: bool,
    intent_closure_complete: bool,
    simulation_only: bool,
    intents: Vec<IntentDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntentDocument {
    intent_ref: String,
    semantic_ref: String,
    handler_ref: String,
    adapter_ref: String,
    release_class: Option<String>,
}

pub(crate) fn run_world_promotion_command(command: WorldPromotionCommand) -> Result<()> {
    match command {
        WorldPromotionCommand::Plan { request, out } => plan(&request, &out),
        WorldPromotionCommand::Promote { state_root, request } => unavailable_promote(&state_root, &request),
        WorldPromotionCommand::OutboxInspect { state_root } => inspect_outbox(&state_root),
        WorldPromotionCommand::RetryPlan {
            state_root,
            reservation_ref,
            attempt_ref,
            next_attempt_ref,
            acknowledge_duplicate_risk,
        } => retry_plan(&state_root, &reservation_ref, &attempt_ref, &next_attempt_ref, acknowledge_duplicate_risk),
        WorldPromotionCommand::Reconcile { state_root } => reconcile(&state_root),
        WorldPromotionCommand::Deny {
            state_root,
            reservation_ref,
        } => unavailable_mutation(&state_root, "deny", &reservation_ref),
        WorldPromotionCommand::Abandon {
            state_root,
            attempt_ref,
            acknowledge_unknown_outcome,
        } => {
            if !acknowledge_unknown_outcome {
                return Err(MoltenError::invalid_harness("abandon requires explicit unknown-outcome acknowledgement"));
            }
            unavailable_mutation(&state_root, "abandon", &attempt_ref)
        }
    }
}

fn plan(request_path: &Path, out: &Path) -> Result<()> {
    let request = read_request(request_path)?;
    let plan = plan_world_promotion(&request)
        .map_err(|issues| MoltenError::invalid_harness(format!("world promotion planning denied: {issues:?}")))?;
    let canonical = canonical_promotion_plan(&plan)?;
    std::fs::write(out, &canonical.bytes)?;
    println!("plan_ref={}", plan.plan_ref);
    println!("candidate_head={}", plan.after.head);
    println!("reservation_count={}", plan.reservations.len());
    println!("external_effects_completed=false");
    println!("plan_out={}", out.display());
    Ok(())
}

fn unavailable_promote(state_root: &Path, request_path: &Path) -> Result<()> {
    let _store = open_store(state_root)?;
    let request = read_request(request_path)?;
    let plan = plan_world_promotion(&request)
        .map_err(|issues| MoltenError::invalid_harness(format!("world promotion planning denied: {issues:?}")))?;
    println!("plan_ref={}", plan.plan_ref);
    println!("decision=denied");
    println!("issue=current-authority-adapter-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone promotion is disabled until current authority and intent-closure adapters are composed",
    ))
}

fn inspect_outbox(state_root: &Path) -> Result<()> {
    let store = open_store(state_root)?;
    let reservations = store.list_reservations()?;
    println!("reservation_count={}", reservations.len());
    for reservation in reservations {
        println!("reservation.{}={}", reservation.state.as_str(), reservation.reservation_ref);
    }
    println!("dispatch_authorized=false");
    Ok(())
}

fn retry_plan(
    state_root: &Path,
    reservation_ref: &str,
    attempt_ref: &str,
    next_attempt_ref: &str,
    is_acknowledged: bool,
) -> Result<()> {
    let store = open_store(state_root)?;
    let reservation_ref = WorldReleaseReservationRef::new(reservation_ref.to_string()).map_err(reference_error)?;
    let attempt_ref = WorldReleaseAttemptRef::new(attempt_ref.to_string()).map_err(reference_error)?;
    let _next_attempt_ref = WorldReleaseAttemptRef::new(next_attempt_ref.to_string()).map_err(reference_error)?;
    let reservation = store
        .read_reservation(&reservation_ref)?
        .ok_or_else(|| MoltenError::invalid_harness("reservation not found"))?;
    let attempt = store.read_attempt(&attempt_ref)?.ok_or_else(|| MoltenError::invalid_harness("attempt not found"))?;
    println!("reservation_ref={}", reservation.reservation_ref);
    println!("attempt_state={}", attempt.state.as_str());
    println!("duplicate_risk_acknowledged={is_acknowledged}");
    println!("decision=denied");
    println!("issue=current-plan-and-authority-adapters-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone retry planning requires the current immutable promotion plan and authority adapters",
    ))
}

fn reconcile(state_root: &Path) -> Result<()> {
    let store = open_store(state_root)?;
    let reservations = store.list_reservations()?;
    let unresolved = reservations
        .iter()
        .filter(|reservation| {
            matches!(
                reservation.state,
                WorldReleaseState::Uncertain | WorldReleaseState::Conflict | WorldReleaseState::Attempting
            )
        })
        .count();
    println!("reservation_count={}", reservations.len());
    println!("unresolved_count={unresolved}");
    println!("observation_first=true");
    println!("automatic_retry=false");
    Ok(())
}

fn unavailable_mutation(state_root: &Path, action: &str, reference: &str) -> Result<()> {
    let _store = open_store(state_root)?;
    molten::preserves_rail::validate_content_ref(reference)
        .map_err(|_| MoltenError::invalid_harness("operator reference is invalid"))?;
    println!("action={action}");
    println!("reference={reference}");
    println!("decision=denied");
    println!("issue=current-operator-authority-adapter-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone outbox mutation is disabled until current operator authority is composed",
    ))
}

fn read_request(path: &Path) -> Result<WorldPromotionRequest> {
    let document: PromotionDocument = serde_json::from_slice(&std::fs::read(path)?)
        .map_err(|error| MoltenError::invalid_harness(format!("parse promotion request: {error}")))?;
    let policy_ref = WorldHeadPolicyRef::new(document.policy_ref).map_err(head_reference_error)?;
    Ok(WorldPromotionRequest {
        operation_ref: WorldPromotionOperationRef::new(document.operation_ref).map_err(reference_error)?,
        branch_id: WorldBranchId::new(document.branch_id).map_err(head_reference_error)?,
        branch_class: WorldBranchClass::Candidate,
        expected_head: WorldCommitRef::new(document.expected_head).map_err(commit_reference_error)?,
        candidate_head: WorldCommitRef::new(document.candidate_head).map_err(commit_reference_error)?,
        expected_generation: document.expected_generation,
        policy_ref: policy_ref.clone(),
        authority: WorldPromotionAuthorityObservation {
            authority_ref: WorldPromotionAuthorityRef::new(document.authority_ref).map_err(reference_error)?,
            policy_ref,
            observed_generation: document.expected_generation,
            admitted: document.authority_admitted,
        },
        intent_closure_complete: document.intent_closure_complete,
        simulation_only: document.simulation_only,
        intents: document.intents.into_iter().map(parse_intent).collect::<Result<Vec<_>>>()?,
        bounds: WorldPromotionBounds::standard(),
    })
}

fn parse_intent(document: IntentDocument) -> Result<WorldEffectIntent> {
    Ok(WorldEffectIntent {
        intent_ref: WorldEffectIntentRef::new(document.intent_ref).map_err(reference_error)?,
        semantic_ref: WorldSemanticIntentRef::new(document.semantic_ref).map_err(reference_error)?,
        handler_ref: WorldPromotionHandlerRef::new(document.handler_ref).map_err(reference_error)?,
        adapter_ref: WorldPromotionAdapterRef::new(document.adapter_ref).map_err(reference_error)?,
        release_class: document.release_class.map(|value| parse_release_class(&value)).transpose()?,
    })
}

fn parse_release_class(value: &str) -> Result<WorldIntentReleaseClass> {
    match value {
        "release" => Ok(WorldIntentReleaseClass::Release),
        "deny" => Ok(WorldIntentReleaseClass::Deny),
        "simulate" => Ok(WorldIntentReleaseClass::Simulate),
        "retain" => Ok(WorldIntentReleaseClass::Retain),
        _ => Err(MoltenError::invalid_harness("unsupported intent release class")),
    }
}

fn open_store(state_root: &Path) -> Result<LocalWorldPromotionStore> {
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    LocalWorldPromotionStore::open(&storage)
}

fn reference_error(error: WorldPromotionReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world promotion reference: {error:?}"))
}

fn head_reference_error(error: molten_core::world_head::WorldHeadReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world head reference: {error}"))
}

fn commit_reference_error(error: molten_core::world_commit::WorldCommitReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("invalid world commit reference: {error:?}"))
}
