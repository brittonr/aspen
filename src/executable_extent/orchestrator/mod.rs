//! Executable-extent decode, admission, mapping, and teardown shell.

// r[impl molten.world_extents.materialization]
// r[impl molten.world_extents.activation]

mod mapping;

pub use mapping::MappedBundle;

const MAX_BUNDLE_BYTES: usize = 65_536;
const MAX_RECEIPT_BYTES: usize = 65_536;
const INERT_DISPOSITION: &str = "inert";

/// Explicit immutable inputs to one consumer operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerRequest {
    /// Capability-relative bundle manifest leaf.
    pub bundle_manifest_leaf: String,
    /// Capability-relative detached producer receipt leaf.
    pub producer_receipt_leaf: String,
    /// Molten-owned semantic code identity.
    pub semantic_code: molten_core::executable_extent::SemanticCodeIdentity,
    /// Current explicit compatibility profile.
    pub consumer: molten_core::executable_extent::ConsumerProfile,
    /// Whether weaker ordinary artifacts are forbidden.
    pub extent_profile_required: bool,
}

/// Bounded executable-extent consumer failure.
#[derive(Debug)]
pub enum ConsumeError {
    /// Capability-relative member access failed.
    Source(crate::executable_extent::SourceError),
    /// Mantle producer bytes or identities failed admission.
    Producer(crate::executable_extent::producer::Error),
    /// Independent extent remeasurement differed.
    ExtentRemeasurement,
    /// Pure Molten admission denied structural or compatibility facts.
    Admission(molten_core::executable_extent::AdmissionError),
    /// Current admission could not be observed conclusively.
    AdmissionPort(crate::executable_extent::AdmissionPortError),
    /// Shared Linux materialization, mapping, or teardown failed.
    Linux(executable_extent_linux::LinuxError),
    /// Detached consumer receipt serialization failed.
    Record,
    /// A successful plan violated a shell invariant.
    Invariant,
}

/// A valid bundle that remains inert, or owned live mappings.
pub enum ConsumeOutcome {
    /// Current authority denied mapping. No mapping effect occurred.
    Inert(Box<crate::executable_extent::ConsumerReceipt>),
    /// Current authority admitted owned live mappings.
    Mapped(Box<MappedBundle>),
}

/// Reads, remeasures, admits, and conditionally maps one producer bundle.
///
/// # Errors
///
/// Returns the earliest bounded source, producer, remeasurement, admission,
/// mapping, or receipt failure. An unavailable authority observation fails
/// closed.
pub fn consume_bundle(
    source: &impl crate::executable_extent::BundleSource,
    admission: &impl crate::executable_extent::CurrentAdmissionPort,
    request: &ConsumerRequest,
) -> Result<ConsumeOutcome, ConsumeError> {
    let bundle_bytes =
        source.read_leaf(&request.bundle_manifest_leaf, MAX_BUNDLE_BYTES).map_err(ConsumeError::Source)?;
    let bundle = crate::executable_extent::producer::decode_bundle(&bundle_bytes).map_err(ConsumeError::Producer)?;
    let producer_bytes =
        source.read_leaf(&request.producer_receipt_leaf, MAX_RECEIPT_BYTES).map_err(ConsumeError::Source)?;
    let producer =
        crate::executable_extent::producer::decode_receipt(&producer_bytes, &bundle).map_err(ConsumeError::Producer)?;
    if producer.bundle_manifest_leaf != request.bundle_manifest_leaf {
        return Err(ConsumeError::Producer(crate::executable_extent::producer::Error::Linkage));
    }
    crate::executable_extent::producer::detached_review(&bundle).map_err(ConsumeError::Producer)?;

    let prepared = mapping::prepare(source, request, &bundle, &producer)?;
    let activation = admission.observe(&prepared.producer.code_root).map_err(ConsumeError::AdmissionPort)?;
    let decision = molten_core::executable_extent::admit_code_profile(
        &molten_core::executable_extent::CodeProfile::ExecutableExtent(Box::new(prepared.producer.clone())),
        request.extent_profile_required,
        &prepared.remeasured,
        &request.consumer,
        activation,
    )
    .map_err(ConsumeError::Admission)?;
    let molten_core::executable_extent::AdmissionDecision::ExecutableExtents(plan) = decision else {
        return Err(ConsumeError::Invariant);
    };
    match plan.activation {
        molten_core::executable_extent::ActivationDecision::Admit => mapping::map_admitted(bundle, producer, prepared),
        molten_core::executable_extent::ActivationDecision::Deny(denial) => {
            inert(bundle, producer, prepared.producer.code_root, denial)
        }
    }
}

fn inert(
    bundle: crate::executable_extent::producer::Bundle,
    producer: crate::executable_extent::producer::Receipt,
    profile: molten_core::executable_extent::ExtentCodeRootProfile,
    denial: molten_core::executable_extent::ActivationDenial,
) -> Result<ConsumeOutcome, ConsumeError> {
    let receipt = crate::executable_extent::record::build(crate::executable_extent::record::ReceiptInput {
        bundle: &bundle,
        producer: &producer,
        profile: &profile,
        disposition: INERT_DISPOSITION,
        denial: Some(denial_name(denial).to_string()),
        mappings: Vec::new(),
    })
    .map_err(|_error| ConsumeError::Record)?;
    Ok(ConsumeOutcome::Inert(Box::new(receipt)))
}

const fn denial_name(denial: molten_core::executable_extent::ActivationDenial) -> &'static str {
    match denial {
        molten_core::executable_extent::ActivationDenial::ArtifactNotCurrent => "artifact-not-current",
        molten_core::executable_extent::ActivationDenial::RuntimeNotCurrent => "runtime-not-current",
        molten_core::executable_extent::ActivationDenial::ResourcesUnavailable => "resources-unavailable",
        molten_core::executable_extent::ActivationDenial::PolicyNotCurrent => "policy-not-current",
        molten_core::executable_extent::ActivationDenial::ExecutionUnauthorized => "execution-unauthorized",
    }
}
