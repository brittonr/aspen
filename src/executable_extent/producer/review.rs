//! Detached Artifact Auth and pinned Artifact Binding review values.

pub(crate) fn detached(bundle: &super::model::Bundle) -> Result<(), super::Error> {
    const MAX_IDENTIFIER_BYTES: usize = 256;
    const BINDING_REVISION: u64 = 1;
    const MAX_BINDINGS: usize = 1;
    let _scope = artifact_auth_core::AuthenticationScope {
        domain: "onix.molten.executable-extent.authentication.v1".to_string(),
        purpose: "admit-produced-executable-extent-bundle".to_string(),
        profile_id: super::model::BUNDLE_SCHEMA.to_string(),
        subject: artifact_ref(super::model::BUNDLE_SCHEMA, &bundle.bundle_identity_blake3),
        parents: vec![artifact_ref(
            "mantle-source-artifact-v1",
            &bundle.source_artifact_blake3,
        )],
        verifier_context: artifact_ref("mantle-executable-extent-plan-v1", &bundle.plan_identity_blake3),
    };
    let target = artifact_binding_core::ArtifactId::try_new(&bundle.bundle_identity_blake3, MAX_IDENTIFIER_BYTES)
        .map_err(|_error| super::Error::DetachedReview)?;
    let snapshot = artifact_binding_core::SnapshotId::try_new(&bundle.bundle_identity_blake3, MAX_IDENTIFIER_BYTES)
        .map_err(|_error| super::Error::DetachedReview)?;
    artifact_binding_core::resolve(
        &artifact_binding_core::ResolutionRequest::Pinned(artifact_binding_core::PinnedRequest {
            target,
            binding_revision: artifact_binding_core::BindingRevision::new(BINDING_REVISION),
            snapshot,
        }),
        None,
        artifact_binding_core::ResolutionLimits {
            max_bindings: MAX_BINDINGS,
        },
    )
    .map_err(|_error| super::Error::DetachedReview)?;
    Ok(())
}

fn artifact_ref(profile: &str, digest_hex: &str) -> artifact_auth_core::ArtifactRef {
    artifact_auth_core::ArtifactRef {
        profile: profile.to_string(),
        algorithm: artifact_auth_core::ALGORITHM_BLAKE3.to_string(),
        digest_hex: digest_hex.to_string(),
    }
}
