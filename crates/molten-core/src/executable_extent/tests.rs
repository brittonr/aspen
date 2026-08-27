use super::*;

const DIGEST_BYTE: u8 = 0x41;
const OTHER_DIGEST_BYTE: u8 = 0x42;
const PAGE_BYTES: u64 = 4_096;
const EXTENT_ORDINAL: u32 = 0;

fn digest(byte: u8) -> [u8; blake3::OUT_LEN] {
    [byte; blake3::OUT_LEN]
}

fn code_root() -> ExtentCodeRootProfile {
    ExtentCodeRootProfile {
        semantic_code: SemanticCodeIdentity::from_bytes(digest(DIGEST_BYTE)),
        built_artifact: BuiltArtifactIdentity::from_bytes(digest(DIGEST_BYTE)),
        extent_manifest: ExtentManifestIdentity::from_bytes(digest(DIGEST_BYTE)),
        producer_receipt: ProducerReceiptIdentity::from_bytes(digest(DIGEST_BYTE)),
        runtime_cohort: RuntimeCohortIdentity::from_bytes(digest(DIGEST_BYTE)),
        policy: PolicyIdentity::from_bytes(digest(DIGEST_BYTE)),
    }
}

fn extent() -> ExtentDescriptor {
    ExtentDescriptor {
        ordinal: EXTENT_ORDINAL,
        source_offset_bytes: 0,
        virtual_offset_bytes: 0,
        length_bytes: PAGE_BYTES,
        identity: ExecutableExtentIdentity::from_bytes(digest(DIGEST_BYTE)),
        permission: executable_extent_core::ExtentPermission::ExecutableReadOnly,
    }
}

fn bundle() -> ProducerBundleFacts {
    let descriptor = extent();
    let shared = executable_extent_core::Extent {
        offset_bytes: descriptor.virtual_offset_bytes,
        length_bytes: descriptor.length_bytes,
        content_digest: executable_extent_core::ContentDigest::from_bytes(*descriptor.identity.as_bytes()),
        permission: descriptor.permission,
    };
    let shared_extents = [shared];
    let layout = executable_extent_core::Layout {
        schema_version: executable_extent_core::LAYOUT_SCHEMA_VERSION_V1,
        page_profile: executable_extent_core::PageProfile {
            page_size_bytes: PAGE_BYTES,
            maximum_virtual_bytes: PAGE_BYTES,
        },
        target: executable_extent_core::TargetProfile {
            architecture: "x86_64",
            abi: "linux-gnu",
            endianness: executable_extent_core::Endianness::Little,
        },
        extents: &shared_extents,
    };
    let Ok(layout_identity) = executable_extent_core::identify_layout(&layout) else {
        return invalid_bundle();
    };
    ProducerBundleFacts {
        code_root: code_root(),
        layout_identity: *layout_identity.as_bytes(),
        format: "mantle-flat-page-v1".to_string(),
        architecture: "x86_64".to_string(),
        abi: "linux-gnu".to_string(),
        endianness: executable_extent_core::Endianness::Little,
        page_size_bytes: PAGE_BYTES,
        maximum_virtual_bytes: PAGE_BYTES,
        extents: vec![descriptor],
        closure_complete: true,
    }
}

fn invalid_bundle() -> ProducerBundleFacts {
    ProducerBundleFacts {
        code_root: code_root(),
        layout_identity: digest(OTHER_DIGEST_BYTE),
        format: "invalid".to_string(),
        architecture: String::new(),
        abi: String::new(),
        endianness: executable_extent_core::Endianness::Little,
        page_size_bytes: 0,
        maximum_virtual_bytes: 0,
        extents: Vec::new(),
        closure_complete: false,
    }
}

fn consumer() -> ConsumerProfile {
    ConsumerProfile {
        architecture: "x86_64".to_string(),
        abi: "linux-gnu".to_string(),
        endianness: executable_extent_core::Endianness::Little,
        page_size_bytes: PAGE_BYTES,
        runtime_cohort: RuntimeCohortIdentity::from_bytes(digest(DIGEST_BYTE)),
        policy: PolicyIdentity::from_bytes(digest(DIGEST_BYTE)),
    }
}

const fn activation() -> ActivationFacts {
    ActivationFacts {
        artifact_current: true,
        runtime_current: true,
        resources_available: true,
        policy_current: true,
        execution_authorized: true,
    }
}

