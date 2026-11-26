use chaoscontrol_snapshot_descriptor as chaos;
use preserves::IOValue;

use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

// Identity projections. Each destructures its DTO exhaustively (no `..`) and matches enums without
// a wildcard, so an upstream field or variant change fails to compile here instead of silently
// entering or escaping an identity. Every collection is projected as an ordered Preserves sequence
// in its declared order.

pub(super) fn strings_value(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

pub(super) fn msr_indices_value(indices: &[u32]) -> IOValue {
    sequence(indices.iter().map(|index| u64_value(u64::from(*index))).collect())
}

pub(super) fn tagged_digest_value(digest: &chaos::TaggedDigest) -> IOValue {
    let chaos::TaggedDigest { algorithm, hex } = digest;
    let algorithm = match algorithm {
        chaos::DigestAlgorithm::Blake3 => "blake3",
        chaos::DigestAlgorithm::Sha256 => "sha256",
    };
    record("tagged-digest", vec![string(algorithm), string(hex)])
}

fn content_identity_value(content: &chaos::ContentIdentity) -> IOValue {
    let chaos::ContentIdentity {
        digest,
        length_bytes,
        codec,
    } = content;
    record("content-identity", vec![tagged_digest_value(digest), u64_value(*length_bytes), string(codec)])
}

pub(super) fn payload_closure_value(payload: &chaos::PayloadClosure) -> IOValue {
    let chaos::PayloadClosure {
        kind,
        logical_payload,
        manifest,
        members,
    } = payload;
    let kind = match kind {
        chaos::ClosureKind::Monolithic => "monolithic",
        chaos::ClosureKind::Chunked => "chunked",
    };
    let manifest = match manifest {
        Some(manifest) => record("some", vec![content_identity_value(manifest)]),
        None => record("none", Vec::new()),
    };
    let members = members
        .iter()
        .map(|member| {
            let chaos::ClosureMember { order, role, content } = member;
            let role = match role {
                chaos::ClosureRole::SnapshotPayload => "snapshot-payload",
                chaos::ClosureRole::SnapshotChunk => "snapshot-chunk",
            };
            record("closure-member", vec![
                u64_value(u64::from(*order)),
                string(role),
                content_identity_value(content),
            ])
        })
        .collect();
    record("payload-closure", vec![
        string(kind),
        content_identity_value(logical_payload),
        manifest,
        sequence(members),
    ])
}

pub(super) fn devices_value(devices: &[chaos::DeviceCohort]) -> IOValue {
    sequence(
        devices
            .iter()
            .map(|device| {
                let chaos::DeviceCohort {
                    identity,
                    queue_count,
                    backend,
                } = device;
                let chaos::DeviceIdentity {
                    base_address,
                    irq,
                    device_id,
                } = identity;
                let backend = match backend {
                    chaos::DeviceBackend::Network => "network",
                    chaos::DeviceBackend::Block => "block",
                    chaos::DeviceBackend::Entropy => "entropy",
                };
                record("device-cohort", vec![
                    u64_value(*base_address),
                    u64_value(u64::from(*irq)),
                    u64_value(u64::from(*device_id)),
                    u64_value(u64::from(*queue_count)),
                    string(backend),
                ])
            })
            .collect(),
    )
}

pub(super) fn state_owner_value(owner: &chaos::StateOwner) -> IOValue {
    let chaos::StateOwner { owner_id, schema_id } = owner;
    record("state-owner", vec![string(owner_id), string(schema_id)])
}

pub(crate) fn guest_artifact_value(artifact: &chaos::GuestArtifact) -> IOValue {
    let chaos::GuestArtifact { role, content } = artifact;
    let role = match role {
        chaos::GuestArtifactRole::Kernel => "kernel",
        chaos::GuestArtifactRole::Initrd => "initrd",
        chaos::GuestArtifactRole::DiskImage => "disk-image",
        chaos::GuestArtifactRole::GuestBinary => "guest-binary",
    };
    record("guest-artifact", vec![string(role), content_identity_value(content)])
}

pub(crate) fn guest_artifacts_value(artifacts: &[chaos::GuestArtifact]) -> IOValue {
    sequence(artifacts.iter().map(guest_artifact_value).collect())
}
