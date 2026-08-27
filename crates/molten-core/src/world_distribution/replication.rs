use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;
use crate::content_replication::Manifest;
use crate::content_replication::ReconcileInput;
use crate::content_replication::RepairPolicy;
use crate::content_replication::ReplicaPolicy;
use crate::content_replication::ReplicaRule;
use crate::content_replication::ResourceLimits;
use crate::content_replication::plan;
use crate::content_replication::validate_manifest;

const WORLD_OBJECT_MANIFEST_CONTEXT: &str = "onixresearch.molten.world-distribution.object-manifest.v1";

// r[impl molten.world_distribution.closure]
pub fn world_replication_manifest(
    projection: &WorldDagProjection,
    profile: &WorldReplicationProfile,
) -> Result<Manifest, Vec<WorldDistributionIssue>> {
    if projection.objects.is_empty() || projection.objects.len() > MAX_WORLD_DISTRIBUTION_OBJECTS {
        return Err(vec![WorldDistributionIssue::ObjectLimitExceeded]);
    }
    if profile.max_transfer_bytes == 0 || profile.max_transfer_bytes > MAX_WORLD_DISTRIBUTION_BYTES {
        return Err(vec![WorldDistributionIssue::InvalidBounds("replication-transfer-bytes")]);
    }
    let mut contents = projection
        .objects
        .iter()
        .map(|descriptor| {
            Ok(ReplicaRule {
                content_ref: descriptor.object_ref.as_str().to_string(),
                manifest_ref: object_manifest_ref(&projection.requested, descriptor)?,
                encoded_bytes: descriptor.encoded_bytes,
                protected: true,
                transform_ref: None,
                cleanup_authority_ref: None,
            })
        })
        .collect::<Result<Vec<_>, WorldDistributionIssue>>()
        .map_err(|issue| vec![issue])?;
    contents.sort();
    let manifest = Manifest {
        service_id: profile.service_id.clone(),
        generation: profile.generation,
        membership_epoch: profile.membership_epoch,
        placement_epoch: profile.placement_epoch,
        authority_ref: profile.authority_ref.clone(),
        identity_ref: profile.identity_ref.clone(),
        content_profile_ref: profile.content_profile_ref.clone(),
        transport_profile_ref: profile.transport_profile_ref.clone(),
        retention_policy_ref: profile.retention_policy_ref.clone(),
        evidence_profile_ref: profile.evidence_profile_ref.clone(),
        ports: crate::content_replication::REQUIRED_PORTS.iter().map(ToString::to_string).collect(),
        policy: ReplicaPolicy {
            desired_replicas: profile.desired_replicas,
            minimum_verified_replicas: profile.minimum_verified_replicas,
            minimum_fault_domains: profile.minimum_fault_domains,
        },
        repair: RepairPolicy {
            max_attempts: profile.max_attempts,
            allow_handoff: false,
            cleanup_after_handoff: false,
        },
        resources: ResourceLimits {
            max_concurrent_transfers: profile.max_concurrent_transfers,
            max_transfer_bytes: profile.max_transfer_bytes,
            max_queue_depth: profile.max_queue_depth,
            max_timers: profile.max_timers,
            max_diagnostics: MAX_WORLD_DISTRIBUTION_DIAGNOSTICS,
        },
        contents,
        non_claims: crate::content_replication::NON_CLAIMS.iter().map(ToString::to_string).collect(),
    };
    let issues = validate_manifest(&manifest);
    if !issues.is_empty() {
        return Err(vec![WorldDistributionIssue::ReplicationPlanningDenied(format!(
            "manifest:{issues:?}"
        ))]);
    }
    Ok(manifest)
}

// r[impl molten.world_distribution.closure]
// r[impl molten.world_distribution.partial]
pub fn plan_world_replication(
    projection: &WorldDagProjection,
    request: &WorldReplicationPlanRequest,
) -> Result<WorldReplicationPlan, Vec<WorldDistributionIssue>> {
    let manifest = world_replication_manifest(projection, &request.profile)?;
    validate_replication_inventory(projection, &manifest, &request.inventory)?;
    let shared_plan = plan(&ReconcileInput {
        manifest: manifest.clone(),
        inventory: request.inventory.clone(),
        peers: request.peers.clone(),
        history: request.history.clone(),
        observed_tick: request.observed_tick,
    })
    .map_err(|issue| vec![WorldDistributionIssue::ReplicationPlanningDenied(format!("{issue:?}"))])?;
    let expected = projection.objects.iter().map(|descriptor| descriptor.object_ref.as_str()).collect::<BTreeSet<_>>();
    let unsolicited = shared_plan.actions.iter().find(|action| !expected.contains(action.content_ref.as_str()));
    if let Some(action) = unsolicited {
        return Err(vec![WorldDistributionIssue::ReplicationObjectUnsolicited(
            action.content_ref.clone(),
        )]);
    }
    Ok(WorldReplicationPlan {
        closure_ref: projection.requested.clone(),
        manifest,
        shared_plan,
        activation_authorized: false,
        non_claims: distribution_non_claims(),
    })
}

fn validate_replication_inventory(
    projection: &WorldDagProjection,
    manifest: &Manifest,
    inventory: &crate::content_replication::Inventory,
) -> Result<(), Vec<WorldDistributionIssue>> {
    let manifest_by_ref = manifest
        .contents
        .iter()
        .map(|content| (content.content_ref.as_str(), content))
        .collect::<BTreeMap<_, _>>();
    if manifest_by_ref.len() != projection.objects.len() {
        return Err(vec![WorldDistributionIssue::ReplicationManifestDrift]);
    }
    let expected = projection.objects.iter().map(|descriptor| descriptor.object_ref.as_str()).collect::<BTreeSet<_>>();
    let unknown = inventory.replicas.iter().find(|replica| !expected.contains(replica.content_ref.as_str()));
    if let Some(replica) = unknown {
        return Err(vec![WorldDistributionIssue::ReplicationObjectUnsolicited(
            replica.content_ref.clone(),
        )]);
    }
    Ok(())
}

fn object_manifest_ref(
    closure_ref: &crate::world_commit::WorldCommitRef,
    descriptor: &WorldObjectDescriptor,
) -> Result<String, WorldDistributionIssue> {
    let mut hasher = blake3::Hasher::new_derive_key(WORLD_OBJECT_MANIFEST_CONTEXT);
    update(&mut hasher, closure_ref.as_str())?;
    update(&mut hasher, descriptor.object_ref.as_str())?;
    update(&mut hasher, descriptor.domain.as_str())?;
    update(&mut hasher, descriptor.schema_ref.as_str())?;
    hasher.update(&descriptor.encoded_bytes.to_be_bytes());
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn update(hasher: &mut blake3::Hasher, value: &str) -> Result<(), WorldDistributionIssue> {
    let length = u64::try_from(value.len()).map_err(|_| WorldDistributionIssue::ByteLimitExceeded)?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
    Ok(())
}
