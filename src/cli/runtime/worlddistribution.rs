#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world distribution CLI composes explicit capability stores and bounded read-only views"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "operator commands retain the public world distribution protocol spelling"
)]

use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::retention::CandidateExplainInput;
use molten::retention::explain_candidate;
use molten::world_commit::LocalWorldCommitStore;
use molten::world_distribution::canonical_world_closure_plan;
use molten::world_distribution::load_world_dag_projection;
use molten::world_head::LocalWorldHeadStore;
use molten::world_head::WorldHeadConflictPort;
use molten_core::dag_sync::DagBounds;
use molten_core::dag_sync::DagEpochRef;
use molten_core::dag_sync::DagPolicyRef;
use molten_core::dag_sync::DagSyncStrategy;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_distribution::MAX_WORLD_DISTRIBUTION_BYTES;
use molten_core::world_distribution::MAX_WORLD_DISTRIBUTION_OBJECTS;
use molten_core::world_distribution::WorldSyncContext;
use molten_core::world_distribution::plan_world_closure;
use molten_core::world_head::WorldBranchId;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldDistributionCommand {
    SyncPlan {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        commit: String,
        #[arg(long)]
        epoch_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        generation: u64,
        #[arg(long)]
        assume_missing: bool,
        #[arg(long)]
        out: PathBuf,
    },
    Sync {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        commit: String,
    },
    Resume {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        commit: String,
        #[arg(long)]
        progress: PathBuf,
    },
    ClosureInspect {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        commit: String,
    },
    ClaimsInspect {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        branch: String,
    },
    PinsInspect {
        #[arg(long)]
        retention_root: PathBuf,
        #[arg(long)]
        object_ref: String,
    },
    RetentionExplain {
        #[arg(long)]
        retention_root: PathBuf,
        #[arg(long)]
        object_ref: String,
        #[arg(long)]
        object_kind: Option<String>,
        #[arg(long)]
        retention_class: Option<String>,
    },
}

pub(crate) fn run_world_distribution_command(command: WorldDistributionCommand) -> Result<()> {
    match command {
        WorldDistributionCommand::SyncPlan {
            state_root,
            commit,
            epoch_ref,
            policy_ref,
            generation,
            assume_missing,
            out,
        } => sync_plan(SyncPlanInput {
            state_root,
            commit,
            epoch_ref,
            policy_ref,
            generation,
            assume_missing,
            out,
        }),
        WorldDistributionCommand::Sync { state_root, commit } => unavailable_sync(&state_root, &commit, None),
        WorldDistributionCommand::Resume {
            state_root,
            commit,
            progress,
        } => unavailable_sync(&state_root, &commit, Some(&progress)),
        WorldDistributionCommand::ClosureInspect { state_root, commit } => closure_inspect(&state_root, &commit),
        WorldDistributionCommand::ClaimsInspect { state_root, branch } => claims_inspect(&state_root, &branch),
        WorldDistributionCommand::PinsInspect {
            retention_root,
            object_ref,
        } => retention_explain(&retention_root, &object_ref, None, None, true),
        WorldDistributionCommand::RetentionExplain {
            retention_root,
            object_ref,
            object_kind,
            retention_class,
        } => retention_explain(&retention_root, &object_ref, object_kind.as_deref(), retention_class.as_deref(), false),
    }
}

struct SyncPlanInput {
    state_root: PathBuf,
    commit: String,
    epoch_ref: String,
    policy_ref: String,
    generation: u64,
    assume_missing: bool,
    out: PathBuf,
}

fn sync_plan(input: SyncPlanInput) -> Result<()> {
    let commit = parse_commit_ref(&input.commit)?;
    let projection = local_projection(&input.state_root, &commit)?;
    let inventory = if input.assume_missing {
        Vec::new()
    } else {
        projection.objects.iter().map(|object| object.object_ref.clone()).collect()
    };
    let plan = plan_world_closure(&projection, &WorldSyncContext {
        inventory,
        progress: None,
        peers: Vec::new(),
        epoch_ref: DagEpochRef::new(input.epoch_ref)
            .map_err(|error| MoltenError::invalid_harness(format!("invalid sync epoch ref: {error:?}")))?,
        generation: input.generation,
        policy_ref: DagPolicyRef::new(input.policy_ref)
            .map_err(|error| MoltenError::invalid_harness(format!("invalid sync policy ref: {error:?}")))?,
        strategy: DagSyncStrategy::Resumable,
        bounds: dag_bounds(),
    })
    .map_err(|issues| MoltenError::invalid_harness(format!("world sync planning denied: {issues:?}")))?;
    let canonical = canonical_world_closure_plan(&plan)?;
    std::fs::write(&input.out, &canonical.bytes)?;
    println!("commit_ref={}", projection.requested);
    println!("plan_ref={}", plan.shared_plan.plan_ref);
    println!("complete={}", plan.complete);
    println!("missing={}", plan.missing.len());
    println!("activation_authorized=false");
    println!("plan_out={}", input.out.display());
    Ok(())
}

