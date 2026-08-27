use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use molten_core::dag_sync::DagSchemaRef;
use molten_core::world_commit::RootKind;
use molten_core::world_commit::WorldCommitBounds;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_distribution::MAX_WORLD_DISTRIBUTION_OBJECTS;
use molten_core::world_distribution::WorldCommitObject;
use molten_core::world_distribution::WorldDagProjection;
use molten_core::world_distribution::WorldDagProjectionInput;
use molten_core::world_distribution::WorldRootObject;
use molten_core::world_distribution::project_world_dag;

use crate::error::MoltenError;
use crate::error::Result;
use crate::world_commit::WorldCommitPublicationPort;
use crate::world_commit::WorldImmutableObjectPort;
use crate::world_commit::parse_canonical_world_commit_with_ref;

pub fn load_world_dag_projection<S>(
    store: &S,
    requested: &WorldCommitRef,
    bounds: &WorldCommitBounds,
) -> Result<WorldDagProjection>
where
    S: WorldCommitPublicationPort + WorldImmutableObjectPort,
{
    if bounds.max_closure_objects == 0 || bounds.max_closure_objects > MAX_WORLD_DISTRIBUTION_OBJECTS {
        return Err(MoltenError::invalid_harness("world distribution store bounds exceed the supported object cohort"));
    }
    let mut pending = VecDeque::from([requested.clone()]);
    let mut seen_commits = BTreeSet::new();
    let mut commits = Vec::new();
    let mut roots = BTreeMap::new();
    while let Some(commit_ref) = pending.pop_front() {
        if !seen_commits.insert(commit_ref.clone()) {
            continue;
        }
        if seen_commits.len().saturating_add(roots.len()) > bounds.max_closure_objects {
            return Err(MoltenError::invalid_harness("world distribution closure exceeds its object bound"));
        }
        let bytes = store
            .read_commit(&commit_ref)
            .map_err(|error| MoltenError::invalid_harness(format!("world distribution commit read failed: {error}")))?;
        let canonical = parse_canonical_world_commit_with_ref(&bytes, &commit_ref, bounds)?;
        let schema_root = canonical
            .core
            .roots
            .iter()
            .find(|root| root.kind() == RootKind::Schema)
            .ok_or_else(|| MoltenError::invalid_harness("world commit has no schema root"))?;
        let schema_ref = DagSchemaRef::new(schema_root.as_str().to_string())
            .map_err(|error| MoltenError::invalid_harness(format!("world root schema ref is invalid: {error:?}")))?;
        for root in &canonical.core.roots {
            let root_bytes = store.read_root(root).map_err(|error| {
                MoltenError::invalid_harness(format!("world distribution root read failed: {error}"))
            })?;
            let encoded_bytes = u64::try_from(root_bytes.len())
                .map_err(|_| MoltenError::invalid_harness("world root size exceeds u64"))?;
            let descriptor = WorldRootObject {
                root: root.clone(),
                schema_ref: schema_ref.clone(),
                encoded_bytes,
            };
            if let Some(existing) = roots.insert(root.clone(), descriptor.clone())
                && existing != descriptor
            {
                return Err(MoltenError::invalid_harness(
                    "one world root was observed under incompatible schema descriptors",
                ));
            }
        }
        pending.extend(canonical.core.parents.iter().cloned());
        commits.push(WorldCommitObject {
            commit_ref,
            core: canonical.core,
            canonical_bytes: bytes,
        });
    }
    let input = WorldDagProjectionInput {
        requested: requested.clone(),
        commits,
        roots: roots.into_values().collect(),
        bounds: bounds.clone(),
    };
    project_world_dag(&input)
        .map_err(|issues| MoltenError::invalid_harness(format!("world distribution projection denied: {issues:?}")))
}
