#![allow(
    tigerstyle::non_trait_imports,
    reason = "the world-merge CLI composes explicit commits, root maps, profiles, and bounded operator output"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "operator commands retain the public world-merge protocol spelling"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::world_commit::CanonicalWorldCommit;
use molten::world_commit::LocalWorldCommitStore;
use molten::world_commit::WorldCommitPublicationPort;
use molten::world_merge::canonical_world_diff;
use molten::world_merge::canonical_world_merge_plan;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_commit::WorldRootRef;
use molten_core::world_merge::WorldMergeBounds;
use molten_core::world_merge::WorldMergeMode;
use molten_core::world_merge::WorldMergePolicyRef;
use molten_core::world_merge::WorldMergeProfile;
use molten_core::world_merge::WorldMergeProfileRef;
use molten_core::world_merge::WorldMergeRequest;
use molten_core::world_merge::WorldMergeRootInput;
use molten_core::world_merge::WorldMergeSchemaRef;
use molten_core::world_merge::WorldMergeValue;
use molten_core::world_merge::diff_world_roots;
use molten_core::world_merge::plan_world_merge;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

const MAX_ANCESTRY_COMMITS: usize = 4_096;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum WorldMergeCommand {
    Diff {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        base: String,
        #[arg(long)]
        left: String,
        #[arg(long)]
        right: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    MergePlan {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        base: String,
        #[arg(long)]
        left: String,
        #[arg(long)]
        right: String,
        #[arg(long)]
        profile_ref: String,
        #[arg(long)]
        policy_ref: String,
        #[arg(long)]
        out: PathBuf,
    },
    ConflictInspect {
        #[arg(long)]
        conflict: PathBuf,
    },
    MergePublish {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        plan: PathBuf,
    },
}

pub(crate) fn run_world_merge_command(command: WorldMergeCommand) -> Result<()> {
    match command {
        WorldMergeCommand::Diff {
            state_root,
            base,
            left,
            right,
            out,
        } => diff(&state_root, &base, &left, &right, out.as_deref()),
        WorldMergeCommand::MergePlan {
            state_root,
            base,
            left,
            right,
            profile_ref,
            policy_ref,
            out,
        } => plan(PlanInput {
            state_root,
            base,
            left,
            right,
            profile_ref,
            policy_ref,
            out,
        }),
        WorldMergeCommand::ConflictInspect { conflict } => inspect_conflict(&conflict),
        WorldMergeCommand::MergePublish { state_root, plan } => publish(&state_root, &plan),
    }
}

fn diff(state_root: &Path, base: &str, left: &str, right: &str, out: Option<&Path>) -> Result<()> {
    let (_, request) = load_request(state_root, base, left, right, default_profile()?)?;
    let report = diff_world_roots(&request)
        .map_err(|issues| MoltenError::invalid_harness(format!("world diff denied: {issues:?}")))?;
    let canonical = canonical_world_diff(&report)?;
    if let Some(out) = out {
        std::fs::write(out, &canonical.bytes)?;
    }
    println!("diff_ref={}", canonical.report_ref);
    for root in report.roots {
        println!("root.{}={}", root.kind.as_str(), root.class.as_str());
    }
    Ok(())
}

struct PlanInput {
    state_root: PathBuf,
    base: String,
    left: String,
    right: String,
    profile_ref: String,
    policy_ref: String,
    out: PathBuf,
}

fn plan(input: PlanInput) -> Result<()> {
    let profile = conservative_profile(&input.profile_ref, &input.policy_ref)?;
    let (_, request) = load_request(&input.state_root, &input.base, &input.left, &input.right, profile)?;
    let plan = plan_world_merge(&request, None)
        .map_err(|issues| MoltenError::invalid_harness(format!("world merge planning denied: {issues:?}")))?;
    let canonical = canonical_world_merge_plan(&plan)?;
    std::fs::write(&input.out, &canonical.bytes)?;
    println!("plan_ref={}", plan.plan_ref);
    println!("plan_artifact_ref={}", canonical.plan_ref);
    println!("conflict_count={}", plan.conflicts.len());
    println!("mutation=not-performed");
    Ok(())
}

fn inspect_conflict(path: &Path) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let decoded = molten::preserves_rail::strict_canonical_decode(&bytes)?;
    println!("conflict_ref={}", decoded.value_ref);
    println!("{}", molten::preserves_rail::to_text(&decoded.value)?);
    Ok(())
}

fn publish(state_root: &Path, plan: &Path) -> Result<()> {
    let _root = NodeStateRoot::open_existing(state_root)?;
    let bytes = std::fs::read(plan)?;
    let decoded = molten::preserves_rail::strict_canonical_decode(&bytes)?;
    println!("plan_artifact_ref={}", decoded.value_ref);
    println!("decision=denied");
    println!("issue=current-merge-authority-adapter-unavailable");
    Err(MoltenError::invalid_harness(
        "standalone merge publication is disabled until authority, migration, and handler adapters are composed",
    ))
}

fn load_request(
    state_root: &Path,
    base: &str,
    left: &str,
    right: &str,
    profile: WorldMergeProfile,
) -> Result<(LocalWorldCommitStore, WorldMergeRequest)> {
    let base_ref = parse_commit_ref(base)?;
    let left_ref = parse_commit_ref(left)?;
    let right_ref = parse_commit_ref(right)?;
    let root = NodeStateRoot::open_existing(state_root)?;
    let storage = root.namespace(NodeStateNamespaceKind::Storage)?;
    let store = LocalWorldCommitStore::open(&storage)?;
    let base_commit = load_commit(&store, &base_ref)?;
    let left_commit = load_commit(&store, &left_ref)?;
    let right_commit = load_commit(&store, &right_ref)?;
    let is_left_descendant = is_ancestor(&store, &base_ref, &left_commit)?;
    let is_right_descendant = is_ancestor(&store, &base_ref, &right_commit)?;
    let roots = root_inputs(&base_commit, &left_commit, &right_commit)?;
    Ok((store, WorldMergeRequest {
        base_head: base_ref,
        source_heads: vec![left_ref, right_ref],
        common_ancestor_verified: is_left_descendant && is_right_descendant,
        common_ancestor_ambiguous: false,
        roots,
        profile,
        bounds: WorldMergeBounds::standard(),
    }))
}

