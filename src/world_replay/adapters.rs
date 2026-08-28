use molten_core::content_store_adapter::ContentManifestDescriptor;
use molten_core::world_replay::*;
use preserves::IOValue;

use super::CanonicalWorldReplayRecord;
use super::WORLD_TRANSITION_TRACE_RECORD;
use super::WorldReplayMemberPayload;
use crate::error::MoltenError;
use crate::error::Result;

// r[impl molten.world_replay.capsule]
pub fn world_replay_trace_member(
    trace: &WorldTransitionTrace,
    record: &CanonicalWorldReplayRecord,
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    if record.kind != WORLD_TRANSITION_TRACE_RECORD {
        return Err(MoltenError::invalid_harness("world replay trace adapter received the wrong record kind"));
    }
    member_payload(
        trace.trace_ref.clone(),
        vec![WorldReplayCapsuleMemberRole::Trace],
        WorldReplayMemberCodec::CanonicalPreservesV1,
        WorldReplayMemberProtection::Public,
        record.bytes.clone(),
    )
}

pub fn world_replay_commit_member(
    commit: &crate::world_commit::CanonicalWorldCommit,
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    member_payload(
        commit.commit_ref.as_str().to_string(),
        vec![WorldReplayCapsuleMemberRole::WorldCommit],
        WorldReplayMemberCodec::CanonicalPreservesV1,
        WorldReplayMemberProtection::Public,
        commit.bytes.clone(),
    )
}

pub fn world_replay_snapshot_descriptor_member(
    descriptor: &crate::world_snapshot::CanonicalSnapshotArtifact,
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    member_payload(
        descriptor.artifact_ref.clone(),
        vec![WorldReplayCapsuleMemberRole::SnapshotDescriptor],
        WorldReplayMemberCodec::CanonicalPreservesV1,
        WorldReplayMemberProtection::Public,
        descriptor.bytes.clone(),
    )
}

// r[impl molten.world_replay.capsule]
pub fn world_replay_content_manifest_member(
    descriptor: &ContentManifestDescriptor,
    canonical_manifest_bytes: &[u8],
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    let issues = molten_core::content_store_adapter::validate_manifest_descriptor(descriptor);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("world replay content manifest denied: {issues:?}")));
    }
    let observed_ref = crate::preserves_rail::content_ref_from_bytes(canonical_manifest_bytes);
    if observed_ref != descriptor.manifest_ref {
        return Err(MoltenError::invalid_harness(
            "world replay content manifest bytes do not match the manifest identity",
        ));
    }
    member_payload(
        descriptor.manifest_ref.clone(),
        vec![WorldReplayCapsuleMemberRole::ContentManifest],
        WorldReplayMemberCodec::ContentManifestV1,
        WorldReplayMemberProtection::Public,
        canonical_manifest_bytes.to_vec(),
    )
}

// r[impl molten.world_replay.capsule]
pub fn world_replay_sealed_reproduction_member(
    bundle_value: &IOValue,
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    let bundle = crate::harness::parse_repro_bundle(bundle_value)?;
    if bundle.kind != crate::harness::ReproBundleKind::Report {
        return Err(MoltenError::invalid_harness("failure reproduction bundles cannot satisfy a world replay capsule"));
    }
    if bundle.loss_classification.as_deref() == Some("diagnostic-only") {
        return Err(MoltenError::invalid_harness(
            "diagnostic-only reproduction bundles cannot satisfy a world replay capsule",
        ));
    }
    let protection = if bundle.loss_classification.as_deref() == Some("requires-reveal") {
        let descriptor_ref = bundle.private_bundle_profile_ref.ok_or_else(|| {
            MoltenError::invalid_harness("encrypted reproduction bundle lacks its protection descriptor")
        })?;
        WorldReplayMemberProtection::Ciphertext { descriptor_ref }
    } else {
        WorldReplayMemberProtection::Public
    };
    let bytes = crate::preserves_rail::canonical_bytes(bundle_value)?;
    let object_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    if object_ref != bundle.bundle_ref {
        return Err(MoltenError::invalid_harness("sealed reproduction bundle identity does not match canonical bytes"));
    }
    member_payload(
        object_ref,
        vec![WorldReplayCapsuleMemberRole::SealedReproductionBundle],
        WorldReplayMemberCodec::SealedReproductionBundleV1,
        protection,
        bytes,
    )
}

pub fn world_replay_raw_member(
    roles: Vec<WorldReplayCapsuleMemberRole>,
    protection: WorldReplayMemberProtection,
    bytes: Vec<u8>,
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    let object_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    member_payload(object_ref, roles, WorldReplayMemberCodec::RawBytesV1, protection, bytes)
}

fn member_payload(
    object_ref: String,
    mut roles: Vec<WorldReplayCapsuleMemberRole>,
    codec: WorldReplayMemberCodec,
    protection: WorldReplayMemberProtection,
    bytes: Vec<u8>,
) -> Result<(WorldReplayCapsuleMember, WorldReplayMemberPayload)> {
    crate::preserves_rail::validate_content_ref(&object_ref)
        .map_err(|_| MoltenError::invalid_harness("world replay adapter produced a noncanonical object ref"))?;
    if bytes.is_empty() {
        return Err(MoltenError::invalid_harness("world replay capsule member bytes must not be empty"));
    }
    let byte_length = u64::try_from(bytes.len())
        .map_err(|_| MoltenError::invalid_harness("world replay capsule member length exceeds u64"))?;
    if byte_length > MAX_WORLD_REPLAY_MEMBER_BYTES {
        return Err(MoltenError::invalid_harness("world replay capsule member exceeds the byte bound"));
    }
    roles.sort();
    roles.dedup();
    if roles.is_empty() || roles.len() > MAX_WORLD_REPLAY_ROLES_PER_MEMBER {
        return Err(MoltenError::invalid_harness("world replay capsule member roles are empty or overbound"));
    }
    Ok((
        WorldReplayCapsuleMember {
            object_ref: object_ref.clone(),
            roles,
            codec,
            byte_length,
            protection,
        },
        WorldReplayMemberPayload { object_ref, bytes },
    ))
}
