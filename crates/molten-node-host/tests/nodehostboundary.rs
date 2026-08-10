const REQUIRED_PRODUCTION_DEPENDENCIES: [&str; 3] = ["cap-fs-ext", "cap-std", "molten-core"];

fn production_dependencies(source: &str) -> Result<std::collections::BTreeSet<String>, String> {
    let manifest = source.parse::<toml::Table>().map_err(|error| error.to_string())?;
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "manifest must define a dependencies table".to_string())?;
    Ok(dependencies.keys().cloned().collect())
}

fn validate_production_dependencies(source: &str) -> Result<(), Vec<String>> {
    let dependencies = production_dependencies(source).map_err(|error| vec![error])?;
    let required = REQUIRED_PRODUCTION_DEPENDENCIES.into_iter().collect::<std::collections::BTreeSet<_>>();
    let mut diagnostics = dependencies
        .iter()
        .filter(|dependency| !required.contains(dependency.as_str()))
        .map(|dependency| format!("forbidden node-host dependency: {dependency}"))
        .collect::<Vec<_>>();
    diagnostics.extend(
        required
            .iter()
            .filter(|dependency| !dependencies.contains(**dependency))
            .map(|dependency| format!("missing required node-host dependency: {dependency}")),
    );
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn fixture_source(name: &str) -> std::io::Result<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/node-host-boundary")
        .join(name)
        .join("Cargo.toml");
    std::fs::read_to_string(path)
}

fn require_denial(result: Result<(), Vec<String>>) -> std::io::Result<Vec<String>> {
    match result {
        Ok(()) => Err(std::io::Error::other("dependency boundary unexpectedly admitted invalid input")),
        Err(diagnostics) => Ok(diagnostics),
    }
}

#[test]
fn production_and_positive_manifests_admit_only_node_host_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    // r[verify molten.node_host.crate_boundary]
    let production_result = validate_production_dependencies(include_str!("../Cargo.toml"));
    assert!(production_result.is_ok(), "production manifest diagnostics: {production_result:?}");
    let positive_result = validate_production_dependencies(&fixture_source("positive")?);
    assert!(positive_result.is_ok(), "positive fixture diagnostics: {positive_result:?}");
    Ok(())
}

#[test]
fn forbidden_host_dependencies_are_reported_without_partial_admission() -> Result<(), Box<dyn std::error::Error>> {
    // r[verify molten.node_host.crate_boundary]
    let diagnostics = require_denial(validate_production_dependencies(&fixture_source("invalid-host")?))?;
    for dependency in ["clap", "molten", "molten-release-policy"] {
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.contains(dependency)));
    }
    Ok(())
}

#[test]
fn missing_core_dependency_is_reported() -> Result<(), Box<dyn std::error::Error>> {
    // r[verify molten.node_host.crate_boundary]
    let diagnostics = require_denial(validate_production_dependencies(&fixture_source("invalid-missing-core")?))?;
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "missing required node-host dependency: molten-core")
    );
    Ok(())
}

#[test]
fn malformed_manifest_fails_before_dependency_classification() -> Result<(), Box<dyn std::error::Error>> {
    // r[verify molten.node_host.crate_boundary]
    let diagnostics = require_denial(validate_production_dependencies(&fixture_source("malformed")?))?;
    assert_eq!(diagnostics.len(), 1);
    assert!(!diagnostics[0].is_empty());
    Ok(())
}

#[test]
fn direct_node_host_path_opens_capability_state_and_denies_invalid_locators() -> Result<(), Box<dyn std::error::Error>>
{
    // r[verify molten.node_host.facade_compatibility]
    const MARKER_BYTES: &[u8] = b"marker";
    const MARKER_BYTE_COUNT: u64 = MARKER_BYTES.len() as u64;

    let directory = cap_tempfile::tempdir(cap_tempfile::ambient_authority())?;
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(directory.try_clone()?);
    // r[verify molten.node_host.bridge_authority]
    root.create_layout()?;
    let marker = molten_node_host::node_state::NodeStatePath::parse("receipts/marker.bin")?;
    root.write(&marker, MARKER_BYTES)?;
    assert_eq!(root.read(&marker, MARKER_BYTE_COUNT)?, MARKER_BYTES);
    assert!(molten_node_host::node_state::NodeStatePath::parse("../escape").is_err());
    Ok(())
}