fn load_commit(store: &LocalWorldCommitStore, reference: &WorldCommitRef) -> Result<CanonicalWorldCommit> {
    let bytes = store
        .read_commit(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("world commit read failed: {error}")))?;
    molten::world_commit::parse_canonical_world_commit_with_ref(&bytes, reference, &operator_world_bounds())
}

fn is_ancestor(
    store: &LocalWorldCommitStore,
    ancestor: &WorldCommitRef,
    candidate: &CanonicalWorldCommit,
) -> Result<bool> {
    if &candidate.commit_ref == ancestor {
        return Ok(true);
    }
    let mut queue = VecDeque::from(candidate.core.parents.clone());
    let mut seen = BTreeSet::new();
    while let Some(reference) = queue.pop_front() {
        if &reference == ancestor {
            return Ok(true);
        }
        if !seen.insert(reference.clone()) {
            continue;
        }
        if seen.len() > MAX_ANCESTRY_COMMITS {
            return Err(MoltenError::invalid_harness("world merge ancestry exceeds its bound"));
        }
        let parent = load_commit(store, &reference)?;
        queue.extend(parent.core.parents);
    }
    Ok(false)
}

fn root_inputs(
    base: &CanonicalWorldCommit,
    left: &CanonicalWorldCommit,
    right: &CanonicalWorldCommit,
) -> Result<Vec<WorldMergeRootInput>> {
    let base_roots = roots_by_kind(base);
    let left_roots = roots_by_kind(left);
    let right_roots = roots_by_kind(right);
    let kinds = base_roots
        .keys()
        .chain(left_roots.keys())
        .chain(right_roots.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let base_schema = schema_for(base)?;
    let left_schema = schema_for(left)?;
    let right_schema = schema_for(right)?;
    Ok(kinds
        .into_iter()
        .map(|kind| WorldMergeRootInput {
            kind,
            base: merge_value(base_roots.get(&kind), base_schema.clone()),
            left: merge_value(left_roots.get(&kind), left_schema.clone()),
            right: merge_value(right_roots.get(&kind), right_schema.clone()),
        })
        .collect())
}

fn roots_by_kind(commit: &CanonicalWorldCommit) -> BTreeMap<RootKind, WorldRootRef> {
    commit.core.roots.iter().map(|root| (root.kind(), root.clone())).collect()
}

fn schema_for(commit: &CanonicalWorldCommit) -> Result<Option<WorldMergeSchemaRef>> {
    commit
        .core
        .roots
        .iter()
        .find(|root| root.kind() == RootKind::Schema)
        .map(|root| {
            WorldMergeSchemaRef::new(root.as_str().to_string())
                .map_err(|error| MoltenError::invalid_harness(format!("invalid merge schema ref: {error}")))
        })
        .transpose()
}

fn merge_value(root: Option<&WorldRootRef>, schema_ref: Option<WorldMergeSchemaRef>) -> WorldMergeValue {
    WorldMergeValue {
        root: root.cloned(),
        schema_ref,
        available: true,
        canonical_bytes: None,
        keyed_values: BTreeMap::new(),
    }
}

fn default_profile() -> Result<WorldMergeProfile> {
    conservative_profile(&reference("diff-profile"), &reference("diff-policy"))
}

fn conservative_profile(profile_ref: &str, policy_ref: &str) -> Result<WorldMergeProfile> {
    Ok(WorldMergeProfile {
        profile_ref: WorldMergeProfileRef::new(profile_ref.to_string())
            .map_err(|error| MoltenError::invalid_harness(format!("invalid merge profile ref: {error}")))?,
        policy_ref: WorldMergePolicyRef::new(policy_ref.to_string())
            .map_err(|error| MoltenError::invalid_harness(format!("invalid merge policy ref: {error}")))?,
        root_modes: BTreeMap::from([
            (RootKind::Artifact, WorldMergeMode::IdenticalOnly),
            (RootKind::Schema, WorldMergeMode::IdenticalOnly),
            (RootKind::DurableState, WorldMergeMode::AncestorReplacement),
            (RootKind::RuntimeProfile, WorldMergeMode::IdenticalOnly),
            (RootKind::Policy, WorldMergeMode::IdenticalOnly),
        ]),
        migrations: BTreeMap::new(),
        handlers: BTreeMap::new(),
    })
}

fn operator_world_bounds() -> molten_core::world_commit::WorldCommitBounds {
    molten_core::world_commit::WorldCommitBounds {
        max_parents: molten_core::world_commit::MAX_WORLD_COMMIT_PARENTS,
        max_roots: molten_core::world_commit::MAX_WORLD_COMMIT_ROOTS,
        max_revision_fences: molten_core::world_commit::MAX_WORLD_COMMIT_REVISION_FENCES,
        max_closure_objects: molten_core::world_commit::MAX_WORLD_COMMIT_CLOSURE_OBJECTS,
    }
}

fn parse_commit_ref(value: &str) -> Result<WorldCommitRef> {
    WorldCommitRef::new(value.to_string())
        .map_err(|error| MoltenError::invalid_harness(format!("invalid world commit ref: {error:?}")))
}

fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}
