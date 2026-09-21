pub(crate) mod observation;
mod receipt;
pub(crate) mod surface;

pub use observation::AdmissionRecord;
pub use observation::ManifestSource;
pub use observation::observe_artifact_surface;
pub use observation::record_surface_admission;
pub use receipt::ADMISSION_EVIDENCE_SCHEMA;
pub use receipt::ADMISSION_NON_CLAIMS;
pub use receipt::AdmissionEvidence;
pub use receipt::AdmissionInput;
pub use receipt::admission_evidence_value;
pub use receipt::build_admission_evidence;
pub use receipt::validate_admission_evidence;
pub use receipt::validate_non_claims;
pub use surface::DeclaredManifest;
pub use surface::DeclaredObservation;
pub use surface::DeclaredVerdict;
pub use surface::DeclaredWorld;
pub use surface::MANIFEST_SCHEMA;
pub use surface::ManifestInput;
pub use surface::admit_declared;
pub use surface::build_manifest;
pub use surface::declared_world;
pub use surface::facts_observation;
pub use surface::manifest_value;

/// Identity of the verifier that owns declared-surface admission.
pub const ADMISSION_VERIFIER: &str = "molten.wasm-import-admission/1";

/// Expected extraction tool identity pinned to the admitted wasmparser cohort.
pub fn expected_extraction_tool() -> String {
    format!("wasmparser/{}", super::profile::COMPONENT_WASMPARSER_VERSION)
}