fn remeasured() -> [RemeasuredExtent; 1] {
    [RemeasuredExtent {
        ordinal: EXTENT_ORDINAL,
        length_bytes: PAGE_BYTES,
        identity: ExecutableExtentIdentity::from_bytes(digest(DIGEST_BYTE)),
    }]
}

#[test]
fn admits_exact_extents_with_closed_wx_plan() {
    let decision = admit_code_profile(
        &CodeProfile::ExecutableExtent(Box::new(bundle())),
        true,
        &remeasured(),
        &consumer(),
        activation(),
    );
    let Ok(AdmissionDecision::ExecutableExtents(plan)) = decision else {
        return;
    };
    assert_eq!(plan.activation, ActivationDecision::Admit);
    assert_eq!(plan.mappings.len(), 1);
    assert!(plan.mappings[0].protect_executable.is_some());
    assert_eq!(plan.mappings[0].unmap.plan.effect, executable_extent_core::EffectIntent::Unmap);
}

#[test]
fn valid_extents_remain_inert_without_current_authority() {
    let denied = ActivationFacts {
        execution_authorized: false,
        ..activation()
    };
    let decision = admit_code_profile(
        &CodeProfile::ExecutableExtent(Box::new(bundle())),
        true,
        &remeasured(),
        &consumer(),
        denied,
    );
    let Ok(AdmissionDecision::ExecutableExtents(plan)) = decision else {
        return;
    };
    assert_eq!(plan.activation, ActivationDecision::Deny(ActivationDenial::ExecutionUnauthorized));
    assert_eq!(plan.mappings.len(), 1);
}

#[test]
fn rejects_identity_length_and_layout_drift() {
    let mut identity_drift = remeasured();
    identity_drift[0].identity = ExecutableExtentIdentity::from_bytes(digest(OTHER_DIGEST_BYTE));
    assert_eq!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(bundle())),
            true,
            &identity_drift,
            &consumer(),
            activation(),
        ),
        Err(AdmissionError::ExtentIdentityMismatch)
    );
    let mut length_drift = remeasured();
    length_drift[0].length_bytes = 1;
    assert_eq!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(bundle())),
            true,
            &length_drift,
            &consumer(),
            activation(),
        ),
        Err(AdmissionError::ExtentLengthMismatch)
    );
    let mut layout_drift = bundle();
    layout_drift.layout_identity = digest(OTHER_DIGEST_BYTE);
    assert_eq!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(layout_drift)),
            true,
            &remeasured(),
            &consumer(),
            activation(),
        ),
        Err(AdmissionError::LayoutIdentityMismatch)
    );
}

#[test]
fn rejects_target_page_closure_and_cohort_mismatch() {
    let mut target = consumer();
    target.architecture = "aarch64".to_string();
    assert!(matches!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(bundle())),
            true,
            &remeasured(),
            &target,
            activation(),
        ),
        Err(AdmissionError::Compatibility(executable_extent_core::CompatibilityError::ArchitectureMismatch))
    ));
    let mut page = consumer();
    page.page_size_bytes = PAGE_BYTES.saturating_mul(2);
    assert!(matches!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(bundle())),
            true,
            &remeasured(),
            &page,
            activation(),
        ),
        Err(AdmissionError::Compatibility(executable_extent_core::CompatibilityError::PageSizeMismatch))
    ));
    let mut incomplete = bundle();
    incomplete.closure_complete = false;
    assert_eq!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(incomplete)),
            true,
            &remeasured(),
            &consumer(),
            activation(),
        ),
        Err(AdmissionError::IncompleteClosure)
    );
    let mut cohort = consumer();
    cohort.runtime_cohort = RuntimeCohortIdentity::from_bytes(digest(OTHER_DIGEST_BYTE));
    assert_eq!(
        admit_code_profile(
            &CodeProfile::ExecutableExtent(Box::new(bundle())),
            true,
            &remeasured(),
            &cohort,
            activation(),
        ),
        Err(AdmissionError::RuntimeCohortMismatch)
    );
}

#[test]
fn ordinary_artifact_fallback_is_explicit_and_weaker() {
    let identity = BuiltArtifactIdentity::from_bytes(digest(DIGEST_BYTE));
    assert_eq!(
        admit_code_profile(&CodeProfile::OrdinaryArtifact(identity), false, &[], &consumer(), activation(),),
        Ok(AdmissionDecision::OrdinaryArtifact(identity))
    );
    assert_eq!(
        admit_code_profile(&CodeProfile::OrdinaryArtifact(identity), true, &[], &consumer(), activation(),),
        Err(AdmissionError::ExtentProfileRequired)
    );
}
