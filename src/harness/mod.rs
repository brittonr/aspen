#[path = "gate.rs"]
mod barrier;
#[path = "wasm/executor.rs"]
mod component;
pub mod composition;
#[path = "steel/executor.rs"]
mod metal;
#[path = "core.rs"]
pub mod nucleus;
#[path = "executor.rs"]
mod performer;
#[path = "replay.rs"]
mod playback;
mod runner;
mod schema;

mod core {
    pub(crate) use super::nucleus::*;
}
mod executor {
    pub(crate) use super::performer::*;
}
mod replay {
    pub(crate) use super::playback::*;
}
mod steel_executor {
    pub(crate) use super::metal::*;
}
mod wasm_executor {
    pub(crate) use super::component::*;
}

pub type HarnessStep = nucleus::CoreStep;

pub use barrier::Check;
pub use barrier::Receipt;
pub use barrier::check_summary;
pub use barrier::check_value;
pub use barrier::parse_receipt;
pub use barrier::parse_repro_verify_receipt;
pub use barrier::receipt_summary;
pub use barrier::receipt_value;
pub use barrier::repro_bundle_value_with_export_profile;
pub use barrier::repro_verify_receipt_summary;
pub use barrier::repro_verify_receipt_value;
pub use barrier::sealed_repro_bundle_value_with_command;
pub use performer::ActorExecutorDecl;
pub use performer::ActorExecutorKind;
pub use performer::actor_executor_registry;
pub use playback::ReplayOutcome;
pub use playback::ReportValidation;
pub use playback::replay_report_value;
pub use playback::report_summary;
pub use playback::validate_report_value;
pub use runner::HarnessRun;
pub use runner::run_suite;
pub use runner::run_suite_value;
pub use schema::ActorDecl;
pub use schema::ActorExecutorConfig;
pub use schema::ActorKind;
pub use schema::AdapterExecutorConfig;
pub use schema::Budget;
pub use schema::CapabilityGateEvidence;
pub use schema::Failure;
pub use schema::RemoteProxyExecutorConfig;
pub use schema::ReproBundle;
pub use schema::ReproBundleKind;
pub use schema::ReproExportProfile;
pub use schema::SteelExecutorConfig;
pub use schema::Suite;
pub use schema::WasmExecutorConfig;
pub use schema::actor_registry_value;
pub use schema::boundary_coverage_value;
pub use schema::budget_gate_value;
pub use schema::budget_limits_value;
pub use schema::capabilities_value;
pub use schema::capability_gate_value;
pub use schema::deterministic_multipeer_receipt_value;
pub use schema::executor_preflights_value;
pub use schema::failure_repro_bundle_value;
pub use schema::failure_repro_bundle_value_with_command;
pub use schema::failure_summary;
pub use schema::failure_value;
pub use schema::golden_trace_update_receipt_value;
pub use schema::parse_budget_gate;
pub use schema::parse_capabilities;
pub use schema::parse_capability_gate;
pub use schema::parse_executor_preflights;
pub use schema::parse_failure;
pub use schema::parse_policy;
pub use schema::parse_policy_gate;
pub use schema::parse_repro_bundle;
pub use schema::parse_suite;
pub use schema::policy_gate_value;
pub use schema::policy_value;
pub use schema::report_failure_value;
pub use schema::report_suite_value;
pub use schema::repro_bundle_report_value;
pub use schema::repro_bundle_summary;
pub use schema::repro_bundle_value;
pub use schema::repro_bundle_value_with_command;
pub use schema::run_receipt_value;
pub use schema::suite_failure_value;
pub use schema::upgrade_replay_receipt_value;
pub use schema::validate_deterministic_multipeer_receipt;
pub use schema::validate_golden_trace_update_receipt;
pub use schema::validate_harness_run_receipt;
pub use schema::validate_upgrade_replay_receipt;

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p002/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p003/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p004/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p005/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p006/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p007/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p008/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p009/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p010/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p011/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/harness/parts/mod/tests/m000/p012/body.rs"));
}
