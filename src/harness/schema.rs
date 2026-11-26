//! Harness suite, report, gate, preflight, receipt, and repro-bundle schemas.
//!
//! Each child module owns one responsibility; directories group them by area. Siblings reach each
//! other through `super::<module>::<item>`, and other harness modules use only the re-exports
//! below.

// Data model, suite intake, and shared Preserves field decoding.
#[path = "schema/model/decode.rs"]
mod decode;
#[path = "schema/model/fields.rs"]
mod fields;
#[path = "schema/model/intake.rs"]
mod intake;
#[path = "schema/model/model.rs"]
mod model;
#[path = "schema/model/observed.rs"]
mod observed;

// Step, event, hostcall, report, and effect-log values.
#[path = "schema/trace/calls.rs"]
mod calls;
#[path = "schema/trace/events.rs"]
mod events;
#[path = "schema/trace/handles.rs"]
mod handles;
#[path = "schema/trace/outcome.rs"]
mod outcome;
#[path = "schema/trace/reading.rs"]
mod reading;
#[path = "schema/trace/runs.rs"]
mod runs;
#[path = "schema/trace/tape.rs"]
mod tape;

// Budget and admission-policy gates with their Nickel and Basalt preflight evidence.
#[path = "schema/gates/allotment.rs"]
mod allotment;
#[path = "schema/gates/allowance.rs"]
mod allowance;
#[path = "schema/gates/covenant.rs"]
mod covenant;
#[path = "schema/gates/evaluator.rs"]
mod evaluator;
#[path = "schema/gates/rules.rs"]
mod rules;
#[path = "schema/gates/spend.rs"]
mod spend;

// Capability gate, authority contract, and UCAN proofset evidence.
#[path = "schema/grants/grants.rs"]
mod grants;
#[path = "schema/grants/proofs.rs"]
mod proofs;
#[path = "schema/grants/warrant.rs"]
mod warrant;

// Report evidence validation: admission, actor registry, runtime predicates, boundaries,
// executions.
#[path = "schema/checks/bytecode.rs"]
mod bytecode;
#[path = "schema/checks/cast.rs"]
mod cast;
#[path = "schema/checks/crossing.rs"]
mod crossing;
#[path = "schema/checks/scheme.rs"]
mod scheme;
#[path = "schema/checks/trellis.rs"]
mod trellis;
#[path = "schema/checks/vetting.rs"]
mod vetting;

// Coverage, golden-trace, upgrade-replay, run, and multipeer receipts.
#[path = "schema/receipts/attestation.rs"]
mod attestation;
#[path = "schema/receipts/blessing.rs"]
mod blessing;
#[path = "schema/receipts/migration.rs"]
mod migration;
#[path = "schema/receipts/tally.rs"]
mod tally;

// Repro bundle construction and parsing.
#[path = "schema/repro/bundling.rs"]
mod bundling;
#[path = "schema/repro/manifest.rs"]
mod manifest;
#[path = "schema/repro/sealing.rs"]
mod sealing;
#[path = "schema/repro/tailored.rs"]
mod tailored;
#[path = "schema/repro/unpack.rs"]
mod unpack;
#[path = "schema/repro/unseal.rs"]
mod unseal;

// Redaction traversal, export profiles, gates, and transform receipts.
#[path = "schema/redaction/audience.rs"]
mod audience;
#[path = "schema/redaction/disclosure.rs"]
mod disclosure;
#[path = "schema/redaction/ledgering.rs"]
mod ledgering;
#[path = "schema/redaction/masking.rs"]
mod masking;
#[path = "schema/redaction/scrub.rs"]
mod scrub;

// Actor registry and executor preflight evidence.
#[path = "schema/preflight/clearance.rs"]
mod clearance;
#[path = "schema/preflight/crosscheck.rs"]
mod crosscheck;
#[path = "schema/preflight/readiness.rs"]
mod readiness;
#[path = "schema/preflight/roster.rs"]
mod roster;
#[path = "schema/preflight/setup.rs"]
mod setup;

// Executor configuration checks, Wasm inspection, and per-kind preflight validation.
#[path = "schema/executors/introspect.rs"]
mod introspect;
#[path = "schema/executors/sources.rs"]
mod sources;
#[path = "schema/executors/variants.rs"]
mod variants;

