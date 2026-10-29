/// The synchronous, closed no-WASI effect-request execution cohort.
pub fn hosted() -> super::super::model::Result<super::super::model::RuntimeProfile> {
    let source =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/wasm-component-runtime/generated/effect-profile.json"));
    let exported: super::super::model::ProfileExport = serde_json::from_str(source).map_err(|error| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::ProfileDenial,
            format!("effect profile export is invalid: {error}"),
        )
    })?;
    super::validate_export(&exported)?;
    Ok(exported.profile)
}
