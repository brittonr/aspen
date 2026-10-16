pub fn classify_artifact(bytes: &[u8]) -> super::model::Result<super::model::ArtifactKind> {
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|error| {
            super::model::Denial::classified(
                super::model::DenialClass::ProfileDenial,
                format!("WebAssembly artifact classification failed: {error}"),
            )
        })?;
        if let wasmparser::Payload::Version { encoding, .. } = payload {
            return Ok(match encoding {
                wasmparser::Encoding::Module => super::model::ArtifactKind::CoreModule,
                wasmparser::Encoding::Component => super::model::ArtifactKind::Component,
            });
        }
    }
    Err(super::model::Denial::new("WebAssembly artifact has no outer module or component header"))
}

pub fn classify_for_profile(
    requested_profile: super::model::RequestedExecutionProfile,
    bytes: &[u8],
) -> super::model::Result<super::model::ArtifactKind> {
    let kind = classify_artifact(bytes)?;
    if kind != requested_profile.required_kind() {
        return Err(super::model::Denial::new(format!(
            "artifact kind {} does not match requested profile {}; fallback is forbidden",
            kind.as_str(),
            requested_profile.as_str()
        )));
    }
    Ok(kind)
}
