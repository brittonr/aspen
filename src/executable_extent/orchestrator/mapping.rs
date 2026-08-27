//! Admitted member remeasurement, mapping, and explicit teardown.

const MAX_EXTENT_BYTES: usize = 1_048_576;
const MAPPED_DISPOSITION: &str = "mapped-and-unmapped";

pub struct MappedBundle {
    mappings: Vec<Live>,
    bundle: crate::executable_extent::producer::Bundle,
    producer: crate::executable_extent::producer::Receipt,
    profile: molten_core::executable_extent::ExtentCodeRootProfile,
}

struct Live {
    ordinal: u32,
    extent_identity: [u8; blake3::OUT_LEN],
    mapping_identity: [u8; blake3::OUT_LEN],
    mapped_state: executable_extent_core::PermissionState,
    mapping: executable_extent_linux::Mapping,
}

pub(super) struct PreparedFacts {
    pub producer: molten_core::executable_extent::ProducerBundleFacts,
    pub remeasured: Vec<molten_core::executable_extent::RemeasuredExtent>,
    payloads: Vec<Vec<u8>>,
}

struct MemberFacts {
    descriptors: Vec<molten_core::executable_extent::ExtentDescriptor>,
    remeasured: Vec<molten_core::executable_extent::RemeasuredExtent>,
    payloads: Vec<Vec<u8>>,
}

impl MappedBundle {
    /// Explicitly removes every mapping and builds a detached consumer receipt.
    ///
    /// # Errors
    ///
    /// Returns the first unmap or receipt serialization failure.
    pub fn complete(self) -> Result<crate::executable_extent::ConsumerReceipt, super::ConsumeError> {
        let mut observations = Vec::with_capacity(self.mappings.len());
        for owned in self.mappings {
            let unmap = owned.mapping.unmap().map_err(super::ConsumeError::Linux)?;
            if unmap.final_state != executable_extent_core::PermissionState::Unmapped {
                return Err(super::ConsumeError::Invariant);
            }
            observations.push(crate::executable_extent::ConsumerMappingObservation {
                ordinal: owned.ordinal,
                extent_identity_blake3: crate::executable_extent::producer::encode_digest(&owned.extent_identity),
                mapping_identity_blake3: crate::executable_extent::producer::encode_digest(&owned.mapping_identity),
                mapped_state: state_name(owned.mapped_state).to_string(),
                final_state: state_name(unmap.final_state).to_string(),
            });
        }
        crate::executable_extent::record::build(crate::executable_extent::record::ReceiptInput {
            bundle: &self.bundle,
            producer: &self.producer,
            profile: &self.profile,
            disposition: MAPPED_DISPOSITION,
            denial: None,
            mappings: observations,
        })
        .map_err(|_error| super::ConsumeError::Record)
    }
}

pub(super) fn prepare(
    source: &impl crate::executable_extent::BundleSource,
    request: &super::ConsumerRequest,
    bundle: &crate::executable_extent::producer::Bundle,
    producer: &crate::executable_extent::producer::Receipt,
) -> Result<PreparedFacts, super::ConsumeError> {
    let source_identity = crate::executable_extent::producer::decode_digest(&bundle.source_artifact_blake3)
        .map_err(super::ConsumeError::Producer)?;
    let profile = build_profile(request, bundle, producer, source_identity)?;
    let members = read_members(source, bundle)?;
    let endianness = match bundle.endianness.as_str() {
        "little" => executable_extent_core::Endianness::Little,
        _ => {
            return Err(super::ConsumeError::Producer(crate::executable_extent::producer::Error::Profile));
        }
    };
    Ok(PreparedFacts {
        producer: molten_core::executable_extent::ProducerBundleFacts {
            code_root: profile,
            layout_identity: crate::executable_extent::producer::decode_digest(&bundle.layout_identity_blake3)
                .map_err(super::ConsumeError::Producer)?,
            format: bundle.format.clone(),
            architecture: bundle.architecture.clone(),
            abi: bundle.abi.clone(),
            endianness,
            page_size_bytes: bundle.page_size_bytes,
            maximum_virtual_bytes: bundle.maximum_virtual_bytes,
            extents: members.descriptors,
            closure_complete: true,
        },
        remeasured: members.remeasured,
        payloads: members.payloads,
    })
}

fn build_profile(
    request: &super::ConsumerRequest,
    bundle: &crate::executable_extent::producer::Bundle,
    producer: &crate::executable_extent::producer::Receipt,
    source_identity: [u8; blake3::OUT_LEN],
) -> Result<molten_core::executable_extent::ExtentCodeRootProfile, super::ConsumeError> {
    Ok(molten_core::executable_extent::ExtentCodeRootProfile {
        semantic_code: request.semantic_code,
        built_artifact: molten_core::executable_extent::BuiltArtifactIdentity::from_bytes(source_identity),
        extent_manifest: molten_core::executable_extent::ExtentManifestIdentity::from_bytes(
            crate::executable_extent::producer::decode_digest(&bundle.bundle_identity_blake3)
                .map_err(super::ConsumeError::Producer)?,
        ),
        producer_receipt: molten_core::executable_extent::ProducerReceiptIdentity::from_bytes(
            crate::executable_extent::producer::decode_digest(&producer.receipt_identity_blake3)
                .map_err(super::ConsumeError::Producer)?,
        ),
        runtime_cohort: request.consumer.runtime_cohort,
        policy: request.consumer.policy,
    })
}