pub use allowance::budget_limits_value;
pub use allowance::budget_value;
pub use allowance::parse_budget;
pub use attestation::deterministic_multipeer_receipt_value;
pub use attestation::run_receipt_value;
pub use attestation::validate_deterministic_multipeer_receipt;
pub use attestation::validate_harness_run_receipt;
pub use blessing::golden_trace_update_receipt_value;
pub use blessing::validate_golden_trace_update_receipt;
pub use bundling::failure_repro_bundle_value;
pub use bundling::failure_repro_bundle_value_with_command;
pub use bundling::profiled_repro_bundle_value_with_command;
pub use bundling::repro_bundle_value;
pub use bundling::repro_bundle_value_with_command;
pub use bundling::sealed_repro_bundle_value_with_command_and_receipt;
pub use calls::actor_input_value;
pub use calls::actor_output_value;
pub use calls::hostcall_decision_value;
pub use calls::hostcall_request_value;
pub use calls::validate_hostcall_effect_binding_request;
pub use cast::actor_ids_for_step;
pub use cast::validate_actor_registry_evidence;
pub use clearance::parse_executor_preflights;
pub use crosscheck::validate_executor_preflight_evidence;
pub use crossing::validate_hostcall_evidence;
pub use events::admission_decision_event_value_with_authority;
pub use events::event_value;
pub use events::observation_value;
pub use events::snapshot_value;
pub use events::step_value;
pub use grants::capabilities_value;
pub use grants::capability_gate_value;
pub use grants::parse_capabilities;
pub use grants::parse_capability_gate;
pub use grants::validate_capability_gate_evidence;
pub use intake::parse_suite;
pub use intake::suite_ref;
pub use introspect::wasm_executor_export_name;
pub use introspect::wasm_module_bytes;
pub use introspect::wasm_module_ref;
pub use migration::upgrade_replay_receipt_value;
pub use migration::validate_upgrade_replay_receipt;
pub use model::ActorDecl;
pub use model::ActorExecutorConfig;
pub use model::ActorKind;
pub use model::AdapterExecutorConfig;
pub use model::Budget;
pub use model::BudgetEvidence;
pub use model::BudgetUsage;
pub use model::Failure;
pub use model::RemoteProxyExecutorConfig;
pub use model::Report;
pub use model::ReproBundle;
pub use model::ReproBundleKind;
pub use model::ReproExportProfile;
pub use model::SteelExecutorConfig;
pub use model::Suite;
pub use model::WasmExecutorConfig;
pub use observed::CapabilityGateEvidence;
pub use observed::EffectLogEntry;
pub use observed::EventBoundary;
pub use observed::HostcallEvidenceContext;
pub use observed::Observation;
pub use outcome::ReportValueInput;
pub use outcome::failure_summary;
pub use outcome::failure_value;
pub use outcome::parse_failure;
pub use outcome::report_failure_value;
pub use outcome::report_value;
pub use outcome::suite_failure_value;
pub use proofs::admission_authority_evidence;
pub use readiness::executor_preflights_value;
pub use readiness::validate_executor_preflight_inputs;
pub use reading::parse_report;
pub use reading::report_suite_value;
pub use roster::actor_registry_value;
pub use roster::parse_actor_registry;
pub use rules::parse_policy;
pub use rules::parse_policy_gate;
pub use rules::policy_gate_value;
pub use rules::policy_value;
pub use rules::validate_policy_gate_evidence;
pub use runs::SteelExecutionReceiptInput;
pub use runs::SteelResourceReceiptInput;
pub use runs::WasmAbiReceiptInput;
pub use runs::WasmExecutionReceiptInput;
pub use runs::steel_execution_receipt_value;
pub use runs::wasm_execution_receipt_value;
pub use sources::steel_source_ref;
pub use spend::budget_gate_value;
pub use spend::parse_budget_gate;
pub use spend::validate_budget_fixture_evidence;
pub use spend::validate_budget_gate_evidence;
pub use tally::boundary_coverage_value;
pub use tape::append_effect_entries_from_events;
pub use tape::effect_log_from_observations;
#[cfg(test)]
pub use tape::effect_log_value;
pub use tape::effect_request_sequence;
pub use tape::effect_response_sequence_and_value;
pub use tape::event_boundary;
pub use trellis::validate_runtime_predicate_evidence;
pub use unpack::parse_repro_bundle;
pub use unpack::repro_bundle_report_value;
pub use unpack::repro_bundle_summary;
pub use vetting::parse_admission_decision_event;
pub use vetting::validate_admission_evidence;
