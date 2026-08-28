const CARGO_MANIFEST: &str = include_str!("../Cargo.toml");
const EXECUTION_PORTS: &str = include_str!("../src/fabric_execution/ports.rs");
const EXECUTION_LIVE_ADAPTER: &str = include_str!("../src/fabric_execution/live.rs");
const EXECUTION_SIMULATION_ADAPTER: &str = include_str!("../src/fabric_execution/simulation.rs");
const SYSTEM_EXTENSION_COMPOSITION: &str = include_str!("../src/system_extension/executionfabric.rs");
const EXECUTION_CORE_ADMISSION: &str = include_str!("../crates/molten-core/src/fabric_execution/admission/mod.rs");
const EXPECTED_REVISION: &str = "29dac88ecded94457572db3fdfaaaab95fa91525";
const EXPECTED_REPOSITORY: &str = "https://git.onix.computer/z2CpqLFpdP36fZXYUK5ZNWxMibpCo.git";

// r[verify molten.fabric_execution.component_pin]
// r[verify molten.fabric_execution.port_contract]
#[test]
fn execution_boundary_uses_one_immutable_mechanism_and_application_owned_port() {
    assert!(CARGO_MANIFEST.contains(EXPECTED_REPOSITORY));
    assert!(CARGO_MANIFEST.contains(EXPECTED_REVISION));
    assert!(!CARGO_MANIFEST.contains("bounded-exec = { path"));
    assert!(EXECUTION_PORTS.contains("pub trait ExecutionFabricPort"));
    assert!(!EXECUTION_LIVE_ADAPTER.contains("pub trait ExecutionFabricPort"));
    assert!(!EXECUTION_SIMULATION_ADAPTER.contains("pub trait ExecutionFabricPort"));
    assert!(EXECUTION_CORE_ADMISSION.contains("pub fn admit_execution_request"));
    assert!(SYSTEM_EXTENSION_COMPOSITION.contains("compose_system_extension_execution_fabric"));
}

// r[verify molten.fabric_execution.port_contract]
// r[verify molten.fabric_execution.nonclaims]
#[test]
fn execution_adapters_do_not_bypass_bounded_exec_or_hide_profile_fallback() {
    assert!(!EXECUTION_LIVE_ADAPTER.contains("std::process::Command"));
    assert!(!EXECUTION_LIVE_ADAPTER.contains("Command::new"));
    assert!(EXECUTION_LIVE_ADAPTER.contains("bounded_exec::run"));
    assert!(EXECUTION_LIVE_ADAPTER.contains("no fallback was selected"));
    assert!(EXECUTION_PORTS.contains("ExecutionPortFailureKind"));
    assert!(EXECUTION_LIVE_ADAPTER.contains("REQUIRED_EXECUTION_NON_CLAIMS"));
}
