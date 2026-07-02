#[path = "gate.rs"]
mod barrier;
#[path = "wasm/executor.rs"]
mod component;
#[path = "schema.rs"]
mod format;
#[path = "steel/executor.rs"]
mod metal;
#[path = "core.rs"]
pub mod nucleus;
#[path = "executor.rs"]
mod performer;
#[path = "replay.rs"]
mod playback;
mod runner;

mod core {
    pub(crate) use super::nucleus::*;
}
mod executor {
    pub(crate) use super::performer::*;
}
mod replay {
    pub(crate) use super::playback::*;
}
mod schema {
    pub(crate) use super::format::*;
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
pub use format::ActorDecl;
pub use format::ActorExecutorConfig;
pub use format::ActorKind;
pub use format::AdapterExecutorConfig;
pub use format::Budget;
pub use format::CapabilityGateEvidence;
pub use format::Failure;
pub use format::RemoteProxyExecutorConfig;
pub use format::ReproBundle;
pub use format::ReproBundleKind;
pub use format::ReproExportProfile;
pub use format::SteelExecutorConfig;
pub use format::Suite;
pub use format::WasmExecutorConfig;
pub use format::actor_registry_value;
pub use format::boundary_coverage_value;
pub use format::budget_gate_value;
pub use format::budget_limits_value;
pub use format::capabilities_value;
pub use format::capability_gate_value;
pub use format::deterministic_multipeer_receipt_value;
pub use format::executor_preflights_value;
pub use format::failure_repro_bundle_value;
pub use format::failure_repro_bundle_value_with_command;
pub use format::failure_summary;
pub use format::failure_value;
pub use format::golden_trace_update_receipt_value;
pub use format::parse_budget_gate;
pub use format::parse_capabilities;
pub use format::parse_capability_gate;
pub use format::parse_executor_preflights;
pub use format::parse_failure;
pub use format::parse_policy;
pub use format::parse_policy_gate;
pub use format::parse_repro_bundle;
pub use format::parse_suite;
pub use format::policy_gate_value;
pub use format::policy_value;
pub use format::report_failure_value;
pub use format::report_suite_value;
pub use format::repro_bundle_report_value;
pub use format::repro_bundle_summary;
pub use format::repro_bundle_value;
pub use format::repro_bundle_value_with_command;
pub use format::run_receipt_value;
pub use format::suite_failure_value;
pub use format::upgrade_replay_receipt_value;
pub use format::validate_deterministic_multipeer_receipt;
pub use format::validate_golden_trace_update_receipt;
pub use format::validate_harness_run_receipt;
pub use format::validate_upgrade_replay_receipt;
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
}
