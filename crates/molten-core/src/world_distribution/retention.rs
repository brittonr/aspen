use std::collections::BTreeMap;
use std::collections::BTreeSet;

use artifact_binding_core::ArtifactAttribution;
use artifact_binding_core::GenerationAttribution;
use artifact_binding_core::GenerationId;
use artifact_binding_core::ReachabilitySnapshot;
use artifact_binding_core::RetirementCompleteness;
use artifact_binding_core::RetirementRequest;
use artifact_binding_core::RootClassCompleteness;
use artifact_binding_core::SnapshotId;
use artifact_binding_core::classify_retirement;

mod support;

use self::support::*;
use super::*;

// r[impl molten.world_distribution.retention_roots]
// r[impl molten.world_distribution.gc_boundary]
#[allow(
    tigerstyle::function_length,
    reason = "the pure retention projection keeps completeness, remote uncertainty, reachability, and non-authority fields visible"
)]
pub fn project_world_retention(
    request: &WorldRetentionProjectionRequest,
) -> Result<WorldRetentionReport, Vec<WorldDistributionIssue>> {
    if request.classes.len() > MAX_WORLD_RETENTION_CLASSES {
        return Err(vec![WorldDistributionIssue::RetentionClassLimitExceeded]);
    }
    let class_root_count = bounded_sum(request.classes.iter().map(|observation| observation.roots.len()))?;
    let lease_root_count = bounded_sum(request.remote_leases.iter().map(|lease| lease.roots.len()))?;
    let graph_root_capacity = class_root_count
        .checked_add(lease_root_count)
        .ok_or_else(|| vec![WorldDistributionIssue::RetentionRootLimitExceeded("total".to_string())])?;
    if graph_root_capacity > MAX_WORLD_DISTRIBUTION_OBJECTS {
        return Err(vec![WorldDistributionIssue::RetentionRootLimitExceeded("total".to_string())]);
    }
    let class_evidence_count = bounded_sum(request.classes.iter().map(|observation| observation.evidence_refs.len()))?;
    let evidence_capacity = class_evidence_count
        .checked_add(request.remote_leases.len())
        .ok_or_else(|| vec![WorldDistributionIssue::ObjectLimitExceeded])?;
    if evidence_capacity > MAX_WORLD_DISTRIBUTION_EVIDENCE_REFS {
        return Err(vec![WorldDistributionIssue::ObjectLimitExceeded]);
    }
    validate_content_ref(&request.snapshot_ref, "snapshot")?;
    validate_content_ref(&request.generation_ref, "generation")?;
    let known = request
        .projection
        .objects
        .iter()
        .map(|descriptor| descriptor.object_ref.clone())
        .collect::<BTreeSet<_>>();
    let mut by_class = BTreeMap::new();
    let mut issues = Vec::with_capacity(MAX_WORLD_DISTRIBUTION_EVIDENCE_REFS);
    for observation in &request.classes {
        if by_class.insert(observation.class, observation).is_some() {
            issues.push(WorldDistributionIssue::DuplicateRetentionClass(observation.class.as_str().to_string()));
        }
        validate_class_observation(observation, &known, &mut issues);
    }
    let missing_classes = WorldRetentionClass::all()
        .into_iter()
        .filter(|class| by_class.get(class).is_none_or(|observation| !observation.observed))
        .collect::<Vec<_>>();
    for class in &missing_classes {
        issues.push(WorldDistributionIssue::MissingRetentionClass(class.as_str().to_string()));
    }
    let mut unresolved_remote = Vec::with_capacity(request.remote_leases.len());
    for lease in &request.remote_leases {
        validate_remote_lease(lease, &known, &mut issues);
        if lease.state.unresolved() {
            unresolved_remote.push(lease.lease_ref.clone());
        }
    }
    let has_fatal_issue = issues.iter().any(|issue| !matches!(issue, WorldDistributionIssue::MissingRetentionClass(_)));
    if has_fatal_issue {
        return Err(normalize_issues(issues));
    }

    let is_reference_index_complete = missing_classes.is_empty()
        && unresolved_remote.is_empty()
        && request.edge_inventory_complete
        && request.attribution_inventory_complete;
    let snapshot = SnapshotId::try_new(&request.snapshot_ref, WORLD_BINDING_IDENTIFIER_BYTES).map_err(|error| {
        vec![WorldDistributionIssue::RetentionBindingDenied(format!(
            "snapshot:{error:?}"
        ))]
    })?;
    let generation =
        GenerationId::try_new(&request.generation_ref, WORLD_BINDING_IDENTIFIER_BYTES).map_err(|error| {
            vec![WorldDistributionIssue::RetentionBindingDenied(format!(
                "generation:{error:?}"
            ))]
        })?;
    let mut graph_roots = Vec::with_capacity(graph_root_capacity);
    let mut evidence_refs = Vec::with_capacity(evidence_capacity);
    for observation in by_class.values().filter(|observation| observation.observed) {
        evidence_refs.extend(observation.evidence_refs.iter().cloned());
        for object in &observation.roots {
            graph_roots.push(binding_root(observation.class, &observation.owner_ref, object)?);
        }
    }
    let mut remote_refs = Vec::with_capacity(request.remote_leases.len());
    for lease in &request.remote_leases {
        evidence_refs.push(lease.evidence_ref.clone());
        if lease.state.retains_roots() {
            remote_refs.push(lease.lease_ref.clone());
            for object in &lease.roots {
                graph_roots.push(binding_root(WorldRetentionClass::RemoteLease, &lease.peer_ref, object)?);
            }
        }
    }
    graph_roots.sort_by(|left, right| {
        left.class
            .as_str()
            .cmp(right.class.as_str())
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
            .then_with(|| left.target.as_str().cmp(right.target.as_str()))
    });
    graph_roots.dedup();
    let edges = binding_edges(&request.projection)?;
    let attributions = request
        .projection
        .objects
        .iter()
        .map(|descriptor| {
            Ok(ArtifactAttribution {
                snapshot: snapshot.clone(),
                artifact: artifact(descriptor.object_ref.as_str())?,
                ownership: GenerationAttribution::Exclusive(generation.clone()),
            })
        })
        .collect::<Result<Vec<_>, Vec<WorldDistributionIssue>>>()?;
    if attributions.len() > MAX_WORLD_DISTRIBUTION_OBJECTS {
        return Err(vec![WorldDistributionIssue::ObjectLimitExceeded]);
    }
    let class = binding_root_class()?;
    let decision = classify_retirement(
        &RetirementRequest {
            generation: generation.clone(),
            declared_root_classes: vec![class.clone()],
            graph: ReachabilitySnapshot {
                id: snapshot.clone(),
                roots: graph_roots,
                edges,
            },
            attributions,
            completeness: RetirementCompleteness {
                snapshot,
                root_classes: vec![RootClassCompleteness {
                    class,
                    complete: is_reference_index_complete,
                }],
                edge_inventory_complete: is_reference_index_complete,
                attribution_inventory_complete: is_reference_index_complete,
            },
        },
        binding_limits(),
    )
    .map_err(|error| vec![WorldDistributionIssue::RetentionBindingDenied(format!("{error:?}"))])?;
    let binding_report = WorldBindingReachabilityReport {
        decision,
        observation_only: true,
        retention_authorized: false,
        deletion_authorized: false,
    };
    let mut retained_refs = binding_report
        .decision
        .pin_paths
        .iter()
        .flat_map(|path| path.artifacts.iter().map(|artifact| artifact.as_str().to_string()))
        .collect::<Vec<_>>();
    retained_refs.sort();
    retained_refs.dedup();
    remote_refs.sort();
    remote_refs.dedup();
    evidence_refs.sort();
    evidence_refs.dedup();
    unresolved_remote.sort();
    unresolved_remote.dedup();
    Ok(WorldRetentionReport {
        snapshot_ref: request.snapshot_ref.clone(),
        generation_ref: request.generation_ref.clone(),
        retained_refs,
        remote_refs,
        evidence_refs,
        missing_classes,
        unresolved_remote,
        reference_index_complete: is_reference_index_complete,
        shared_classification: binding_report.decision.classification,
        binding_report,
        observation_only: true,
        retention_authorized: false,
        deletion_authorized: false,
        non_claims: distribution_non_claims(),
    })
}