fn read_members(
    source: &impl crate::executable_extent::BundleSource,
    bundle: &crate::executable_extent::producer::Bundle,
) -> Result<MemberFacts, super::ConsumeError> {
    let mut descriptors = Vec::with_capacity(bundle.extents.len());
    let mut remeasured = Vec::with_capacity(bundle.extents.len());
    let mut payloads = Vec::with_capacity(bundle.extents.len());
    for extent in &bundle.extents {
        let expected_length =
            usize::try_from(extent.length_bytes).map_err(|_error| super::ConsumeError::ExtentRemeasurement)?;
        if expected_length > MAX_EXTENT_BYTES {
            return Err(super::ConsumeError::ExtentRemeasurement);
        }
        let bytes = source.read_leaf(&extent.member_leaf, expected_length).map_err(super::ConsumeError::Source)?;
        if bytes.len() != expected_length {
            return Err(super::ConsumeError::ExtentRemeasurement);
        }
        let measured = *blake3::hash(&bytes).as_bytes();
        let expected = crate::executable_extent::producer::decode_digest(&extent.content_blake3)
            .map_err(super::ConsumeError::Producer)?;
        if measured != expected {
            return Err(super::ConsumeError::ExtentRemeasurement);
        }
        let identity = molten_core::executable_extent::ExecutableExtentIdentity::from_bytes(measured);
        descriptors.push(molten_core::executable_extent::ExtentDescriptor {
            ordinal: extent.ordinal,
            source_offset_bytes: extent.source_offset_bytes,
            virtual_offset_bytes: extent.virtual_offset_bytes,
            length_bytes: extent.length_bytes,
            identity,
            permission: executable_extent_core::ExtentPermission::ExecutableReadOnly,
        });
        remeasured.push(molten_core::executable_extent::RemeasuredExtent {
            ordinal: extent.ordinal,
            length_bytes: extent.length_bytes,
            identity,
        });
        payloads.push(bytes);
    }
    Ok(MemberFacts {
        descriptors,
        remeasured,
        payloads,
    })
}

pub(super) fn map_admitted(
    bundle: crate::executable_extent::producer::Bundle,
    producer: crate::executable_extent::producer::Receipt,
    prepared: PreparedFacts,
) -> Result<super::ConsumeOutcome, super::ConsumeError> {
    let bundle_identity = crate::executable_extent::producer::decode_digest(&bundle.bundle_identity_blake3)
        .map_err(super::ConsumeError::Producer)?;
    let mut mappings = Vec::with_capacity(bundle.extents.len());
    for (extent, bytes) in bundle.extents.iter().zip(&prepared.payloads) {
        mappings.push(map_one(extent, bytes, &bundle, &prepared.producer.code_root, &bundle_identity)?);
    }
    if mappings.len() != bundle.extents.len() || mappings.is_empty() {
        return Err(super::ConsumeError::Invariant);
    }
    Ok(super::ConsumeOutcome::Mapped(Box::new(MappedBundle {
        mappings,
        bundle,
        producer,
        profile: prepared.producer.code_root,
    })))
}

fn map_one(
    extent: &crate::executable_extent::producer::Member,
    bytes: &[u8],
    bundle: &crate::executable_extent::producer::Bundle,
    profile: &molten_core::executable_extent::ExtentCodeRootProfile,
    bundle_identity: &[u8; blake3::OUT_LEN],
) -> Result<Live, super::ConsumeError> {
    let extent_identity = crate::executable_extent::producer::decode_digest(&extent.content_blake3)
        .map_err(super::ConsumeError::Producer)?;
    let shared_extents = [executable_extent_core::Extent {
        offset_bytes: extent.virtual_offset_bytes,
        length_bytes: extent.length_bytes,
        content_digest: executable_extent_core::ContentDigest::from_bytes(extent_identity),
        permission: executable_extent_core::ExtentPermission::ExecutableReadOnly,
    }];
    let target = executable_extent_core::TargetProfile {
        architecture: &bundle.architecture,
        abi: &bundle.abi,
        endianness: executable_extent_core::Endianness::Little,
    };
    let layout = executable_extent_core::Layout {
        schema_version: executable_extent_core::LAYOUT_SCHEMA_VERSION_V1,
        page_profile: executable_extent_core::PageProfile {
            page_size_bytes: bundle.page_size_bytes,
            maximum_virtual_bytes: bundle.maximum_virtual_bytes,
        },
        target,
        extents: &shared_extents,
    };
    let consumer = executable_extent_core::CompatibilityProfile {
        page_size_bytes: bundle.page_size_bytes,
        target,
    };
    let sealed = executable_extent_linux::materialize_and_seal(bytes).map_err(super::ConsumeError::Linux)?;
    let mapping = executable_extent_linux::map_extent(sealed.as_fd(), &layout, &consumer, 0)
        .map_err(super::ConsumeError::Linux)?;
    if mapping.state() != executable_extent_core::PermissionState::ExecutableReadOnly
        || mapping.content_digest().as_bytes() != &extent_identity
    {
        return Err(super::ConsumeError::Invariant);
    }
    let mapping_identity = crate::executable_extent::record::mapping_identity(
        bundle_identity,
        &extent_identity,
        profile.runtime_cohort.as_bytes(),
        profile.policy.as_bytes(),
        extent.ordinal,
    );
    drop(sealed);
    Ok(Live {
        ordinal: extent.ordinal,
        extent_identity,
        mapping_identity,
        mapped_state: mapping.state(),
        mapping,
    })
}

const fn state_name(state: executable_extent_core::PermissionState) -> &'static str {
    match state {
        executable_extent_core::PermissionState::Absent => "absent",
        executable_extent_core::PermissionState::WritableNonExecutableStaging => "writable-nonexecutable-staging",
        executable_extent_core::PermissionState::SealedReadOnly => "sealed-read-only",
        executable_extent_core::PermissionState::MappedReadOnly => "mapped-read-only",
        executable_extent_core::PermissionState::ExecutableReadOnly => "executable-read-only",
        executable_extent_core::PermissionState::Unmapped => "unmapped",
    }
}
