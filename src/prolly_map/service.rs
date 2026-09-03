use std::collections::BTreeSet;

use molten_core::prolly_map::*;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProllyServiceError {
    Domain(Vec<ProllyIssue>),
    Port(ProllyPortError),
    Receipt(String),
    GenerationOverflow,
    GcAdmissionDenied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProllyDeletionAdmission {
    pub roots: Vec<NodeRef>,
    pub pins: Vec<NodeRef>,
    pub candidate_unreachable: Vec<NodeRef>,
    pub generation_current: bool,
    pub retention_policy_allows: bool,
    pub deletion_authority_present: bool,
}

pub type ProllyServiceResult<T> = std::result::Result<T, ProllyServiceError>;

// r[impl molten.prolly_map.storage_boundary]
pub fn load_prolly_snapshot(
    port: &impl ProllyBlockStorePort,
    profile: &ProllyProfile,
    root: ProllyRoot,
) -> ProllyServiceResult<MapSnapshot> {
    let mut pending = vec![root.top_node_ref.clone()];
    let mut seen = BTreeSet::new();
    let mut blocks = Vec::new();
    while let Some(node_ref) = pending.pop() {
        if !seen.insert(node_ref.as_str().to_string()) {
            continue;
        }
        if exceeds_graph_bound(seen.len(), profile.limits.max_graph_facts) {
            return Err(ProllyServiceError::Domain(vec![ProllyIssue::GraphLimitExceeded]));
        }
        let bytes = port.read_block(&node_ref).map_err(ProllyServiceError::Port)?.ok_or_else(|| {
            ProllyServiceError::Domain(vec![ProllyIssue::MissingBlock(node_ref.as_str().to_string())])
        })?;
        let block = EncodedBlock {
            node_ref: node_ref.clone(),
            bytes,
        };
        if let ProllyNode::Internal(internal) = decode_block(profile, &block).map_err(ProllyServiceError::Domain)? {
            pending.extend(internal.children.into_iter().map(|child| child.node_ref));
        }
        blocks.push(block);
    }
    let snapshot = MapSnapshot { root, blocks };
    validate_snapshot(profile, &snapshot).map_err(ProllyServiceError::Domain)?;
    Ok(snapshot)
}

pub fn publish_prolly_edit(
    port: &mut impl ProllyBlockStorePort,
    map_id: &str,
    expected: &ExpectedProllyRoot,
    plan: &EditPlan,
) -> ProllyServiceResult<CanonicalProllyPublicationReceipt> {
    validate_snapshot_from_plan(plan)?;
    if expected.root_ref.as_ref().is_some_and(|root_ref| root_ref != &plan.prior_root_ref) {
        return Err(ProllyServiceError::Domain(vec![ProllyIssue::RootIdentityMismatch]));
    }
    port.stage_blocks(&plan.staged_blocks).map_err(ProllyServiceError::Port)?;
    let generation = expected.generation.checked_add(1).ok_or(ProllyServiceError::GenerationOverflow)?;
    let next = PublishedProllyRoot {
        root: plan.next.snapshot.root.clone(),
        generation,
    };
    let observation = match port.compare_and_advance(map_id, expected, &next) {
        Ok(ProllyPublicationObservation::Unknown) => reconcile_publication(port, map_id, expected, &next)?,
        Ok(observation) => observation_status(observation),
        Err(error) if error.outcome_unknown => reconcile_publication(port, map_id, expected, &next)?,
        Err(error) => return Err(ProllyServiceError::Port(error)),
    };
    let mut staged_block_refs = plan.staged_blocks.iter().map(|block| block.node_ref.clone()).collect::<Vec<_>>();
    staged_block_refs.sort();
    let receipt = ProllyPublicationReceipt {
        schema: PROLLY_PUBLICATION_RECEIPT_SCHEMA.to_string(),
        map_id: map_id.to_string(),
        prior_root_ref: expected.root_ref.clone(),
        next_root_ref: next.root.root_ref,
        generation,
        staged_block_refs,
        status: observation,
        authorizes_future_mutation: false,
        deletion_authorized: false,
        non_claims: prolly_receipt_non_claims(),
    };
    canonical_prolly_publication_receipt(&receipt).map_err(|error| ProllyServiceError::Receipt(error.to_string()))
}

pub fn execute_prolly_gc(
    port: &mut impl ProllyBlockStorePort,
    plan: &GcPlan,
    admission: &ProllyDeletionAdmission,
) -> ProllyServiceResult<()> {
    let mut roots = admission.roots.clone();
    roots.sort();
    roots.dedup();
    let mut pins = admission.pins.clone();
    pins.sort();
    pins.dedup();
    let mut candidates = admission.candidate_unreachable.clone();
    candidates.sort();
    candidates.dedup();
    if !plan.complete
        || plan.deletion_authorized
        || roots != plan.roots
        || pins != plan.pins
        || candidates != plan.candidate_unreachable
        || !admission.generation_current
        || !admission.retention_policy_allows
        || !admission.deletion_authority_present
    {
        return Err(ProllyServiceError::GcAdmissionDenied);
    }
    port.delete_blocks(&plan.candidate_unreachable).map_err(ProllyServiceError::Port)
}

fn validate_snapshot_from_plan(plan: &EditPlan) -> ProllyServiceResult<()> {
    if plan.next.snapshot.root.profile_ref != plan.profile_ref
        || plan.next.snapshot.root.root_ref == plan.prior_root_ref
            && plan.edit_count > 0
            && !plan.staged_blocks.is_empty()
    {
        return Err(ProllyServiceError::Domain(vec![ProllyIssue::RootProfileMismatch]));
    }
    Ok(())
}

fn reconcile_publication(
    port: &impl ProllyBlockStorePort,
    map_id: &str,
    expected: &ExpectedProllyRoot,
    next: &PublishedProllyRoot,
) -> ProllyServiceResult<ProllyPublicationStatus> {
    let observed = port.read_root(map_id).map_err(ProllyServiceError::Port)?;
    if observed.as_ref() == Some(next) {
        return Ok(ProllyPublicationStatus::AppliedAfterReconciliation);
    }
    if expected_matches(expected, observed.as_ref()) {
        return Ok(ProllyPublicationStatus::NotAppliedAfterReconciliation);
    }
    Ok(ProllyPublicationStatus::Unknown)
}

fn expected_matches(expected: &ExpectedProllyRoot, observed: Option<&PublishedProllyRoot>) -> bool {
    match (expected.root_ref.as_ref(), observed) {
        (None, None) => expected.generation == 0,
        (Some(root_ref), Some(observed)) => {
            root_ref == &observed.root.root_ref && expected.generation == observed.generation
        }
        _ => false,
    }
}

const fn observation_status(observation: ProllyPublicationObservation) -> ProllyPublicationStatus {
    match observation {
        ProllyPublicationObservation::Applied => ProllyPublicationStatus::Applied,
        ProllyPublicationObservation::AlreadyApplied => ProllyPublicationStatus::AlreadyApplied,
        ProllyPublicationObservation::Stale => ProllyPublicationStatus::Stale,
        ProllyPublicationObservation::Unknown => ProllyPublicationStatus::Unknown,
    }
}

fn exceeds_graph_bound(length: usize, maximum: u32) -> bool {
    match u32::try_from(length) {
        Ok(length) => length > maximum,
        Err(_) => true,
    }
}
