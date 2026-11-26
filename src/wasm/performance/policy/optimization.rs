//! Shell for optimization policy: the pure rules live in
//! `molten_wasm_policy_core::performance::optimization`; conformance binds the embedded supported
//! component runtime profile.

pub use molten_wasm_policy_core::performance::optimization::BASELINE_OPTIMIZATION_PROFILE_ID;
pub use molten_wasm_policy_core::performance::optimization::COW_OPTIMIZATION_PROFILE_ID;
pub use molten_wasm_policy_core::performance::optimization::ConformanceRecord;
pub use molten_wasm_policy_core::performance::optimization::INSTANCE_PRE_OPTIMIZATION_PROFILE_ID;
pub use molten_wasm_policy_core::performance::optimization::POOLING_OPTIMIZATION_PROFILE_ID;
pub use molten_wasm_policy_core::performance::optimization::admit_capacity;
pub use molten_wasm_policy_core::performance::optimization::configuration_ref;
pub use molten_wasm_policy_core::performance::optimization::conformance_record_ref;
pub use molten_wasm_policy_core::performance::optimization::profile_ref;
pub use molten_wasm_policy_core::performance::optimization::validate_profile;

/// Validates conformance against the embedded supported component runtime profile.
pub fn validate_conformance(
    optimization: &super::model::OptimizationProfile,
    record: &ConformanceRecord,
) -> super::model::Result<()> {
    let component_profile = crate::wasm_component::supported().map_err(|error| {
        super::model::Denial::new(format!("component profile required by optimization conformance is invalid: {error}"))
    })?;
    molten_wasm_policy_core::performance::optimization::validate_conformance(optimization, record, &component_profile)
}
