mod admission;
mod evidence;
mod imports;
mod migration;
mod model;
mod profile;
mod runtime;
#[path = "parts/slicing/p000/body.rs"]
mod slicing;

pub use admission::ArtifactFacts;
pub use admission::COMPONENT_INVOKE_EXPORT;
pub use admission::ExecutionPlan;
pub use admission::GrowthFacts;
pub use admission::ImportGrant;
pub use admission::plan_execution;
pub use evidence::Admission;
pub use evidence::AdmissionEnvelope;
pub use evidence::ArtifactSource;
pub use evidence::COMPONENT_ADMISSION_ENVELOPE_SCHEMA;
pub use evidence::COMPONENT_RECEIPT_SCHEMA;
pub use evidence::Decision;
pub use evidence::ExecutionRecord;
pub use evidence::Input;
pub use evidence::MANTLE_COMPONENT_BUNDLE_SCHEMA;
pub use evidence::MantleBundle;
pub use evidence::MaterializedObjectIdentity;
pub use evidence::Stage;
pub use evidence::VerifiedBundle;
pub use evidence::build;
pub use evidence::mantle_bundle_ref;
pub use evidence::replay_receipts_match;
pub use evidence::summary;
pub use evidence::validate;
pub use evidence::validate_against;
pub use evidence::validate_chain;
pub use evidence::value;
pub use evidence::verify;
pub use imports::ADMISSION_EVIDENCE_SCHEMA;
pub use imports::ADMISSION_NON_CLAIMS;
pub use imports::ADMISSION_VERIFIER;
pub use imports::AdmissionEvidence;
pub use imports::AdmissionInput;
pub use imports::AdmissionRecord;
pub use imports::DeclaredManifest;
pub use imports::DeclaredObservation;
pub use imports::DeclaredVerdict;
pub use imports::DeclaredWorld;
pub use imports::MANIFEST_SCHEMA;
pub use imports::ManifestInput;
pub use imports::ManifestSource;
pub use imports::admission_evidence_value;
pub use imports::admit_declared;
pub use imports::build_admission_evidence;
pub use imports::build_manifest;
pub use imports::declared_world;
pub use imports::expected_extraction_tool;
pub use imports::facts_observation;
pub use imports::manifest_value;
pub use imports::observe_artifact_surface;
pub use imports::record_surface_admission;
pub use imports::validate_admission_evidence;
pub use imports::validate_non_claims;
pub use migration::classify_artifact;
pub use migration::classify_for_profile;
pub use model::ArtifactKind;
pub use model::Consumer;
pub use model::Denial;
pub use model::DenialClass;
pub use model::DeterminismProfile;
pub use model::EvidenceScope;
pub use model::FeatureCohort;
pub use model::GrowthStrategy;
pub use model::ProfileExport;
pub use model::RequestedExecutionProfile;
pub use model::ResourceLimits;
pub use model::Result;
pub use model::RuntimeProfile;
pub use model::SlicingProfile;
pub use model::ToolchainCohort;
pub use model::WitCohort;
pub use profile::COMPONENT_NON_CLAIMS;
pub use profile::COMPONENT_PROFILE_ID;
pub use profile::COMPONENT_WIT_PACKAGE;
pub use profile::COMPONENT_WIT_SOURCE_REF;
pub use profile::COMPONENT_WIT_WORLD;
pub use profile::TIME_SLICED_PROFILE_ID;
pub use profile::r#ref;
pub use profile::supported;
pub use profile::time_sliced;
pub use profile::validate_runtime;
pub use runtime::ExecutionOutcome;
pub use runtime::ExecutionRequest;
pub use runtime::SliceProgress;
pub use runtime::SlicedExecution;
pub use runtime::execute;
pub use runtime::start_sliced_execution;
pub use slicing::ContinuationKey;
pub use slicing::PendingTurn;
pub use slicing::SliceBoundary;
pub use slicing::SliceIssue;
pub use slicing::SliceLedger;
pub use slicing::SliceReplayMismatch;
pub use slicing::admit_requested_profile;

#[cfg(test)]
pub(crate) fn test_identity_bytes() -> Vec<u8> {
    tests::support::identity_bytes()
}

#[cfg(test)]
pub(crate) fn test_alternate_bytes() -> Vec<u8> {
    tests::support::invalid_output_bytes()
}

#[cfg(test)]
pub(crate) fn test_precompiled_bytes() -> Vec<u8> {
    let profile = supported().expect("supported component profile");
    let component_bytes = test_identity_bytes();
    runtime::test_precompile(&profile, &component_bytes).expect("precompiled component test fixture")
}

#[cfg(test)]
mod tests;
