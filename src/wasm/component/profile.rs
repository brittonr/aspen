//! Shell for the reviewed component runtime cohort: embeds and decodes the generated Nickel
//! exports from `config/wasm-component-runtime/generated/`; validation and identity are the pure
//! `molten_wasm_policy_core::component::profile` rules.

mod effects;
pub use effects::hosted;
pub use molten_wasm_policy_core::component::profile::COMPONENT_NON_CLAIMS;
pub use molten_wasm_policy_core::component::profile::COMPONENT_PROFILE_ID;
pub use molten_wasm_policy_core::component::profile::COMPONENT_WASMPARSER_VERSION;
pub use molten_wasm_policy_core::component::profile::COMPONENT_WIT_PACKAGE;
pub use molten_wasm_policy_core::component::profile::COMPONENT_WIT_SOURCE_REF;
pub use molten_wasm_policy_core::component::profile::COMPONENT_WIT_WORLD;
pub use molten_wasm_policy_core::component::profile::TIME_SLICED_PROFILE_ID;
pub use molten_wasm_policy_core::component::profile::r#ref;
pub use molten_wasm_policy_core::component::profile::validate_runtime;

const PROFILE_EXPORT_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/wasm-component-runtime/generated/profile.json"));
const TIME_SLICED_PROFILE_EXPORT_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/config/wasm-component-runtime/generated/time-sliced-profile.json"
));

// r[impl molten.wasm_component_adoption.patched_engine]
pub fn supported() -> super::model::Result<super::model::RuntimeProfile> {
    decode(PROFILE_EXPORT_JSON, "component profile export is invalid")
}

/// The admitted time-sliced component profile: the base component profile
/// with an independent scheduling quantum and total execution budget.
pub fn time_sliced() -> super::model::Result<super::model::RuntimeProfile> {
    decode(TIME_SLICED_PROFILE_EXPORT_JSON, "time-sliced component profile export is invalid")
}

/// Decodes one embedded export and admits it; decode failures are profile denials.
pub(super) fn decode(source: &str, context: &str) -> super::model::Result<super::model::RuntimeProfile> {
    let exported: super::model::ProfileExport = serde_json::from_str(source).map_err(|error| {
        super::model::Denial::classified(super::model::DenialClass::ProfileDenial, format!("{context}: {error}"))
    })?;
    molten_wasm_policy_core::component::profile::admit_export(exported)
}
