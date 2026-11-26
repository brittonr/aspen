//! Shell for the reviewed performance profile: embeds and decodes the generated Nickel export
//! `config/wasm-component-performance/generated/profile.json`; validation and identity are the pure
//! `molten_wasm_policy_core::performance::profile` rules.

pub use molten_wasm_policy_core::performance::profile::PERFORMANCE_COMPONENT_PROFILE_ID;
pub use molten_wasm_policy_core::performance::profile::PERFORMANCE_NON_CLAIMS;
pub use molten_wasm_policy_core::performance::profile::PERFORMANCE_PROFILE_ID;
pub use molten_wasm_policy_core::performance::profile::PERFORMANCE_PROFILE_SCHEMA;
pub use molten_wasm_policy_core::performance::profile::SIGHTGLASS_REVISION;
pub use molten_wasm_policy_core::performance::profile::WASMTIME_COMPONENT_COHORT;
pub use molten_wasm_policy_core::performance::profile::r#ref;
pub use molten_wasm_policy_core::performance::profile::suite_ref;
pub use molten_wasm_policy_core::performance::profile::validate;

const PROFILE_EXPORT_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/wasm-component-performance/generated/profile.json"));

pub fn supported() -> super::model::Result<super::model::Profile> {
    let exported: super::model::ProfileExport = serde_json::from_str(PROFILE_EXPORT_JSON).map_err(|error| {
        super::model::Denial::new(format!("Wasm component performance profile export is invalid: {error}"))
    })?;
    molten_wasm_policy_core::performance::profile::admit_export(exported)
}