fn closure_inspect(state_root: &Path, commit: &str) -> Result<()> {
    let commit = parse_commit_ref(commit)?;
    let projection = local_projection(state_root, &commit)?;
    println!("commit_ref={}", projection.requested);
    println!("objects={}", projection.objects.len());
    println!("total_bytes={}", projection.total_bytes);
    for object in projection.objects {
        println!("object.{}.{}={}", object.domain.as_str(), object.encoded_bytes, object.object_ref.as_str());
    }
    println!("activation_authorized=false");
    Ok(())
}

fn unavailable_sync(state_root: &Path, commit: &str, progress: Option<&Path>) -> Result<()> {
    let commit = parse_commit_ref(commit)?;
    let _projection = local_projection(state_root, &commit)?;
    if let Some(progress) = progress {
        let bytes = std::fs::read(progress)?;
        let _ = molten::preserves_rail::strict_canonical_decode(&bytes)?;
    }
    println!("commit_ref={commit}");
    println!("decision=denied");
    println!("issue=current-authority-and-peer-transport-adapters-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone world synchronization is disabled until current authority, resource, peer, and content adapters are composed",
    ))
}

fn claims_inspect(state_root: &Path, branch: &str) -> Result<()> {
    let branch = WorldBranchId::new(branch)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid world branch: {error}")))?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldHeadStore::open(&storage)?;
    let conflicts = store
        .read_conflicts(&branch)
        .map_err(|error| MoltenError::invalid_harness(format!("read world claim conflicts: {error}")))?;
    println!("branch={branch}");
    println!("conflicts={}", conflicts.len());
    for conflict in conflicts {
        println!("conflict_ref={}", molten::preserves_rail::content_ref_from_bytes(&conflict));
    }
    println!("selection=not-performed");
    Ok(())
}

fn retention_explain(
    root: &Path,
    object_ref: &str,
    object_kind: Option<&str>,
    retention_class: Option<&str>,
    pins_only: bool,
) -> Result<()> {
    let explain = explain_candidate(CandidateExplainInput {
        root,
        object_ref,
        object_kind,
        retention_class,
        action: None,
        subsystem: Some("world-distribution"),
    })?;
    println!("object_ref={}", explain.object_ref);
    println!("pin_count={}", explain.pin_refs.len());
    for pin in explain.pin_refs {
        println!("pin_ref={pin}");
    }
    if !pins_only {
        println!("explain_ref={}", explain.explain_ref);
        println!("gc_plan_count={}", explain.gc_plan_refs.len());
        println!("remote_clearance_count={}", explain.remote_clearance_refs.len());
    }
    println!("deletion_authorized=false");
    Ok(())
}

fn local_projection(
    state_root: &Path,
    commit: &WorldCommitRef,
) -> Result<molten_core::world_distribution::WorldDagProjection> {
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldCommitStore::open(&storage)?;
    load_world_dag_projection(&store, commit, &world_bounds())
}

fn parse_commit_ref(value: &str) -> Result<WorldCommitRef> {
    WorldCommitRef::new(value.to_string())
        .map_err(|error| MoltenError::invalid_harness(format!("invalid world commit ref: {error:?}")))
}

fn world_bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: MAX_WORLD_DISTRIBUTION_OBJECTS,
    }
}

fn dag_bounds() -> DagBounds {
    DagBounds {
        max_nodes: MAX_WORLD_DISTRIBUTION_OBJECTS,
        max_edges: molten_core::dag_sync::MAX_DAG_EDGES,
        max_roots: molten_core::dag_sync::MAX_DAG_ROOTS,
        max_depth: molten_core::dag_sync::MAX_DAG_DEPTH,
        max_bytes: MAX_WORLD_DISTRIBUTION_BYTES,
        max_steps: MAX_WORLD_DISTRIBUTION_OBJECTS,
        max_peers: molten_core::dag_sync::MAX_DAG_PEERS,
    }
}
