#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-commit CLI visibly composes operator DTOs, capability roots, and read-only report builders"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "the CLI command keeps the public world-commit spelling used by the top-level command"
)]

use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_commit::LocalWorldCommitStore;
use molten::world_commit::WorldCommitPublicationPort;
use molten::world_commit::WorldImmutableObjectPort;
use molten_core::world_commit::ClosureRequest;
use molten_core::world_commit::ParentClosureObservation;
use molten_core::world_commit::RootClosureObservation;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::plan_restore;
use molten_core::world_commit::replay_class;
use molten_core::world_commit::validate_closure;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldCommitCommand {
    Inspect {
        #[arg(long)]
        state_root: PathBuf,
        commit: String,
    },
    Validate {
        #[arg(long)]
        state_root: PathBuf,
        commit: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Explain {
        #[arg(long)]
        state_root: PathBuf,
        commit: String,
    },
    PlanRestore {
        #[arg(long)]
        state_root: PathBuf,
        commit: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

pub(crate) fn run_world_commit_command(command: WorldCommitCommand) -> Result<()> {
    match command {
        WorldCommitCommand::Inspect { state_root, commit } => inspect(&state_root, &commit),
        WorldCommitCommand::Validate {
            state_root,
            commit,
            out,
        } => validate(&state_root, &commit, out.as_deref()),
        WorldCommitCommand::Explain { state_root, commit } => explain(&state_root, &commit),
        WorldCommitCommand::PlanRestore {
            state_root,
            commit,
            out,
        } => plan(&state_root, &commit, out.as_deref()),
    }
}

fn inspect(state_root: &Path, commit: &str) -> Result<()> {
    let (canonical, _) = load_commit(state_root, commit)?;
    println!("commit_ref={}", canonical.commit_ref);
    println!("version={}", canonical.core.version.as_str());
    println!("profile_kind={}", canonical.core.profile.kind.as_str());
    println!("profile_ref={}", canonical.core.profile.profile_ref);
    println!("parent_count={}", canonical.core.parents.len());
    println!("root_count={}", canonical.core.roots.len());
    for root in &canonical.core.roots {
        println!("root.{}={}", root.kind().as_str(), root.as_str());
    }
    Ok(())
}

fn validate(state_root: &Path, commit: &str, out: Option<&Path>) -> Result<()> {
    let (canonical, store) = load_commit(state_root, commit)?;
    let closure = closure_report(&canonical, &store)?;
    let report = molten::world_commit::canonical_closure_report(&canonical.commit_ref, &closure)?;
    let is_complete = closure.complete;
    write_optional(out, &report.bytes)?;
    println!("commit_ref={}", canonical.commit_ref);
    println!("closure={}", if closure.complete { "complete" } else { "incomplete" });
    println!("closure_report_ref={}", report.report_ref);
    if let Some(kind) = closure.first_missing_root {
        println!("first_missing_root={}", kind.as_str());
    }
    for issue in closure.issues {
        println!("issue={issue:?}");
    }
    if !is_complete {
        return Err(MoltenError::invalid_harness("world commit closure validation denied"));
    }
    Ok(())
}

fn explain(state_root: &Path, commit: &str) -> Result<()> {
    let (canonical, _) = load_commit(state_root, commit)?;
    println!("commit_ref={}", canonical.commit_ref);
    println!("profile_kind={}", canonical.core.profile.kind.as_str());
    println!("completeness=profile-relative");
    for root in &canonical.core.roots {
        println!("root.{}.replay_class={}", root.kind().as_str(), replay_class(root.kind()).as_str());
    }
    for non_claim in molten::world_commit::WORLD_COMMIT_NON_CLAIMS {
        println!("non_claim={}", non_claim.as_str());
    }
    Ok(())
}

fn plan(state_root: &Path, commit: &str, out: Option<&Path>) -> Result<()> {
    let (canonical, store) = load_commit(state_root, commit)?;
    let closure = closure_report(&canonical, &store)?;
    let restore = plan_restore(&canonical.commit_ref, &canonical.core, &closure)
        .map_err(|issue| MoltenError::invalid_harness(format!("world restore planning denied: {issue:?}")))?;
    let canonical_plan = molten::world_commit::canonical_restore_plan(&restore)?;
    write_optional(out, &canonical_plan.bytes)?;
    println!("commit_ref={}", canonical.commit_ref);
    println!("restore_plan_ref={}", canonical_plan.plan_ref);
    println!("step_count={}", restore.steps.len());
    for step in restore.steps {
        println!("step={}", step.kind.as_str());
    }
    Ok(())
}

fn load_commit(
    state_root: &Path,
    commit: &str,
) -> Result<(molten::world_commit::CanonicalWorldCommit, LocalWorldCommitStore)> {
    let commit_ref = WorldCommitRef::new(commit.to_string())
        .map_err(|issue| MoltenError::invalid_harness(format!("invalid world commit ref: {issue:?}")))?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldCommitStore::open(&storage)?;
    let bytes = store
        .read_commit(&commit_ref)
        .map_err(|error| MoltenError::invalid_harness(format!("world commit read failed: {error}")))?;
    let canonical =
        molten::world_commit::parse_canonical_world_commit_with_ref(&bytes, &commit_ref, &operator_bounds())?;
    Ok((canonical, store))
}

fn closure_report(
    canonical: &molten::world_commit::CanonicalWorldCommit,
    store: &LocalWorldCommitStore,
) -> Result<molten_core::world_commit::ClosureReport> {
    let roots = canonical
        .core
        .roots
        .iter()
        .map(|root| observe_root_closure(store, root))
        .collect::<Result<Vec<_>>>()?;
    let parent_graph = observe_parent_graph(store, canonical)?;
    Ok(validate_closure(&ClosureRequest {
        commit_ref: canonical.commit_ref.clone(),
        core: canonical.core.clone(),
        roots,
        parent_graph,
        bounds: operator_bounds(),
    }))
}

fn observe_root_closure(
    store: &LocalWorldCommitStore,
    root: &molten_core::world_commit::WorldRootRef,
) -> Result<RootClosureObservation> {
    let is_present = store
        .contains_root(root)
        .map_err(|error| MoltenError::invalid_harness(format!("world root observation failed: {error}")))?;
    let is_verified = is_present && store.read_root(root).is_ok();
    Ok(RootClosureObservation {
        root: root.clone(),
        object_present: is_present,
        identity_matches: is_verified,
        schema_matches: is_verified,
    })
}

fn observe_parent_graph(
    store: &LocalWorldCommitStore,
    canonical: &molten::world_commit::CanonicalWorldCommit,
) -> Result<Vec<ParentClosureObservation>> {
    let bounds = operator_bounds();
    let mut queue = VecDeque::from(canonical.core.parents.clone());
    let mut seen = BTreeSet::new();
    let mut observations = Vec::with_capacity(bounds.max_closure_objects);
    while let Some(parent) = queue.pop_front() {
        if !seen.insert(parent.clone()) {
            continue;
        }
        if seen.len() > bounds.max_closure_objects {
            return Err(MoltenError::invalid_harness("world commit parent closure exceeds its bound"));
        }
        match store.read_commit(&parent) {
            Ok(bytes) => {
                let parsed = molten::world_commit::parse_canonical_world_commit_with_ref(&bytes, &parent, &bounds)?;
                queue.extend(parsed.core.parents.iter().cloned());
                observations.push(ParentClosureObservation {
                    commit_ref: parent,
                    parents: parsed.core.parents,
                    object_present: true,
                });
            }
            Err(_) => observations.push(ParentClosureObservation {
                commit_ref: parent,
                parents: Vec::new(),
                object_present: false,
            }),
        }
    }
    Ok(observations)
}

fn operator_bounds() -> WorldCommitBounds {
    WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: molten_core::world_commit::MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

fn write_optional(path: Option<&Path>, bytes: &[u8]) -> Result<()> {
    if let Some(path) = path {
        std::fs::write(path, bytes).map_err(MoltenError::from)?;
    }
    Ok(())
}
