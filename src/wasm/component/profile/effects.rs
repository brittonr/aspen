/// The synchronous, closed no-WASI effect-request execution cohort.
pub fn hosted() -> super::super::model::Result<super::super::model::RuntimeProfile> {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/config/wasm-component-runtime/generated/effect-profile.json"
    ));
    super::decode(source, "effect profile export is invalid")
}
