mod admission;
mod evidence;
mod migration;
mod model;
mod profile;
mod runtime;

pub use admission::COMPONENT_INVOKE_EXPORT;
pub use admission::ComponentArtifactFacts;
pub use admission::ComponentExecutionPlan;
pub use admission::ComponentGrowthFacts;
pub use admission::ComponentImportGrant;
pub use admission::plan_component_execution;
pub use evidence::COMPONENT_ADMISSION_ENVELOPE_SCHEMA;
pub use evidence::COMPONENT_RECEIPT_SCHEMA;
pub use evidence::ComponentAdmissionEnvelope;
pub use evidence::ComponentArtifactSource;
pub use evidence::ComponentReceipt;
pub use evidence::ComponentReceiptDecision;
pub use evidence::ComponentReceiptInput;
pub use evidence::ComponentReceiptStage;
pub use evidence::MANTLE_COMPONENT_BUNDLE_SCHEMA;
pub use evidence::MantleComponentBundle;
pub use evidence::MaterializationAdmission;
pub use evidence::MaterializedObjectIdentity;
pub use evidence::build_component_receipt;
pub use evidence::component_receipt_summary;
pub use evidence::component_receipt_value;
pub use evidence::mantle_bundle_ref;
pub use evidence::replay_receipts_match;
pub use evidence::validate_component_receipt;
pub use evidence::validate_component_receipt_against;
pub use evidence::validate_component_receipt_chain;
pub use evidence::verify_materialization;
pub use migration::classify_for_profile;
pub use migration::classify_wasm_artifact;
pub use model::ComponentConsumer;
pub use model::ComponentDenial;
pub use model::ComponentDenialClass;
pub use model::ComponentDeterminismProfile;
pub use model::ComponentFeatureCohort;
pub use model::ComponentProfileExport;
pub use model::ComponentResourceLimits;
pub use model::ComponentResult;
pub use model::ComponentRuntimeProfile;
pub use model::ComponentToolchainCohort;
pub use model::ComponentWitCohort;
pub use model::EvidenceScope;
pub use model::GrowthStrategy;
pub use model::RequestedExecutionProfile;
pub use model::WasmArtifactKind;
pub use profile::COMPONENT_NON_CLAIMS;
pub use profile::COMPONENT_PROFILE_ID;
pub use profile::COMPONENT_WIT_PACKAGE;
pub use profile::COMPONENT_WIT_SOURCE_REF;
pub use profile::COMPONENT_WIT_WORLD;
pub use profile::component_profile_ref;
pub use profile::supported_component_profile;
pub use profile::validate_component_profile;
pub use runtime::ComponentExecutionOutcome;
pub use runtime::ComponentExecutionRequest;
pub use runtime::execute_component;

#[cfg(test)]
pub(crate) fn test_identity_component_bytes() -> Vec<u8> {
    tests::support::identity_component_bytes()
}

#[cfg(test)]
pub(crate) fn test_alternate_component_bytes() -> Vec<u8> {
    tests::support::invalid_output_component_bytes()
}

#[cfg(test)]
pub(crate) fn test_precompiled_component_bytes() -> Vec<u8> {
    let profile = supported_component_profile().expect("supported component profile");
    let component_bytes = test_identity_component_bytes();
    runtime::test_precompile_component(&profile, &component_bytes).expect("precompiled component test fixture")
}

#[cfg(test)]
mod tests;
