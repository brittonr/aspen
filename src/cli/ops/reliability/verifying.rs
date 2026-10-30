pub(super) struct Paths {
    pub state_root: std::path::PathBuf,
    pub node_id: String,
    pub source_binding_ref: String,
    pub startup_receipt: std::path::PathBuf,
    pub health_receipt: std::path::PathBuf,
    pub endpoint_proof: std::path::PathBuf,
}

pub(super) fn run(paths: Paths) -> molten::error::Result<()> {
    if paths.startup_receipt != paths.state_root.join("startup-receipt.preserves") {
        return Err(molten::error::Failure::invalid_harness("verifier startup path is not current rooted startup"));
    }
    let health = paths.health_receipt.strip_prefix(&paths.state_root)
        .map_err(|_| molten::error::Failure::invalid_harness("health receipt outside protected node root"))?;
    let health = health.to_str().ok_or_else(|| molten::error::Failure::invalid_harness("non-UTF8 health locator"))?;
    let descriptor: molten::live_reliability::readiness::EndpointProof = super::read(&paths.endpoint_proof)?;
    let root = molten::node_state::DirectoryRoot::open_existing(&paths.state_root)?;
    let result = molten::live_reliability::readiness::verify(molten::live_reliability::readiness::Input {
        root: &root, node_id: &paths.node_id, source_binding_ref: &paths.source_binding_ref,
        health_path: health, descriptor: &descriptor,
    })?;
    let json = serde_json::to_string(&result).map_err(|error| molten::error::Failure::invalid_harness(error.to_string()))?;
    println!("{json}");
    Ok(())
}
