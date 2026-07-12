use super::model::ComponentDenial;
use super::model::ComponentDenialClass;
use super::model::ComponentResult;
use super::model::RequestedExecutionProfile;
use super::model::WasmArtifactKind;

pub fn classify_wasm_artifact(bytes: &[u8]) -> ComponentResult<WasmArtifactKind> {
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| {
            ComponentDenial::classified(
                ComponentDenialClass::ProfileDenial,
                format!("WebAssembly artifact classification failed: {error}"),
            )
        })?;
        if let wasmparser::Payload::Version { encoding, .. } = payload {
            return Ok(match encoding {
                wasmparser::Encoding::Module => WasmArtifactKind::CoreModule,
                wasmparser::Encoding::Component => WasmArtifactKind::Component,
            });
        }
    }
    Err(ComponentDenial::new("WebAssembly artifact has no outer module or component header"))
}

pub fn classify_for_profile(
    requested_profile: RequestedExecutionProfile,
    bytes: &[u8],
) -> ComponentResult<WasmArtifactKind> {
    let kind = classify_wasm_artifact(bytes)?;
    if kind != requested_profile.required_kind() {
        return Err(ComponentDenial::new(format!(
            "artifact kind {} does not match requested profile {}; fallback is forbidden",
            kind.as_str(),
            requested_profile.as_str()
        )));
    }
    Ok(kind)
}
