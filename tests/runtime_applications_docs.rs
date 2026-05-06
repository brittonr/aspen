const RUNTIME_DOC: &str = include_str!("../docs/runtime-applications.md");
const JOBS_CORE: &str = include_str!("../crates/aspen-jobs-core/src/lib.rs");
const CI_CORE: &str = include_str!("../crates/aspen-ci-core/src/lib.rs");
const PLUGIN_MANIFEST: &str = include_str!("../crates/aspen-plugin-api/src/manifest.rs");
const DEPLOY_TYPES: &str = include_str!("../crates/aspen-deploy/src/types.rs");
const RPC_REGISTRY: &str = include_str!("../crates/aspen-rpc-handlers/src/registry.rs");
const FORGE_NODE: &str = include_str!("../crates/aspen-forge/src/node.rs");

fn read_repo_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[test]
fn runtime_applications_doc_is_discoverable() {
    let Some(readme) = read_repo_file("README.md") else {
        return;
    };
    let Some(architecture) = read_repo_file("docs/developer-guide/architecture.md") else {
        return;
    };

    assert!(readme.contains("docs/runtime-applications.md"));
    assert!(readme.contains("Runtime Applications"));
    assert!(architecture.contains("Runtime Applications"));
    assert!(architecture.contains("../runtime-applications.md"));
    assert!(std::path::Path::new("docs/runtime-applications.md").exists());
}

#[test]
fn runtime_applications_doc_preserves_core_contract_terms() {
    for term in [
        "Application",
        "ServiceSpec",
        "ServiceInstance",
        "ExecutionPlan",
        "ExecutionRun",
        "Executioner",
        "Flawless",
        "Temporal",
        "Event History",
        "side-effect history",
        "RuntimeArtifact",
        "RuntimeCapabilityBinding",
        "RuntimeReceipt",
        "Plugin",
        "Adapter",
        "Overlooked or high-risk seams",
    ] {
        assert!(RUNTIME_DOC.contains(term), "runtime doc should mention {term}");
    }
}

#[test]
fn runtime_applications_doc_tracks_current_source_anchors() {
    assert!(RUNTIME_DOC.contains("crates/aspen-jobs-core/src/lib.rs"));
    assert!(RUNTIME_DOC.contains("crates/aspen-ci-core/src/lib.rs"));
    assert!(RUNTIME_DOC.contains("crates/aspen-plugin-api/src/manifest.rs"));
    assert!(RUNTIME_DOC.contains("crates/aspen-deploy/src/types.rs"));
    assert!(RUNTIME_DOC.contains("crates/aspen-rpc-handlers/src/registry.rs"));
    assert!(RUNTIME_DOC.contains("crates/aspen-forge/src/node.rs"));

    assert!(JOBS_CORE.contains("pub struct JobSpec"));
    assert!(JOBS_CORE.contains("pub enum Schedule"));
    assert!(CI_CORE.contains("pub use config::PipelineConfig"));
    assert!(PLUGIN_MANIFEST.contains("pub struct PluginManifest"));
    assert!(DEPLOY_TYPES.contains("pub enum DeployArtifact"));
    assert!(RPC_REGISTRY.contains("pub struct HandlerRegistry"));
    assert!(RPC_REGISTRY.contains("pub struct NativeHandlerPlan"));
    assert!(FORGE_NODE.contains("pub struct ForgeNode"));
}

#[test]
fn runtime_applications_doc_keeps_security_and_boundary_guidance() {
    assert!(RUNTIME_DOC.contains("[REDACTED]"));
    assert!(RUNTIME_DOC.contains("should not make every application a WASM plugin"));
    assert!(RUNTIME_DOC.contains("Do not start by rewriting CI or Forge"));
    assert!(RUNTIME_DOC.contains("Leases and fencing"));
    assert!(RUNTIME_DOC.contains("Capability escalation"));
}
