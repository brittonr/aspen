pub mod core;
mod executor;
mod gate;
mod replay;
mod runner;
mod schema;
mod steel_executor;
mod wasm_executor;

pub type HarnessStep = core::CoreStep;

pub use executor::ActorExecutorDecl;
pub use executor::ActorExecutorKind;
pub use executor::actor_executor_registry;
pub use gate::GateCheck;
pub use gate::GateReceipt;
pub use gate::gate_check_summary;
pub use gate::gate_check_value;
pub use gate::gate_receipt_summary;
pub use gate::gate_receipt_value;
pub use gate::parse_gate_receipt;
pub use gate::parse_repro_verify_receipt;
pub use gate::repro_bundle_value_with_export_profile;
pub use gate::repro_verify_receipt_summary;
pub use gate::repro_verify_receipt_value;
pub use gate::sealed_repro_bundle_value_with_command;
pub use replay::ReplayOutcome;
pub use replay::ReportValidation;
pub use replay::replay_report_value;
pub use replay::report_summary;
pub use replay::validate_report_value;
pub use runner::HarnessRun;
pub use runner::run_suite;
pub use runner::run_suite_value;
pub use schema::ActorDecl;
pub use schema::ActorExecutorConfig;
pub use schema::ActorKind;
pub use schema::AdapterExecutorConfig;
pub use schema::CapabilityGateEvidence;
pub use schema::HarnessBudget;
pub use schema::HarnessFailure;
pub use schema::HarnessReproBundle;
pub use schema::HarnessReproBundleKind;
pub use schema::HarnessSuite;
pub use schema::RemoteProxyExecutorConfig;
pub use schema::ReproExportProfile;
pub use schema::SteelExecutorConfig;
pub use schema::WasmExecutorConfig;
pub use schema::actor_registry_value;
pub use schema::budget_gate_value;
pub use schema::budget_limits_value;
pub use schema::capabilities_value;
pub use schema::capability_gate_value;
pub use schema::executor_preflights_value;
pub use schema::failure_repro_bundle_value;
pub use schema::failure_repro_bundle_value_with_command;
pub use schema::failure_summary;
pub use schema::failure_value;
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
pub use schema::suite_failure_value;

#[cfg(test)]
mod tests {
    use super::ReproExportProfile;
    use super::actor_executor_registry;
    use super::core::CoreStep;
    use super::core::RuntimeState;
    use super::core::RuntimeValue;
    use super::failure_repro_bundle_value;
    use super::failure_value;
    use super::gate_check_value;
    use super::gate_receipt_summary;
    use super::gate_receipt_value;
    use super::parse_failure;
    use super::parse_gate_receipt;
    use super::parse_repro_bundle;
    use super::parse_repro_verify_receipt;
    use super::parse_suite;
    use super::replay_report_value;
    use super::repro_bundle_value;
    use super::repro_bundle_value_with_export_profile;
    use super::repro_verify_receipt_value;
    use super::run_suite_value;
    use super::runner::run_suite_with_effect_log;
    use super::schema::effect_log_from_observations;
    use super::schema::parse_report;
    use super::schema::snapshot_value;
    use super::sealed_repro_bundle_value_with_command;
    use super::suite_failure_value;
    use super::validate_report_value;
    use crate::error::MoltenError;
    use crate::preserves_rail::canonical_bytes;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    const TWO_ACTOR_SUITE: &str = r#"
<harness-suite-v1 "molten.harness.suite.v1" "two-actor" 7
  <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
  <actor-registry-v1 "molten.harness.actor-registry.v1" [
    <actor "consumer" "native">
    <actor "producer" "native">
  ]>
  <capabilities-v1 "molten.harness.capabilities.v1" [
    <grant "consumer" "observe" #f "service.ready">
    <grant "producer" "assert" #f "service.ready">
    <grant "producer" "send" "consumer" #f>
    <grant "producer" "clock" #f #f>
    <grant "producer" "random" #f #f>
    <grant "producer" "retract" #f "service.ready">
  ]>
  [
    <observe "consumer" "service.ready">
    <assert "producer" "service.ready">
    <send "producer" "consumer" "hello">
    <clock "producer">
    <random "producer" 100>
    <retract "producer" "service.ready">
  ]>
"#;

    const WASM_ASSERT_IMPORT_MODULE_HEX: &str = "0061736d01000000010401600000021a010f6d6f6c74656e3a686f737463616c6c06617373657274000003020100071a01166d6f6c74656e5f686f737463616c6c5f61737365727400010a0601040010000b";
    const WASM_ASSERT_SEND_IMPORT_MODULE_HEX: &str = "0061736d010000000104016000000231020f6d6f6c74656e3a686f737463616c6c0661737365727400000f6d6f6c74656e3a686f737463616c6c0473656e6400000303020000073102166d6f6c74656e5f686f737463616c6c5f6173736572740002146d6f6c74656e5f686f737463616c6c5f73656e6400030a0b02040010000b040010010b";
    const WASM_WASI_FD_WRITE_IMPORT_MODULE_HEX: &str =
        "0061736d0100000001040160000002230116776173695f736e617073686f745f70726576696577310866645f77726974650000";
    const WASM_ENV_READ_IMPORT_MODULE_HEX: &str = "0061736d01000000010401600000020c0103656e7604726561640000";

    fn error_contains_any(error: &MoltenError, needles: &[&str]) -> bool {
        let message = error.to_string();
        needles.iter().any(|needle| message.contains(needle))
    }

    fn wasm_abi_module_hex(operations: &[&str]) -> String {
        let output = parse_text("<wasm-output \"ok\">").expect("parse wasm output");
        let output_bytes = canonical_bytes(&output).expect("encode wasm output");
        wasm_abi_module_hex_with_output_bytes(operations, &output_bytes)
    }

    fn wasm_abi_module_hex_with_output_bytes(operations: &[&str], output_bytes: &[u8]) -> String {
        let descriptor = (2048u64 << 32) | output_bytes.len() as u64;
        wasm_abi_module_hex_with_descriptor(operations, output_bytes, descriptor)
    }

    fn wasm_abi_module_hex_with_descriptor(operations: &[&str], output_bytes: &[u8], descriptor: u64) -> String {
        let output_data = wat_data(output_bytes);
        let imports = operations
            .iter()
            .map(|operation| {
                format!(
                    r#"  (import "molten:hostcall" "{operation}" (func ${operation} (param i32 i32) (result i64)))"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let exports = operations
            .iter()
            .map(|operation| {
                format!(
                    r#"  (func (export "molten_hostcall_{operation}") (param $ptr i32) (param $len i32) (result i64)
    local.get $ptr
    local.get $len
    call ${operation}
    drop
    i64.const {descriptor})"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let wat = format!(
            r#"(module
{imports}
  (memory (export "memory") 1)
  (data (i32.const 2048) "{output_data}")
  (func (export "molten_alloc") (param $len i32) (result i32)
    i32.const 4096)
  (func (export "molten_dealloc") (param i32) (param i32))
{exports})"#
        );
        bytes_to_hex(&wat::parse_str(wat).expect("compile wat"))
    }

    fn wasm_abi_invalid_hostcall_bytes_hex() -> String {
        let output = parse_text("<wasm-output \"ok\">").expect("parse wasm output");
        let output_bytes = canonical_bytes(&output).expect("encode wasm output");
        let output_data = wat_data(&output_bytes);
        let descriptor = (2048u64 << 32) | output_bytes.len() as u64;
        let wat = format!(
            r#"(module
  (import "molten:hostcall" "assert" (func $assert (param i32 i32) (result i64)))
  (memory (export "memory") 1)
  (data (i32.const 1024) "\ff")
  (data (i32.const 2048) "{output_data}")
  (func (export "molten_alloc") (param $len i32) (result i32)
    i32.const 4096)
  (func (export "molten_dealloc") (param i32) (param i32))
  (func (export "molten_hostcall_assert") (param $ptr i32) (param $len i32) (result i64)
    i32.const 1024
    i32.const 1
    call $assert
    drop
    i64.const {descriptor}))"#
        );
        bytes_to_hex(&wat::parse_str(wat).expect("compile invalid hostcall wat"))
    }

    fn wasm_abi_fuel_exhaustion_hex() -> String {
        let wat = r#"(module
  (import "molten:hostcall" "assert" (func $assert (param i32 i32) (result i64)))
  (memory (export "memory") 1)
  (func (export "molten_alloc") (param $len i32) (result i32)
    i32.const 4096)
  (func (export "molten_dealloc") (param i32) (param i32))
  (func (export "molten_hostcall_assert") (param $ptr i32) (param $len i32) (result i64)
    (loop $again
      br $again)
    i64.const 0))"#;
        bytes_to_hex(&wat::parse_str(wat).expect("compile fuel wat"))
    }

    fn wasm_abi_missing_memory_hex() -> String {
        let wat = r#"(module
  (import "molten:hostcall" "assert" (func $assert (param i32 i32) (result i64)))
  (func (export "molten_alloc") (param $len i32) (result i32)
    i32.const 0)
  (func (export "molten_dealloc") (param i32) (param i32))
  (func (export "molten_hostcall_assert") (param $ptr i32) (param $len i32) (result i64)
    local.get $ptr
    local.get $len
    call $assert
    drop
    i64.const 0))"#;
        bytes_to_hex(&wat::parse_str(wat).expect("compile missing-memory wat"))
    }

    fn wat_data(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("\\{byte:02x}")).collect::<String>()
    }

    fn bytes_to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>()
    }

    const OLD_SHAPE_TWO_ACTOR_SUITE: &str = r#"
<harness-suite-v1 "molten.harness.suite.v1" "two-actor-old-shape" 7
  <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
  <actor-registry-v1 "molten.harness.actor-registry.v1" [
    <actor "consumer" "native">
    <actor "producer" "native">
  ]>
  <capabilities-v1 "molten.harness.capabilities.v1" [
    <grant "consumer" "observe" #f "service.ready">
    <grant "producer" "assert" #f "service.ready">
    <grant "producer" "clock" #f #f>
  ]>
  [
    <observe "consumer" "service.ready">
    <assert "producer" "service.ready">
    <clock "producer">
  ]>
"#;

    fn conformance_suite_for_subject(actor_entry: &str) -> String {
        format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "executor-conformance" 11
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                {actor_entry}
                <actor "sink" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "subject" "assert" #f #f>
                <grant "subject" "send" "sink" #f>
              ]>
              [
                <assert "subject" <service-ready "db" 1>>
                <send "subject" "sink" <payload <service-ready "db" 1> [1 2 3]>>
              ]>"#,
        )
    }

    fn cross_kind_conformance_suite() -> String {
        format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "executor-conformance-cross-kind" 11
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "native" "native">
                <actor "steel" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["assert" "send"]>>>
                <actor "wasm" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{WASM_ASSERT_SEND_IMPORT_MODULE_HEX}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert" "send"]>>>
                <actor "sink" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "native" "assert" #f #f>
                <grant "native" "send" "sink" #f>
                <grant "steel" "assert" #f #f>
                <grant "steel" "send" "sink" #f>
                <grant "wasm" "assert" #f #f>
                <grant "wasm" "send" "sink" #f>
              ]>
              [
                <assert "native" <service-ready "db" 1>>
                <send "native" "sink" <payload <service-ready "db" 1> [1 2 3]>>
                <assert "steel" <service-ready "db" 1>>
                <send "steel" "sink" <payload <service-ready "db" 1> [1 2 3]>>
                <assert "wasm" <service-ready "db" 1>>
                <send "wasm" "sink" <payload <service-ready "db" 1> [1 2 3]>>
              ]>"#,
        )
    }

    #[test]
    fn two_actor_suite_replays_identically() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let replay = replay_report_value(&run.report_value).expect("replay report");
        assert_eq!(run.report_ref, replay.expected_report_ref);
        assert_eq!(run.report_ref, replay.actual_report_ref);
        assert_eq!(run.final_state_hash, replay.final_state_hash);
        let validation = validate_report_value(&run.report_value).expect("validate report");
        assert_eq!(validation.report_ref, run.report_ref);
        assert_eq!(validation.observations, 6);
        let parsed_suite = parse_suite(&suite).expect("parse suite structure");
        assert_eq!(parsed_suite.actors.len(), 2);
        assert!(parsed_suite.actors.iter().all(|actor| actor.kind == super::ActorKind::Native));
    }

    #[test]
    fn executor_hostcall_boundary_evidence_is_recorded_and_gated() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("executor-preflights-v1"));
        assert!(report_text.contains("executor-preflight-v1"));
        assert!(report_text.contains("allowed-hostcalls"));
        assert!(report_text.contains("actor-input-v1"));
        assert!(report_text.contains("hostcall-request-v1"));
        assert!(report_text.contains("handler-binding-ref"));
        assert!(report_text.contains("handle-ref"));
        assert!(report_text.contains("effect-manifest-ref"));
        assert!(report_text.contains("handler-profile-ref"));
        assert!(report_text.contains("effect-request-ref"));
        assert!(report_text.contains("effect-binding-receipt-ref"));
        assert!(report_text.contains("hostcall-decision-v1"));
        assert!(report_text.contains("actor-output-v1"));
        let gate = gate_check_value(&run.report_value).expect("gate report");
        let gate_receipt = gate_receipt_value(&gate);
        let receipt_text = to_text(&gate_receipt).expect("render gate receipt");
        assert!(receipt_text.contains("deterministic-replay-verify-v1"));
        assert!(receipt_text.contains("deterministic-replay-verify"));
        assert!(receipt_text.contains("no-divergence"));
        let receipt = parse_gate_receipt(&gate_receipt).expect("parse gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "executor-kind-binding"));
        assert!(receipt.checks.iter().any(|check| check == "allowed-hostcall-binding"));
        assert!(receipt.checks.iter().any(|check| check == "no-unsupported-executor-fallback"));
        assert!(receipt.checks.iter().any(|check| check == "executor-conformance-suite-binding"));
        assert!(receipt.checks.iter().any(|check| check == "cross-kind-hostcall-conformance"));
        assert!(receipt.checks.iter().any(|check| check == "executor-execution-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "executor-output-ref-binding"));
        assert!(receipt.checks.iter().any(|check| check == "steel-executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "steel-review-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "steel-vm-execution"));
        assert!(receipt.checks.iter().any(|check| check == "steel-resource-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "adapter-executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "remote-proxy-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-executor-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-inspection-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-execution-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "wasmtime-no-wasi"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-fuel-memory-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-abi-byte-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-guest-memory-bounds"));
        assert!(receipt.checks.iter().any(|check| check == "wasm-preserves-abi-ready"));
        assert!(receipt.checks.iter().any(|check| check == "executor-hostcall-boundary"));
        assert!(receipt.checks.iter().any(|check| check == "hostcall-admission-binding"));
        assert!(receipt.checks.iter().any(|check| check == "hostcall-replay"));
        assert!(receipt.checks.iter().any(|check| check == "effect-handler-binding"));
        assert!(receipt.checks.iter().any(|check| check == "effect-handle-binding"));
        assert!(receipt.checks.iter().any(|check| check == "handle-not-authority"));
        assert!(receipt.checks.iter().any(|check| check == "hostcall-handle-replay"));
        assert!(receipt.checks.iter().any(|check| check == "no-ambient-executor-io"));
    }

    #[test]
    fn hostcall_effect_handles_disambiguate_same_kind_requests() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let mut handle_refs = std::collections::BTreeSet::new();
        for observation in &report.observations {
            let request = observation.events[2]
                .collect_simple_record("hostcall-request-v1", None)
                .expect("hostcall request carries handle refs");
            let handle_ref = request[10].collect_simple_record("handle-ref", Some(1)).expect("handle-ref record")[0]
                .as_string()
                .expect("handle-ref string")
                .into_owned();
            crate::preserves_rail::validate_content_ref(&handle_ref).expect("handle ref is canonical");
            handle_refs.insert(handle_ref);
        }
        assert_eq!(handle_refs.len(), report.observations.len());
    }

    #[test]
    fn report_validation_rejects_tampered_hostcall_handle_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("handle-ref", "handle-ref-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered handle report");
        let error = validate_report_value(&tampered_report).expect_err("tampered handle ref fails validation");
        assert!(error.to_string().contains("hostcall-request evidence mismatch"), "{error}");
        let replay_error = replay_report_value(&tampered_report).expect_err("tampered handle ref diverges");
        assert!(replay_error.to_string().contains("hostcall-request"), "{replay_error}");
    }

    #[test]
    fn gate_receipt_requires_executor_resource_checks() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let gate = gate_check_value(&run.report_value).expect("gate report");
        let receipt_value = gate_receipt_value(&gate);
        let receipt_text = to_text(&receipt_value).expect("render receipt");
        assert!(receipt_text.contains("executor-execution-receipts"));
        let tampered_text = receipt_text.replacen("executor-execution-receipt-binding", "executor-execution-stale", 1);
        let tampered = parse_text(&tampered_text).expect("parse tampered gate receipt");
        let error = parse_gate_receipt(&tampered).expect_err("missing executor execution binding check fails");
        assert!(error.to_string().contains("executor-execution-receipt-binding"), "{error}");

        let tampered_replay_text =
            receipt_text.replacen("<divergence \"none\">", "<divergence \"effect-response\">", 1);
        let tampered_replay = parse_text(&tampered_replay_text).expect("parse tampered replay receipt");
        let error = parse_gate_receipt(&tampered_replay).expect_err("tampered generic replay receipt fails");
        assert!(error.to_string().contains("replay verify ref"), "{error}");
    }

    #[test]
    fn gate_receipt_binds_chain_continuity_anchor_and_checkpoint_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let gate = gate_check_value(&run.report_value).expect("gate report");
        let receipt_value = gate_receipt_value(&gate);
        let receipt_text = to_text(&receipt_value).expect("render receipt");
        assert!(receipt_text.contains("chain-evidence"));
        assert!(receipt_text.contains("chain-link-v1"));
        assert!(receipt_text.contains("chain-verify-receipt-v1"));
        assert!(receipt_text.contains("chain-checkpoint-v1"));
        let receipt = parse_gate_receipt(&receipt_value).expect("parse chained gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "chain-continuity"));
        assert!(receipt.checks.iter().any(|check| check == "chain-anchor-descent"));
        assert!(receipt.checks.iter().any(|check| check == "chain-checkpoint-freshness"));
        assert!(receipt.checks.iter().any(|check| check == "chain-predicate-receipts"));

        let missing_check = parse_text(&receipt_text.replacen("chain-continuity", "chain-stale", 1))
            .expect("parse missing chain check receipt");
        let error = parse_gate_receipt(&missing_check).expect_err("missing chain continuity check fails");
        assert!(error.to_string().contains("chain-continuity"), "{error}");

        let tampered_predicate = parse_text(&receipt_text.replacen(
            "molten.chain.checkpoint_covers_range.v1",
            "molten.chain.segment_no_gap.v1",
            1,
        ))
        .expect("parse tampered range predicate receipt");
        let error = parse_gate_receipt(&tampered_predicate).expect_err("tampered range predicate fails");
        assert!(error_contains_any(&error, &["range predicate", "checkpoint"]), "{error}");
    }

    #[test]
    fn gate_receipt_binds_actor_scoped_turn_journals() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let gate = gate_check_value(&run.report_value).expect("gate report");
        let receipt_value = gate_receipt_value(&gate);
        let receipt_text = to_text(&receipt_value).expect("render receipt");
        assert!(receipt_text.contains("turn-journals"));
        assert!(receipt_text.contains("turn-journal"));
        assert!(receipt_text.contains("harness-turn-journal"));
        let receipt = parse_gate_receipt(&receipt_value).expect("parse turn journal gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-chains"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-input-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-admission-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-state-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-no-global-head"));

        let global_scope = parse_text(&receipt_text.replacen("harness-turn-journal", "harness-global-journal", 1))
            .expect("parse global turn journal tamper");
        let error = parse_gate_receipt(&global_scope).expect_err("global turn journal scope fails");
        assert!(error_contains_any(&error, &["not global", "turn journal"]), "{error}");

        let start = receipt_text.find("turn-journals").expect("turn journals text");
        let admission = start + receipt_text[start..].find("\"admission\"").expect("admission context");
        let missing_admission = format!(
            "{}\"missing-admission\"{}",
            &receipt_text[..admission],
            &receipt_text[admission + "\"admission\"".len()..]
        );
        let missing_admission = parse_text(&missing_admission).expect("parse missing admission tamper");
        let error = parse_gate_receipt(&missing_admission).expect_err("missing admission context fails");
        assert!(error.to_string().contains("admission"), "{error}");
    }

    #[test]
    fn report_validation_rejects_tampered_executor_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.find("executor-preflight-v1").expect("executor preflight");
        let relative = report_text[start..].find("allowed-hostcalls").expect("allowed hostcalls");
        let absolute = start + relative;
        let tampered_text = format!(
            "{}tampered-hostcalls{}",
            &report_text[..absolute],
            &report_text[absolute + "allowed-hostcalls".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered executor preflight report");
        let error = validate_report_value(&tampered_report).expect_err("tampered preflight fails validation");
        assert!(error_contains_any(&error, &["executor preflight", "allowed-hostcalls"]));
    }

    #[test]
    fn report_validation_rejects_tampered_hostcall_decision() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.find("hostcall-decision-v1").expect("hostcall decision");
        let relative = report_text[start..].find("admission-binding").expect("hostcall admission binding check");
        let absolute = start + relative;
        let tampered_text = format!(
            "{}tampered-binding{}",
            &report_text[..absolute],
            &report_text[absolute + "admission-binding".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered hostcall report");
        let error = validate_report_value(&tampered_report).expect_err("tampered hostcall decision fails validation");
        assert!(error.to_string().contains("hostcall-decision evidence mismatch"));
        let replay_error = replay_report_value(&tampered_report).expect_err("tampered hostcall decision diverges");
        assert!(replay_error.to_string().contains("hostcall-decision"));
    }

    #[test]
    fn suite_steps_accept_canonical_preserves_payloads() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "preserves-payloads" 9
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f #f>
                <grant "producer" "assert" #f #f>
                <grant "producer" "send" "consumer" #f>
              ]>
              [
                <observe "consumer" <service-ready "db" 1>>
                <assert "producer" <service-ready "db" 1>>
                <send "producer" "consumer" <payload [1 2 3] <ok>>>
              ]>"#,
        )
        .expect("parse Preserves-valued suite");
        let run = run_suite_value(&suite).expect("run Preserves-valued suite");
        replay_report_value(&run.report_value).expect("replay Preserves-valued suite");
        validate_report_value(&run.report_value).expect("validate Preserves-valued report");
        let report_text = to_text(&run.report_value).expect("render Preserves-valued report");
        assert!(report_text.contains("<service-ready \"db\" 1>"));
        assert!(report_text.contains("<payload [1, 2, 3] <ok>>"));
    }

    #[test]
    fn policy_denied_assert_rolls_back_without_committing_assertion() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-assert" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f "service.ready">
                <grant "producer" "assert" #f "service.ready">
              ]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">
              ]>
              [<observe "consumer" "service.ready"> <assert "producer" "service.ready">]>"#,
        )
        .expect("parse deny assert suite");
        let run = run_suite_value(&suite).expect("policy denial is recorded evidence, not ambient failure");
        replay_report_value(&run.report_value).expect("replay deny assert suite");
        let report_text = to_text(&run.report_value).expect("render deny assert report");
        assert!(report_text.contains("admission-decision-v1"));
        assert!(report_text.contains("<decision \"deny\" \"producer cannot assert readiness\">"));
        assert!(report_text.contains("<turn-rolled-back \"producer\" \"producer cannot assert readiness\">"));
        assert!(!report_text.contains("assertion-committed"));
        assert!(!report_text.contains("assertion-observed"));
    }

    #[test]
    fn capability_missing_send_grant_denies_delivery() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "capability-deny-send" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" []>
              [<send "producer" "consumer" "hello">]>"#,
        )
        .expect("parse missing send grant suite");
        let run = run_suite_value(&suite).expect("missing grant is recorded denial evidence");
        validate_report_value(&run.report_value).expect("validate missing grant denial");
        replay_report_value(&run.report_value).expect("replay missing grant denial");
        let report_text = to_text(&run.report_value).expect("render missing grant report");
        assert!(report_text.contains("missing capability grant"));
        assert!(report_text.contains("<authorized #f>"));
        assert!(!report_text.contains("message-delivered"));
    }

    #[test]
    fn time_random_effects_use_deterministic_local_handler_receipts() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        validate_report_value(&run.report_value).expect("validate report");
        replay_report_value(&run.report_value).expect("replay report");
        let report = parse_report(&run.report_value).expect("parse report");
        let report_text = to_text(&run.report_value).expect("render report");
        assert_eq!(report.replay_status, "deterministic");
        assert_eq!(report.profile, "local-deterministic");
        assert!(report_text.contains("time-random-handler-receipt-v1"));
        assert!(report_text.contains("local-deterministic"));
        assert!(report_text.contains("deny-by-default-bypassed-only-by-local-test-handler"));
    }

    #[test]
    fn report_binds_deterministic_handler_profile_identity() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let replay = replay_report_value(&run.report_value).expect("replay report");

        assert_eq!(report.profile, "local-deterministic");
        assert_eq!(report.replay_status, "deterministic");
        assert_eq!(replay.expected_report_ref, run.report_ref);
        assert_eq!(replay.actual_report_ref, run.report_ref);
    }

    #[test]
    fn turn_journal_refs_are_stable_under_replay() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let replay = replay_report_value(&run.report_value).expect("replay report");
        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed_run = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay run");
        let replay_report = parse_report(&replayed_run.report_value).expect("parse replay report");
        let journal_refs = turn_journal_refs(&report);
        let replay_journal_refs = turn_journal_refs(&replay_report);
        let rendered = to_text(&run.report_value).expect("render report");

        assert_eq!(replay.expected_report_ref, replay.actual_report_ref);
        assert_eq!(journal_refs.len(), report.observations.len());
        assert_eq!(journal_refs, replay_journal_refs);
        assert!(rendered.contains("turn-journal-v1"));
        assert!(rendered.contains("scheduler-key"));
        assert!(rendered.contains("effect-refs"));
        assert!(rendered.contains("receipt-refs"));
    }

    fn turn_journal_refs(report: &super::schema::HarnessReport) -> Vec<String> {
        report
            .observations
            .iter()
            .flat_map(|observation| observation.events.iter())
            .filter(|event| event.collect_simple_record("turn-journal-v1", None).is_some())
            .map(|event| canonical_hash(event).expect("journal ref"))
            .collect()
    }

    #[test]
    fn capability_missing_effect_grant_suppresses_effect_request() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "capability-deny-clock" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" []>
              [<clock "producer">]>"#,
        )
        .expect("parse missing clock grant suite");
        let run = run_suite_value(&suite).expect("missing effect grant is recorded denial evidence");
        validate_report_value(&run.report_value).expect("validate missing effect grant denial");
        let report_text = to_text(&run.report_value).expect("render missing effect grant report");
        assert!(report_text.contains("missing capability grant"));
        assert!(!report_text.contains("effect-request"));
        assert!(!report_text.contains("effect-response"));
    }

    #[test]
    fn capability_grant_allows_authorized_send() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "capability-allow-send" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "send" "consumer" #f>
              ]>
              [<send "producer" "consumer" "hello">]>"#,
        )
        .expect("parse send grant suite");
        let run = run_suite_value(&suite).expect("authorized send runs");
        validate_report_value(&run.report_value).expect("validate authorized send");
        let report_text = to_text(&run.report_value).expect("render authorized send report");
        assert!(report_text.contains("<authorized #t>"));
        assert!(report_text.contains("message-delivered"));
    }

    #[test]
    fn policy_denied_send_leaves_no_message_and_denied_clock_has_no_effect_response() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-send-and-clock" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "send" "consumer" #f>
                <grant "producer" "clock" #f #f>
              ]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "send" "consumer" #f "producer cannot send to consumer">
                <deny "producer" "clock" #f #f "producer cannot read clock">
              ]>
              [<send "producer" "consumer" "hello"> <clock "producer">]>"#,
        )
        .expect("parse deny send/clock suite");
        let run = run_suite_value(&suite).expect("policy denial report");
        replay_report_value(&run.report_value).expect("replay deny send/clock suite");
        let report_text = to_text(&run.report_value).expect("render deny send/clock report");
        assert!(report_text.contains("producer cannot send to consumer"));
        assert!(report_text.contains("producer cannot read clock"));
        assert!(!report_text.contains("message-delivered"));
        assert!(!report_text.contains("effect-request"));
        assert!(!report_text.contains("effect-response"));
    }

    #[test]
    fn tampered_policy_decision_reports_policy_divergence_with_failure_diagnostics() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "policy-divergence" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "clock" #f #f>]>
              [<clock "producer">]>"#,
        )
        .expect("parse policy divergence suite");
        let run = run_suite_value(&suite).expect("run policy divergence suite");
        let report_text = to_text(&run.report_value).expect("render policy divergence report");
        let tampered_text = report_text.replacen("admission-decision-v1", "admission-decision-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered policy decision report");
        let error = replay_report_value(&tampered_report).expect_err("tampered policy decision diverges");
        match &error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "policy-decision");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected policy-decision divergence, got {other}"),
        }
        let failure = failure_value("replay", &error, Vec::new());
        let parsed_failure = parse_failure(&failure).expect("parse policy divergence failure");
        assert_eq!(parsed_failure.kind, "policy-decision");
        let diagnostics = parsed_failure
            .diagnostics
            .iter()
            .map(|diagnostic| to_text(diagnostic).expect("render diagnostic"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(diagnostics.contains("<step 0>"));
        assert!(diagnostics.contains("<detail \"event differs\">"));
    }

    #[test]
    fn report_validation_requires_admission_decision_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("admission-decision-v1", "admission-decision-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing admission report");
        let error = validate_report_value(&tampered_report).expect_err("missing admission evidence fails closed");
        assert!(error.to_string().contains("missing admission decision"));
    }

    #[test]
    fn report_validation_recomputes_admission_decision() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-assert-validation" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "assert" #f "service.ready">]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse deny assert validation suite");
        let run = run_suite_value(&suite).expect("run deny assert validation suite");
        let report_text = to_text(&run.report_value).expect("render deny assert validation report");
        let tampered_text = report_text.replacen(
            "<decision \"deny\" \"producer cannot assert readiness\">",
            "<decision \"allow\" \"default-allow\">",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered decision report");
        let error = validate_report_value(&tampered_report).expect_err("tampered decision fails validation");
        assert!(error.to_string().contains("admission decision mismatch"));
    }

    #[test]
    fn report_validation_requires_policy_gate_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("policy-gate-v1"));
        let tampered_text = report_text.replacen("policy-gate-v1", "policy-gate-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing policy gate report");
        let error = validate_report_value(&tampered_report).expect_err("missing policy preflight fails closed");
        assert!(error_contains_any(&error, &["policy gate", "actor-registry"]));
    }

    #[test]
    fn report_validation_rejects_tampered_policy_gate_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let marker = "<policy-ref \"";
        let start = report_text.find(marker).expect("policy ref marker") + marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered policy gate report");
        let error = validate_report_value(&tampered_report).expect_err("tampered policy gate ref fails validation");
        assert!(error_contains_any(&error, &[
            "policy gate ref mismatch",
            "Nickel source policy ref does not match policy gate ref",
        ]));
    }

    #[test]
    fn report_validation_requires_basalt_policy_preflight_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("basalt-preflight"));
        let tampered_text = report_text.replacen("basalt-preflight", "basalt-context", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing Basalt preflight report");
        let error = validate_report_value(&tampered_report).expect_err("missing Basalt preflight fails closed");
        assert!(error.to_string().contains("basalt-preflight"));
    }

    #[test]
    fn report_validation_rejects_tampered_basalt_policy_preflight_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("<reason \"accepted\">", "<reason \"tampered\">", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Basalt preflight report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Basalt preflight fails closed");
        assert!(error.to_string().contains("unsupported Basalt policy preflight reason tampered"));
    }

    #[test]
    fn report_validation_rejects_tampered_nickel_policy_export() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("export-json"));
        let tampered_text = report_text.replacen(
            "molten.harness.policy.nickel-static.v1",
            "molten.harness.policy.nickel-static.tampered",
            2,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Nickel export report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Nickel export fails closed");
        assert!(error_contains_any(&error, &[
            "Nickel policy export JSON does not match source normalization",
            "Nickel policy export ref mismatch",
            "unsupported Nickel source schema",
        ]));
    }

    #[test]
    fn report_validation_requires_capability_gate_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("capability-gate-v1"));
        let tampered_text = report_text.replacen("capability-gate-v1", "capability-gate-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing capability gate report");
        let error = validate_report_value(&tampered_report).expect_err("missing capability preflight fails closed");
        assert!(error_contains_any(&error, &["capability gate", "actor-registry"]));
    }

    #[test]
    fn report_validation_rejects_tampered_capability_gate_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let marker = "<capability-ref \"";
        let start = report_text.find(marker).expect("capability ref marker") + marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered capability gate report");
        let error = validate_report_value(&tampered_report).expect_err("tampered capability gate ref fails validation");
        assert!(error_contains_any(&error, &[
            "capability gate ref mismatch",
            "authority contract normalized capability ref does not match capability gate ref",
            "Basalt authority preflight capability ref does not match capability gate ref",
        ]));
    }

    #[test]
    fn report_validation_requires_basalt_authority_preflight_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("basalt-authority-preflight"));
        let tampered_text = report_text.replacen("basalt-authority-preflight", "basalt-authority-context", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing Basalt authority report");
        let error =
            validate_report_value(&tampered_report).expect_err("missing Basalt authority preflight fails closed");
        assert!(error.to_string().contains("basalt-authority-preflight"));
    }

    #[test]
    fn report_validation_rejects_tampered_basalt_authority_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let capability_gate_start = report_text.find("capability-gate-v1").expect("capability gate");
        let reason_relative =
            report_text[capability_gate_start..].find("<reason \"accepted\">").expect("authority reason");
        let start = capability_gate_start + reason_relative;
        let tampered_text = format!(
            "{}<reason \"tampered\">{}",
            &report_text[..start],
            &report_text[start + "<reason \"accepted\">".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Basalt authority report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Basalt authority fails closed");
        assert!(error.to_string().contains("unsupported Basalt authority preflight reason tampered"));
    }

    #[test]
    fn report_validation_rejects_non_empty_ucan_proofset_until_supported() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let empty_proofset = "<ucan-proofset-v1 \"molten.harness.ucan-proofset.v1\" []>";
        assert!(report_text.contains(empty_proofset));
        let tampered_text = report_text.replacen(
            empty_proofset,
            "<ucan-proofset-v1 \"molten.harness.ucan-proofset.v1\" [<proof \"unchecked\">]>",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse non-empty UCAN proofset report");
        let error = validate_report_value(&tampered_report).expect_err("non-empty UCAN proofset fails closed");
        assert!(error.to_string().contains("UCAN proof refs require Basalt/UCAN proof validation"));
    }

    #[test]
    fn report_validation_rejects_tampered_capability_grant_ref_binding() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let grant_refs_start = report_text.find("<grant-refs [").expect("grant refs");
        let hash_relative = report_text[grant_refs_start..].find("blake3:").expect("first grant ref");
        let start = grant_refs_start + hash_relative;
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered grant ref report");
        let error = validate_report_value(&tampered_report).expect_err("tampered grant ref fails closed");
        assert!(error_contains_any(&error, &[
            "capability gate grant refs do not match embedded capabilities",
            "capability gate evidence does not match embedded authority preflight",
        ]));
    }

    #[test]
    fn report_validation_rejects_tampered_admission_authority() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let gate_ref_marker = "<capability-ref \"";
        let admission_start = report_text.find("admission-decision-v1").expect("admission decision");
        let authority_relative =
            report_text[admission_start..].find(gate_ref_marker).expect("admission capability ref");
        let start = admission_start + authority_relative + gate_ref_marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered authority report");
        let error = validate_report_value(&tampered_report).expect_err("tampered authority binding fails validation");
        assert!(error.to_string().contains("capability authority mismatch"));
        let error = replay_report_value(&tampered_report).expect_err("tampered authority binding diverges on replay");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "capability-decision");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected capability-decision divergence, got {other}"),
        }
    }

    #[test]
    fn unreviewed_steel_predicate_policy_fails_preflight() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-predicate-denied" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <policy-v1 "molten.harness.policy.v1" [
                <steel-predicate "unchecked-dynamic-callable">
              ]>
              [<clock "producer">]>"#,
        )
        .expect("parse suite with unreviewed predicate record");
        let error = parse_suite(&suite).expect_err("unreviewed Steel predicate cannot enter local policy fixture");
        assert!(error.to_string().contains("Steel predicates require reviewed callable receipts"));
    }

    #[test]
    fn report_validation_rejects_committed_action_after_denial() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-commit-validation" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "assert" #f "service.ready">]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "assert" #f "service.ready" "producer cannot assert readiness">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse deny commit validation suite");
        let run = run_suite_value(&suite).expect("run deny commit validation suite");
        let report_text = to_text(&run.report_value).expect("render deny commit validation report");
        let tampered_text = report_text.replacen(
            "<turn-rolled-back \"producer\" \"producer cannot assert readiness\">",
            "<turn-rolled-back \"producer\" \"producer cannot assert readiness\">, <assertion-committed \"producer\" \"service.ready\">",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse denied commit report");
        let error = validate_report_value(&tampered_report).expect_err("denied commit fails validation");
        assert!(error.to_string().contains("denied turn committed action"));
    }

    #[test]
    fn report_validation_rejects_effect_response_after_denial() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "deny-effect-validation" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "producer" "clock" #f #f>]>
              <policy-v1 "molten.harness.policy.v1" [
                <deny "producer" "clock" #f #f "producer cannot read clock">
              ]>
              [<clock "producer">]>"#,
        )
        .expect("parse deny effect validation suite");
        let run = run_suite_value(&suite).expect("run deny effect validation suite");
        let report_text = to_text(&run.report_value).expect("render deny effect validation report");
        let tampered_text = report_text.replacen(
            "<turn-rolled-back \"producer\" \"producer cannot read clock\">",
            "<turn-rolled-back \"producer\" \"producer cannot read clock\">, <effect-request \"clock\" \"producer\" 0>, <effect-response \"clock\" \"producer\" 0 0>",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse denied effect report");
        let error = validate_report_value(&tampered_report).expect_err("denied effect fails validation");
        assert!(error.to_string().contains("denied effect emitted effect request/response"));
    }

    #[test]
    fn unknown_actor_in_step_fails() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "unknown-actor" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              [<send "producer" "missing" "hello">]>"#,
        )
        .expect("parse unknown actor suite");
        let error = run_suite_value(&suite).expect_err("unknown actor should fail");
        assert!(error.to_string().contains("unknown actor missing"));
    }

    #[test]
    fn canonical_failure_artifact_records_unknown_actor() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "unknown-actor" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              [<send "producer" "missing" "hello">]>"#,
        )
        .expect("parse unknown actor suite");
        let error = run_suite_value(&suite).expect_err("unknown actor should fail");
        let failure_value = suite_failure_value("preflight", &error, &suite).expect("failure value");
        let failure = parse_failure(&failure_value).expect("parse failure artifact");
        assert_eq!(failure.phase, "preflight");
        assert_eq!(failure.kind, "invalid-harness");
        assert!(failure.message.contains("unknown actor missing"));
        assert!(failure.diagnostics.iter().any(|diagnostic| {
            to_text(diagnostic)
                .expect("render diagnostic")
                .contains(&canonical_hash(&suite).expect("suite hash"))
        }));
        let rendered = to_text(&failure_value).expect("render failure");
        let reparsed = parse_text(&rendered).expect("reparse failure");
        assert_eq!(canonical_hash(&failure_value).unwrap(), canonical_hash(&reparsed).unwrap());
    }

    #[test]
    fn canonical_failure_artifact_records_resource_divergence() {
        let steps = (0..17).map(|_| "<clock \"a\">".to_string()).collect::<Vec<_>>().join(" ");
        let suite = parse_text(&format!(
            "<harness-suite-v1 \"molten.harness.suite.v1\" \"too-many-effects\" 1 \
             <budget-v1 \"molten.harness.budget.v1\" <limits 64 16 256 65536>> \
             <actor-registry-v1 \"molten.harness.actor-registry.v1\" [<actor \"a\" \"native\">]> \
             <capabilities-v1 \"molten.harness.capabilities.v1\" [<grant \"a\" \"clock\" #f #f>]> [{steps}]>"
        ))
        .expect("parse suite");
        let error = run_suite_value(&suite).expect_err("effect budget should fail");
        let failure_value = suite_failure_value("execute", &error, &suite).expect("failure value");
        let failure = parse_failure(&failure_value).expect("parse failure artifact");
        assert_eq!(failure.phase, "execute");
        assert_eq!(failure.kind, "resource");
        assert!(failure.message.contains("effect count exceeds budget"));
        let rendered = failure
            .diagnostics
            .iter()
            .map(|diagnostic| to_text(diagnostic).expect("render diagnostic"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("<step 16>"));
        assert!(rendered.contains("<expected \"16\">"));
        assert!(rendered.contains("<actual \"17\">"));
    }

    #[test]
    fn canonical_failure_artifact_is_not_a_passing_report() {
        let error = MoltenError::invalid_harness("synthetic preflight failure");
        let failure = failure_value("preflight", &error, Vec::new());
        let report_error = validate_report_value(&failure).expect_err("failure artifacts are diagnostic evidence only");
        assert!(report_error.to_string().contains("expected <harness-report-v1"));
    }

    #[test]
    fn gate_accepts_report_and_report_repro_bundle() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_check = gate_check_value(&run.report_value).expect("gate accepts report");
        assert_eq!(report_check.artifact_kind, "report");
        assert_eq!(report_check.report_ref, run.report_ref);

        let bundle = repro_bundle_value(&run.report_value).expect("bundle report");
        let parsed_bundle = parse_repro_bundle(&bundle).expect("parse report bundle");
        assert_eq!(parsed_bundle.kind, super::HarnessReproBundleKind::Report);
        let unsealed_error = gate_check_value(&bundle).expect_err("unsealed report bundle lacks redaction preflight");
        assert!(error_contains_any(&unsealed_error, &["redaction preflight", "gate receipt"]));

        let sealed_bundle =
            sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
                .expect("sealed bundle report");
        let parsed_sealed = parse_repro_bundle(&sealed_bundle).expect("parse sealed bundle");
        assert_eq!(parsed_sealed.kind, super::HarnessReproBundleKind::Report);
        assert!(parsed_sealed.gate_receipt_ref.is_some());
        assert!(parsed_sealed.redaction_policy_ref.is_some());
        assert!(parsed_sealed.redaction_gate_ref.is_some());
        let embedded_receipt =
            parse_gate_receipt(parsed_sealed.gate_receipt_value.as_ref().expect("sealed bundle embeds gate receipt"))
                .expect("parse embedded gate receipt");
        assert_eq!(embedded_receipt.artifact_kind, "report");
        assert_eq!(embedded_receipt.report_ref, run.report_ref);
        let sealed_check = gate_check_value(&sealed_bundle).expect("gate accepts sealed report bundle");
        assert_eq!(sealed_check.artifact_kind, "repro-bundle");
        assert_eq!(sealed_check.report_ref, run.report_ref);
        let verify_receipt_value = repro_verify_receipt_value(&sealed_bundle).expect("verify sealed report bundle");
        let verify_receipt = parse_repro_verify_receipt(&verify_receipt_value).expect("parse repro verify receipt");
        assert_eq!(verify_receipt.decision, "pass");
        assert_eq!(verify_receipt.bundle_ref, canonical_hash(&sealed_bundle).expect("sealed bundle hash"));
        assert_eq!(verify_receipt.report_ref, run.report_ref);
    }

    #[test]
    fn sealed_repro_bundle_export_rejects_sensitive_markers() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "secret-payload" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse secret suite");
        let run = run_suite_value(&suite).expect("run secret suite");
        let error = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect_err("secret marker fails redaction preflight");
        assert!(error.to_string().contains("sensitive marker secret"));
    }

    #[test]
    fn redacted_diagnostic_profile_emits_transform_and_stays_diagnostic_only() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "secret-diagnostic" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse secret suite");
        let run = run_suite_value(&suite).expect("run secret suite");
        let bundle = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::RedactedDiagnostic,
        )
        .expect("redacted diagnostic bundle");
        let parsed = parse_repro_bundle(&bundle).expect("parse redacted diagnostic bundle");
        assert_eq!(parsed.export_profile.as_deref(), Some("redacted-diagnostic"));
        assert_eq!(parsed.loss_classification.as_deref(), Some("diagnostic-only"));
        assert!(parsed.redaction_transform_receipt_ref.is_some());
        assert!(
            to_text(parsed.report_value.as_ref().expect("redacted report"))
                .expect("redacted report text")
                .contains("redaction-marker-v1")
        );
        let gate_error = gate_check_value(&bundle).expect_err("diagnostic bundle cannot satisfy pass gate");
        assert!(gate_error.to_string().contains("diagnostic-only"));
        let verify_error = repro_verify_receipt_value(&bundle).expect_err("diagnostic bundle cannot verify as pass");
        assert!(verify_error.to_string().contains("diagnostic-only"));
    }

    #[test]
    fn encrypted_private_profile_rejects_malformed_encrypted_ref_marker() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "bad-encrypted" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <encrypted-ref "bad">>]>"#,
        )
        .expect("parse malformed encrypted suite");
        let run = run_suite_value(&suite).expect("run malformed encrypted suite");
        let error = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::EncryptedPrivate,
        )
        .expect_err("malformed encrypted refs fail closed");
        assert!(error.to_string().contains("malformed encrypted-ref"));
    }

    #[test]
    fn redacted_profile_rejects_stale_transform_receipt() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "stale-transform" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse stale transform suite");
        let run = run_suite_value(&suite).expect("run stale transform suite");
        let bundle = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::RedactedDiagnostic,
        )
        .expect("redacted diagnostic bundle");
        let text = to_text(&bundle).expect("bundle text");
        let tampered = parse_text(&text.replacen("output-bundle", "output-bundle-stale", 1))
            .expect("parse stale transform bundle");
        let error = parse_repro_bundle(&tampered).expect_err("stale transform receipt is rejected");
        assert!(error_contains_any(&error, &["output-bundle", "redaction", "expected"]));
    }

    #[test]
    fn redacted_profile_rejects_missed_sensitive_marker() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "missed-marker" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native"> <actor "b" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "send" "b" #f>]>
              [<send "a" "b" <secret "token">>]>"#,
        )
        .expect("parse missed marker suite");
        let run = run_suite_value(&suite).expect("run missed marker suite");
        let bundle = repro_bundle_value_with_export_profile(
            &run.report_value,
            &["molten".into(), "test".into(), "repro".into(), "export".into()],
            ReproExportProfile::RedactedDiagnostic,
        )
        .expect("redacted diagnostic bundle");
        let text = to_text(&bundle).expect("bundle text");
        let tampered =
            parse_text(&text.replacen("redaction-marker-v1", "secret", 1)).expect("parse missed-marker bundle");
        let error = parse_repro_bundle(&tampered).expect_err("missed marker is rejected");
        assert!(
            error_contains_any(&error, &[
                "missed sensitive marker",
                "redaction",
                "secret",
                "schema",
                "suite value"
            ]),
            "unexpected missed marker error: {error}"
        );
    }

    #[test]
    fn sealed_repro_bundle_rejects_tampered_redaction_gate() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let tampered_text = bundle_text.replacen("redaction-gate-v1", "redaction-gate-v0", 1);
        let tampered_bundle = parse_text(&tampered_text).expect("parse tampered redaction bundle");
        let error = gate_check_value(&tampered_bundle).expect_err("tampered redaction gate fails sealed gate");
        assert!(error_contains_any(&error, &["redaction-gate", "redaction gate"]));
    }

    #[test]
    fn sealed_repro_bundle_rejects_tampered_embedded_report() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let report_start = bundle_text.find("<harness-report-v1").expect("embedded report");
        let final_hash_start =
            report_start + bundle_text[report_start..].find(&run.final_state_hash).expect("embedded report final hash");
        let tampered_text = format!(
            "{}{}{}",
            &bundle_text[..final_hash_start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &bundle_text[final_hash_start + run.final_state_hash.len()..]
        );
        let tampered_bundle = parse_text(&tampered_text).expect("parse tampered sealed bundle");
        let error = gate_check_value(&tampered_bundle).expect_err("tampered embedded report fails sealed gate");
        assert!(error_contains_any(&error, &[
            "repro bundle report ref mismatch",
            "repro bundle state refs do not match embedded report",
            "sealed repro bundle embedded gate receipt does not match report",
        ]));
    }

    #[test]
    fn sealed_repro_bundle_rejects_tampered_embedded_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let receipt_start = bundle_text.find("<gate-receipt-v1").expect("embedded receipt");
        let decision_start =
            receipt_start + bundle_text[receipt_start..].find("<decision \"pass\">").expect("receipt decision");
        let tampered_text = format!(
            "{}<decision \"fail\">{}",
            &bundle_text[..decision_start],
            &bundle_text[decision_start + "<decision \"pass\">".len()..]
        );
        let tampered_bundle = parse_text(&tampered_text).expect("parse tampered receipt bundle");
        let error = gate_check_value(&tampered_bundle).expect_err("tampered embedded receipt fails sealed gate");
        assert!(error_contains_any(&error, &[
            "sealed repro bundle gate receipt ref mismatch",
            "unsupported gate receipt decision",
        ]));
    }

    #[test]
    fn sealed_repro_bundle_rejects_mismatched_suite_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let bundle = sealed_repro_bundle_value_with_command(&run.report_value, &["molten".into(), "test".into()])
            .expect("sealed bundle report");
        let bundle_text = to_text(&bundle).expect("render sealed bundle");
        let tampered_text = bundle_text.replacen(
            &canonical_hash(&suite).expect("suite hash"),
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            1,
        );
        let tampered_bundle = parse_text(&tampered_text).expect("parse mismatched suite bundle");
        let error = gate_check_value(&tampered_bundle).expect_err("mismatched suite ref fails sealed gate");
        assert!(error_contains_any(&error, &[
            "artifact refs missing suite ref",
            "repro bundle suite ref does not match embedded report",
            "repro seal suite ref does not match bundle suite ref",
        ]));
    }

    #[test]
    fn gate_receipt_is_canonical_pass_decision_artifact() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let check = gate_check_value(&run.report_value).expect("gate accepts report");
        let receipt_value = gate_receipt_value(&check);
        let receipt = parse_gate_receipt(&receipt_value).expect("parse gate receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.artifact_kind, "report");
        assert_eq!(receipt.report_ref, run.report_ref);
        assert_eq!(receipt.suite_ref, check.suite_ref);
        assert!(receipt.checks.iter().any(|check| check == "budget"));
        assert!(receipt.checks.iter().any(|check| check == "explicit-budget-fixture"));
        assert!(receipt.checks.iter().any(|check| check == "no-default-resource-policy"));
        assert!(receipt.checks.iter().any(|check| check == "resource-policy-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "nickel-resource-policy"));
        assert!(receipt.checks.iter().any(|check| check == "nickel-resource-export"));
        assert!(receipt.checks.iter().any(|check| check == "basalt-resource-receipt"));
        assert!(receipt.checks.iter().any(|check| check == "budget-usage-binding"));
        assert!(receipt.checks.iter().any(|check| check == "actor-registry"));
        assert!(receipt.checks.iter().any(|check| check == "explicit-actor-registry"));
        assert!(receipt.checks.iter().any(|check| check == "no-inferred-actors"));
        assert!(receipt.checks.iter().any(|check| check == "executor-boundary"));
        assert!(receipt.checks.iter().any(|check| check == "admission-policy"));
        assert!(receipt.checks.iter().any(|check| check == "policy-preflight"));
        assert!(receipt.checks.iter().any(|check| check == "nickel-static-policy"));
        assert!(receipt.checks.iter().any(|check| check == "nickel-policy-source"));
        assert!(receipt.checks.iter().any(|check| check == "nickel-export-normalization"));
        assert!(receipt.checks.iter().any(|check| check == "basalt-policy-gate"));
        assert!(receipt.checks.iter().any(|check| check == "basalt-preflight-receipt"));
        assert!(receipt.checks.iter().any(|check| check == "basalt-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "steel-predicate-review"));
        assert!(receipt.checks.iter().any(|check| check == "explicit-capability-fixture"));
        assert!(receipt.checks.iter().any(|check| check == "no-implicit-authority"));
        assert!(receipt.checks.iter().any(|check| check == "capability-context"));
        assert!(receipt.checks.iter().any(|check| check == "capability-grants"));
        assert!(receipt.checks.iter().any(|check| check == "basalt-authority-receipt"));
        assert!(receipt.checks.iter().any(|check| check == "capability-proofset-binding"));
        assert!(receipt.checks.iter().any(|check| check == "grant-ref-binding"));
        assert!(receipt.checks.iter().any(|check| check == "deny-without-capability"));
        assert!(receipt.checks.iter().any(|check| check == "authority-ref-binding"));
        assert!(receipt.checks.iter().any(|check| check == "admission-decisions"));
        assert!(receipt.checks.iter().any(|check| check == "deny-rollback"));
        assert!(receipt.checks.iter().any(|check| check == "denied-effect-suppression"));
        assert!(receipt.checks.iter().any(|check| check == "runtime-predicate-receipts"));
        assert!(receipt.checks.iter().any(|check| check == "assertion-visibility-predicate"));
        assert!(receipt.checks.iter().any(|check| check == "turn-commit-rollback-predicate"));
        assert!(receipt.checks.iter().any(|check| check == "observe-delivery-predicate"));
        assert!(receipt.checks.iter().any(|check| check == "executor-conformance-suite-binding"));
        assert!(receipt.checks.iter().any(|check| check == "cross-kind-hostcall-conformance"));
        assert!(receipt.checks.iter().any(|check| check == "chain-continuity"));
        assert!(receipt.checks.iter().any(|check| check == "chain-anchor-descent"));
        assert!(receipt.checks.iter().any(|check| check == "chain-checkpoint-freshness"));
        assert!(receipt.checks.iter().any(|check| check == "chain-predicate-receipts"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-chains"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-input-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-admission-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-state-binding"));
        assert!(receipt.checks.iter().any(|check| check == "turn-journal-no-global-head"));
        let parsed_report = parse_report(&run.report_value).expect("parse report");
        let runtime_predicates = parsed_report
            .observations
            .iter()
            .flat_map(|observation| observation.events.iter())
            .filter(|event| event.collect_simple_record("runtime-predicate-receipt-v1", None).is_some())
            .count();
        assert!(runtime_predicates >= 3);
        assert!(gate_receipt_summary(&receipt_value).expect("receipt summary").contains("decision=pass"));
        let rendered = to_text(&receipt_value).expect("render receipt");
        let reparsed = parse_text(&rendered).expect("reparse receipt");
        assert_eq!(canonical_hash(&receipt_value).unwrap(), canonical_hash(&reparsed).unwrap());
    }

    #[test]
    fn gate_rejects_failure_and_failure_repro_bundle_as_pass_evidence() {
        let error = MoltenError::invalid_harness("synthetic preflight failure");
        let failure = failure_value("preflight", &error, Vec::new());
        let gate_error = gate_check_value(&failure).expect_err("failure cannot satisfy gate");
        assert!(gate_error.to_string().contains("cannot satisfy pass evidence gate"));

        let failure_bundle = failure_repro_bundle_value(&failure).expect("failure bundle");
        let parsed_bundle = parse_repro_bundle(&failure_bundle).expect("parse failure bundle");
        assert_eq!(parsed_bundle.kind, super::HarnessReproBundleKind::Failure);
        let gate_error = gate_check_value(&failure_bundle).expect_err("failure bundle cannot satisfy gate");
        assert!(gate_error.to_string().contains("cannot satisfy pass evidence gate"));
    }

    #[test]
    fn actor_executor_registry_marks_placeholders() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-future" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "module" "wasm">]>
              [<clock "module">]>"#,
        )
        .expect("parse wasm actor suite");
        let parsed = parse_suite(&suite).expect("parse suite");
        let executors = actor_executor_registry(&parsed.actors);
        assert_eq!(executors.len(), 1);
        assert_eq!(executors[0].executor_kind, super::ActorExecutorKind::WasmPlaceholder);
        assert!(!executors[0].supported);
    }

    #[test]
    fn unsupported_actor_kind_fails_for_now() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-future" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "module" "wasm">]>
              [<clock "module">]>"#,
        )
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("wasm actor kind requires explicit preflight");
        assert!(error.to_string().contains("missing Wasm executor preflight fixture"));
    }

    #[test]
    fn adapter_and_remote_proxy_kinds_remain_fail_closed_without_preflight() {
        for kind in ["adapter", "remote-proxy"] {
            let suite = parse_text(&format!(
                r#"<harness-suite-v1 "molten.harness.suite.v1" "{kind}-disabled" 1
                  <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
                  <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "subject" "{kind}">]>
                  <capabilities-v1 "molten.harness.capabilities.v1" [<grant "subject" "assert" #f "service.ready">]>
                  [<assert "subject" "service.ready">]>"#,
            ))
            .expect("parse disabled executor suite");
            let error = run_suite_value(&suite).expect_err("adapter-like executor kind remains disabled");
            assert!(
                error.to_string().contains(&format!(
                    "executor kind {kind} requires executor adapter preflight and remains disabled in local harness"
                )),
                "{error}"
            );
        }
    }

    #[test]
    fn adapter_and_remote_proxy_run_with_executable_preflight_fixtures() {
        let adapter_suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "adapter-preflight" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 32768>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "subject" "adapter" <adapter-executor-v1 "molten.runtime.adapter-executor.v1"
                  <manifest "local-test-adapter">
                  <abi "molten.adapter.preserves.v1">
                  <allowed-hostcalls ["assert"]>
                  <transcript "deterministic-local">>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "subject" "assert" #f "service.ready">
              ]>
              [<assert "subject" "service.ready">]>"#,
        )
        .expect("parse adapter preflight suite");
        let adapter = run_suite_value(&adapter_suite).expect("adapter preflight suite runs");
        let adapter_text = to_text(&adapter.report_value).expect("render adapter report");
        assert!(adapter_text.contains("adapter-preflight-receipt-v1"));
        assert!(adapter_text.contains("adapter-transcript-replay"));
        validate_report_value(&adapter.report_value).expect("adapter preflight report validates");

        let remote_suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "remote-proxy-preflight" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 32768>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "subject" "remote-proxy" <remote-proxy-executor-v1 "molten.runtime.remote-proxy-executor.v1"
                  <peer "peer:test">
                  <endpoint "iroh:endpoint:test">
                  <contract "molten.actor.contract.test">
                  <allowed-hostcalls ["assert"]>
                  <transcript "verified">>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "subject" "assert" #f "service.ready">
              ]>
              [<assert "subject" "service.ready">]>"#,
        )
        .expect("parse remote proxy preflight suite");
        let remote = run_suite_value(&remote_suite).expect("remote-proxy preflight suite runs");
        let remote_text = to_text(&remote.report_value).expect("render remote report");
        assert!(remote_text.contains("remote-proxy-preflight-receipt-v1"));
        assert!(remote_text.contains("remote-transcript-replay"));
        validate_report_value(&remote.report_value).expect("remote preflight report validates");
    }

    #[test]
    fn adapter_and_remote_proxy_negative_security_preflights_fail_closed() {
        let adapter_ambient = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "adapter-ambient" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 32768>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "subject" "adapter" <adapter-executor-v1 "molten.runtime.adapter-executor.v1"
                  <manifest "ambient-network socket">
                  <abi "molten.adapter.preserves.v1">
                  <allowed-hostcalls ["assert"]>
                  <transcript "deterministic-local">>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "subject" "assert" #f "service.ready">]>
              [<assert "subject" "service.ready">]>"#,
        )
        .expect("parse ambient adapter suite");
        let error = run_suite_value(&adapter_ambient).expect_err("ambient adapter fails preflight");
        assert!(error.to_string().contains("forbidden ambient or stale token"), "{error}");

        let adapter_undeclared = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "adapter-undeclared" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 32768>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "subject" "adapter" <adapter-executor-v1 "molten.runtime.adapter-executor.v1"
                  <manifest "local-test-adapter">
                  <abi "molten.adapter.preserves.v1">
                  <allowed-hostcalls ["send"]>
                  <transcript "deterministic-local">>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "subject" "assert" #f "service.ready">]>
              [<assert "subject" "service.ready">]>"#,
        )
        .expect("parse adapter undeclared suite");
        let error = run_suite_value(&adapter_undeclared).expect_err("undeclared adapter hostcall fails");
        assert!(
            error.to_string().contains("hostcall operation assert is not allowed by adapter executor preflight"),
            "{error}"
        );

        let remote_unknown = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "remote-unknown" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 32768>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "subject" "remote-proxy" <remote-proxy-executor-v1 "molten.runtime.remote-proxy-executor.v1"
                  <peer "unknown">
                  <endpoint "iroh:endpoint:test">
                  <contract "molten.actor.contract.test">
                  <allowed-hostcalls ["assert"]>
                  <transcript "verified">>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "subject" "assert" #f "service.ready">]>
              [<assert "subject" "service.ready">]>"#,
        )
        .expect("parse unknown remote suite");
        let error = run_suite_value(&remote_unknown).expect_err("unknown remote peer fails");
        assert!(error.to_string().contains("cannot satisfy trusted deterministic gate evidence"), "{error}");

        let remote_transcript = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "remote-transcript" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 32768>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "subject" "remote-proxy" <remote-proxy-executor-v1 "molten.runtime.remote-proxy-executor.v1"
                  <peer "peer:test">
                  <endpoint "http://example.invalid">
                  <contract "molten.actor.contract.test stale-signature">
                  <allowed-hostcalls ["assert"]>
                  <transcript "live">>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "subject" "assert" #f "service.ready">]>
              [<assert "subject" "service.ready">]>"#,
        )
        .expect("parse remote transcript suite");
        let error = run_suite_value(&remote_transcript).expect_err("bad remote transport/transcript fails");
        assert!(
            error_contains_any(&error, &[
                "explicit iroh: transport profile",
                "stale signature",
                "must be verified"
            ]),
            "{error}"
        );
    }

    #[test]
    fn executor_conformance_suites_bind_same_profile_across_native_steel_and_wasm() {
        let suite = parse_text(&cross_kind_conformance_suite()).expect("parse cross-kind conformance suite");
        let run = run_suite_value(&suite).expect("run cross-kind conformance suite");
        let report = parse_report(&run.report_value).expect("parse conformance report");
        let preflights = report.executor_preflights.expect("executor preflights");
        let native = preflights
            .preflights
            .iter()
            .find(|preflight| preflight.actor_id == "native")
            .expect("native preflight");
        let steel = preflights
            .preflights
            .iter()
            .find(|preflight| preflight.actor_id == "steel")
            .expect("steel preflight");
        let wasm = preflights.preflights.iter().find(|preflight| preflight.actor_id == "wasm").expect("wasm preflight");
        assert_eq!(native.allowed_hostcalls, vec!["assert".to_string(), "send".to_string()]);
        assert_eq!(native.conformance_refs, steel.conformance_refs);
        assert_eq!(native.conformance_refs, wasm.conformance_refs);
        assert_eq!(native.conformance_refs.len(), 1);
        validate_report_value(&run.report_value).expect("validate cross-kind conformance report");
        let gate = gate_check_value(&run.report_value).expect("gate cross-kind conformance report");
        let receipt = parse_gate_receipt(&gate_receipt_value(&gate)).expect("parse conformance gate receipt");
        assert!(receipt.checks.iter().any(|check| check == "executor-conformance-suite-binding"));
        assert!(receipt.checks.iter().any(|check| check == "cross-kind-hostcall-conformance"));
        assert!(receipt.checks.iter().any(|check| check == "executor-execution-receipt-binding"));
        assert!(receipt.checks.iter().any(|check| check == "executor-output-ref-binding"));
    }

    #[test]
    fn native_steel_and_wasm_conformance_suites_produce_same_final_state() {
        let native_suite = parse_text(&conformance_suite_for_subject(r#"<actor "subject" "native">"#))
            .expect("parse native conformance suite");
        let steel_actor = r#"<actor "subject" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["assert" "send"]>>>"#;
        let steel_suite =
            parse_text(&conformance_suite_for_subject(steel_actor)).expect("parse steel conformance suite");
        let wasm_actor = format!(
            r#"<actor "subject" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{WASM_ASSERT_SEND_IMPORT_MODULE_HEX}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert" "send"]>>>"#,
        );
        let wasm_suite = parse_text(&conformance_suite_for_subject(&wasm_actor)).expect("parse wasm conformance suite");

        let native = run_suite_value(&native_suite).expect("run native conformance suite");
        let steel = run_suite_value(&steel_suite).expect("run steel conformance suite");
        let wasm = run_suite_value(&wasm_suite).expect("run wasm conformance suite");
        assert_eq!(native.final_state_hash, steel.final_state_hash);
        assert_eq!(native.final_state_hash, wasm.final_state_hash);
        validate_report_value(&native.report_value).expect("validate native conformance report");
        validate_report_value(&steel.report_value).expect("validate steel conformance report");
        validate_report_value(&wasm.report_value).expect("validate wasm conformance report");
    }

    #[test]
    fn reviewed_wasm_executor_hostcall_suite_runs_with_inspection_receipt() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-hostcall" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "0061736d01000000010401600000021a010f6d6f6c74656e3a686f737463616c6c06617373657274000003020100071a01166d6f6c74656e5f686f737463616c6c5f61737365727400010a0601040010000b">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        )
        .expect("parse wasm actor suite");
        let parsed = parse_suite(&suite).expect("parse suite");
        let executors = actor_executor_registry(&parsed.actors);
        assert_eq!(executors[0].executor_kind, super::ActorExecutorKind::WasmReviewed);
        assert!(executors[0].supported);
        let run = run_suite_value(&suite).expect("reviewed wasm hostcall suite runs");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("wasm-inspection-receipt-v1"));
        assert!(report_text.contains("molten:hostcall"));
        assert!(report_text.contains("wasm-module-ref-binding"));
        assert!(report_text.contains("wasmparser-inspection"));
        assert!(report_text.contains("wasm-execution-receipt-v1"));
        assert!(report_text.contains("wasmtime-instantiated"));
        assert!(report_text.contains("effect-manifest-bound"));
        assert!(report_text.contains("effect-request-admitted"));
        assert!(report_text.contains("declared-effect-id-required"));
        validate_report_value(&run.report_value).expect("wasm report validates");
    }

    #[test]
    fn reviewed_wasm_preserves_abi_records_input_and_output_refs() {
        let module_hex = wasm_abi_module_hex(&["assert"]);
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-abi" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{module_hex}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm abi suite");
        let run = run_suite_value(&suite).expect("reviewed wasm abi suite runs");
        let report_text = to_text(&run.report_value).expect("render wasm abi report");
        assert!(report_text.contains("molten.wasm.abi.v1"));
        assert!(report_text.contains("input-ref"));
        assert!(report_text.contains("output-ref"));
        assert!(report_text.contains("canonical-preserves-output"));
        validate_report_value(&run.report_value).expect("wasm abi report validates");
    }

    #[test]
    fn wasm_preserves_abi_requires_memory_export() {
        let module_hex = wasm_abi_missing_memory_hex();
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-abi-no-memory" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{module_hex}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm missing memory suite");
        let error = run_suite_value(&suite).expect_err("missing memory export fails");
        assert!(error.to_string().contains("does not export memory"), "{error}");
    }

    #[test]
    fn wasm_preserves_abi_rejects_invalid_output_bytes() {
        let module_hex = wasm_abi_module_hex_with_output_bytes(&["assert"], &[0xff, 0x00]);
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-abi-invalid-output" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{module_hex}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm invalid output suite");
        let error = run_suite_value(&suite).expect_err("invalid Preserves output fails");
        assert!(error.to_string().contains("invalid canonical Preserves output bytes"), "{error}");
    }

    #[test]
    fn wasm_preserves_abi_rejects_descriptor_hostcall_and_fuel_failures() {
        let output = parse_text("<wasm-output \"ok\">").expect("parse wasm output");
        let output_bytes = canonical_bytes(&output).expect("encode wasm output");
        let cases = [
            (
                "wasm-abi-oob-output",
                wasm_abi_module_hex_with_descriptor(&["assert"], &output_bytes, (70_000u64 << 32) | 16),
                "out of guest memory bounds",
            ),
            (
                "wasm-abi-oversized-output",
                wasm_abi_module_hex_with_descriptor(&["assert"], &output_bytes, (2048u64 << 32) | (9 * 1024)),
                "output bytes exceed molten.wasm.abi.v1 limit",
            ),
            (
                "wasm-abi-invalid-hostcall",
                wasm_abi_invalid_hostcall_bytes_hex(),
                "trapped under molten.wasm.abi.v1",
            ),
            ("wasm-abi-fuel", wasm_abi_fuel_exhaustion_hex(), "trapped under molten.wasm.abi.v1"),
        ];
        for (name, module_hex, message) in cases {
            let suite = parse_text(&format!(
                r#"<harness-suite-v1 "molten.harness.suite.v1" "{name}" 5
                  <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
                  <actor-registry-v1 "molten.harness.actor-registry.v1" [
                    <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                      <module-hex "{module_hex}">
                      <wit "molten:hostcalls/runtime-spine@1.0.0">
                      <allowed-hostcalls ["assert"]>>>
                  ]>
                  <capabilities-v1 "molten.harness.capabilities.v1" [
                    <grant "module" "assert" #f "service.ready">
                  ]>
                  [<assert "module" "service.ready">]>"#,
            ))
            .expect("parse wasm negative suite");
            let error = run_suite_value(&suite).expect_err("Wasm ABI negative suite fails closed");
            assert!(error.to_string().contains(message), "{name}: {error}");
        }
    }

    #[test]
    fn wasm_execution_receipt_tampered_abi_input_ref_fails_validation() {
        let module_hex = wasm_abi_module_hex(&["assert"]);
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-abi-input-ref-tamper" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{module_hex}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm input-ref tamper suite");
        let run = run_suite_value(&suite).expect("run wasm input-ref tamper suite");
        let report_text = to_text(&run.report_value).expect("render wasm report");
        let marker = "<input-ref \"";
        let start = report_text.find(marker).expect("input-ref marker") + marker.len();
        let end = start + "blake3:".len();
        let tampered_text = format!("{}blake3:deadbeef{}", &report_text[..start], &report_text[end..]);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Wasm ABI input-ref report");
        let error = validate_report_value(&tampered_report).expect_err("tampered ABI input ref fails validation");
        assert!(
            error_contains_any(&error, &["Wasm execution input ref mismatch", "Wasm execution input ref"]),
            "{error}"
        );
    }

    #[test]
    fn wasm_executor_preflight_rejects_invalid_module() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-invalid" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "00">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        )
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("invalid Wasm module fails preflight");
        assert!(error.to_string().contains("wasmparser validation failed"));
    }

    #[test]
    fn wasm_executor_preflight_rejects_wasi_and_ambient_imports() {
        for (suite_name, module_hex, expected) in [
            ("wasm-wasi-import", WASM_WASI_FD_WRITE_IMPORT_MODULE_HEX, "WASI and ambient imports remain disabled"),
            ("wasm-env-import", WASM_ENV_READ_IMPORT_MODULE_HEX, "WASI and ambient imports remain disabled"),
        ] {
            let suite = parse_text(&format!(
                r#"<harness-suite-v1 "molten.harness.suite.v1" "{suite_name}" 5
                  <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
                  <actor-registry-v1 "molten.harness.actor-registry.v1" [
                    <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                      <module-hex "{module_hex}">
                      <wit "molten:hostcalls/runtime-spine@1.0.0">
                      <allowed-hostcalls ["assert"]>>>
                  ]>
                  <capabilities-v1 "molten.harness.capabilities.v1" [
                    <grant "module" "assert" #f "service.ready">
                  ]>
                  [<assert "module" "service.ready">]>"#,
            ))
            .expect("parse wasm actor suite");
            let error = run_suite_value(&suite).expect_err("ambient Wasm import fails preflight");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn wasm_executor_preflight_rejects_import_outside_allowed_hostcalls() {
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-import-deny" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{WASM_ASSERT_IMPORT_MODULE_HEX}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["send"]>>>
                <actor "sink" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "send" "sink" #f>
              ]>
              [<send "module" "sink" "hello">]>"#,
        ))
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("unlisted Wasm import fails preflight");
        assert!(
            error
                .to_string()
                .contains("Wasm executor import molten:hostcall::assert for actor module is not in allowed hostcalls"),
            "{error}"
        );
    }

    #[test]
    fn wasm_executor_preflight_rejects_empty_wit() {
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-empty-wit" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{WASM_ASSERT_IMPORT_MODULE_HEX}">
                  <wit "">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("empty WIT fails preflight");
        assert!(error.to_string().contains("Wasm executor WIT interface for actor module is empty"));
    }

    #[test]
    fn wasm_inspection_receipt_tamper_fails_validation_before_gate() {
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-receipt-tamper" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{WASM_ASSERT_IMPORT_MODULE_HEX}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm actor suite");
        let run = run_suite_value(&suite).expect("run wasm receipt tamper suite");
        let report_text = to_text(&run.report_value).expect("render wasm report");
        let tampered_text = report_text.replacen("wasmparser-validated", "wasmparser-stale", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Wasm receipt report");
        let error = validate_report_value(&tampered_report).expect_err("tampered Wasm inspection receipt fails");
        assert!(
            error_contains_any(&error, &["wasmparser-validated", "executor preflight evidence mismatch"]),
            "{error}"
        );
    }

    #[test]
    fn wasm_executor_requires_operation_export_to_emit_hostcall() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-missing-export" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "0061736d01000000010401600000021a010f6d6f6c74656e3a686f737463616c6c066173736572740000">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        )
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("Wasm actor must export operation entrypoint");
        assert!(
            error
                .to_string()
                .contains("missing required export molten_hostcall_assert for hostcall operation assert"),
            "{error}"
        );
    }

    #[test]
    fn wasm_execution_receipt_tamper_reports_replay_divergence() {
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-execution-tamper" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{WASM_ASSERT_IMPORT_MODULE_HEX}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse wasm actor suite");
        let run = run_suite_value(&suite).expect("run wasm execution tamper suite");
        let report_text = to_text(&run.report_value).expect("render wasm report");
        let tampered_text = report_text.replacen("wasmtime-instantiated", "wasmtime-stale", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Wasm execution report");
        let error = replay_report_value(&tampered_report).expect_err("tampered Wasm execution receipt diverges");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "wasm-execution");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected wasm-execution divergence, got {other}"),
        }
    }

    #[test]
    fn wasm_executor_preflight_rejects_undeclared_hostcall() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "wasm-hostcall-deny" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "0061736d01000000">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls ["send"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        )
        .expect("parse wasm actor suite");
        let error = run_suite_value(&suite).expect_err("undeclared Wasm hostcall fails preflight");
        assert!(error.to_string().contains("hostcall operation assert is not allowed by Wasm executor preflight"));
    }

    #[test]
    fn reviewed_steel_executor_hostcall_suite_runs_with_preflight_receipt() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-hostcall" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let parsed = parse_suite(&suite).expect("parse suite");
        let executors = actor_executor_registry(&parsed.actors);
        assert_eq!(executors[0].executor_kind, super::ActorExecutorKind::SteelReviewed);
        assert!(executors[0].supported);
        let run = run_suite_value(&suite).expect("reviewed steel hostcall suite runs");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("steel-review-receipt-v1"));
        assert!(report_text.contains("steel-execution-receipt-v1"));
        assert!(report_text.contains("steel-vm-executed"));
        assert!(report_text.contains("effect-manifest-bound"));
        assert!(report_text.contains("effect-request-admitted"));
        assert!(report_text.contains("declared-effect-id-required"));
        assert!(report_text.contains("steel-source-ref-binding"));
        assert!(report_text.contains("steel-callable-review"));
        validate_report_value(&run.report_value).expect("steel report validates");
    }

    #[test]
    fn steel_actor_without_reviewed_executor_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-missing-preflight" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "steel">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let error = run_suite_value(&suite).expect_err("steel actor needs reviewed executor fixture");
        assert!(error.to_string().contains("missing reviewed Steel executor preflight fixture"));
    }

    #[test]
    fn steel_executor_preflight_rejects_ambient_source_tokens() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-ambient" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(open-input-file \"/tmp/ambient\")">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let error = run_suite_value(&suite).expect_err("ambient steel source fails preflight");
        assert!(error.to_string().contains("forbidden ambient IO token open-input-file"));
    }

    #[test]
    fn steel_executor_preflight_rejects_undeclared_hostcall() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-hostcall-deny" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["send"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel actor suite");
        let error = run_suite_value(&suite).expect_err("undeclared Steel hostcall fails preflight");
        assert!(error.to_string().contains("hostcall operation assert is not allowed by Steel executor preflight"));
    }

    #[test]
    fn steel_execution_receipt_tamper_reports_replay_divergence() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-execution-tamper" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) input)">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse steel tamper suite");
        let run = run_suite_value(&suite).expect("run steel tamper suite");
        let report_text = to_text(&run.report_value).expect("render steel report");
        assert!(report_text.contains("resource-bounded"));
        let tampered_text = report_text.replacen("steel-vm-executed", "steel-vm-stale", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered Steel execution report");
        let error = replay_report_value(&tampered_report).expect_err("tampered Steel execution receipt diverges");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "steel-execution");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected steel-execution divergence, got {other}"),
        }
    }

    #[test]
    fn steel_executor_resource_limits_fail_closed() {
        let oversized_source = "x".repeat(9 * 1024);
        let suite = parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-resource" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "{oversized_source}">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        ))
        .expect("parse steel resource suite");
        let error = run_suite_value(&suite).expect_err("oversized Steel source fails resource preflight");
        assert!(error.to_string().contains("exceeds deterministic resource limit"), "{error}");

        let recursive = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-recursive" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) (main input))">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse recursive Steel suite");
        let error = run_suite_value(&recursive).expect_err("recursive Steel source fails resource preflight");
        assert!(error.to_string().contains("recursive callable"), "{error}");

        let unbounded_output = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "steel-unbounded-output" 3
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "producer" "steel" <steel-executor-v1 "molten.runtime.steel-executor.v1"
                  <source "(define (main input) (make-string 9000 #\\a))">
                  <callable "main">
                  <allowed-hostcalls ["assert"]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "producer" "assert" #f "service.ready">
              ]>
              [<assert "producer" "service.ready">]>"#,
        )
        .expect("parse unbounded Steel output suite");
        let error = run_suite_value(&unbounded_output).expect_err("unbounded Steel output fails resource preflight");
        assert!(error.to_string().contains("unbounded resource token make-string"), "{error}");
    }

    #[test]
    fn report_tampered_actor_registry_fails_validation() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let registry_start = report_text.rfind("<actor-registry-v1").expect("report actor registry");
        let native_offset = report_text[registry_start..].find("\"native\"").expect("native actor kind");
        let start = registry_start + native_offset;
        let tampered_text = format!("{}\"steel\"{}", &report_text[..start], &report_text[start + "\"native\"".len()..]);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("tampered actor registry should fail");
        assert!(error.to_string().contains("report actor registry does not match embedded suite actor registry"));
    }

    #[test]
    fn runtime_snapshot_hashes_are_canonical_and_seed_bound() {
        let left = RuntimeState::new(7);
        let right = RuntimeState::new(7);
        let other_seed = RuntimeState::new(8);
        let left_ref = canonical_hash(&snapshot_value(&left.snapshot())).expect("left snapshot ref");
        let right_ref = canonical_hash(&snapshot_value(&right.snapshot())).expect("right snapshot ref");
        let other_ref = canonical_hash(&snapshot_value(&other_seed.snapshot())).expect("other snapshot ref");

        assert_eq!(left_ref, right_ref);
        assert_ne!(left_ref, other_ref);
    }

    #[test]
    fn replay_preserves_initial_and_final_state_hashes() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay with effect log");
        let replayed_report = parse_report(&replayed.report_value).expect("parse replay report");

        assert_eq!(report.initial_state_hash, replayed_report.initial_state_hash);
        assert_eq!(report.final_state_hash, replayed_report.final_state_hash);
        assert_eq!(run.initial_state_hash, replayed.initial_state_hash);
        assert_eq!(run.final_state_hash, replayed.final_state_hash);
    }

    #[test]
    fn runtime_transition_module_is_deterministic_without_io() {
        let steps = [
            CoreStep::Observe {
                actor: "consumer".into(),
                pattern: RuntimeValue::string("service.ready").expect("runtime test value"),
            },
            CoreStep::Assert {
                actor: "producer".into(),
                value: RuntimeValue::string("service.ready").expect("runtime test value"),
            },
            CoreStep::Clock {
                actor: "producer".into(),
            },
            CoreStep::Random {
                actor: "producer".into(),
                upper: 100,
            },
        ];
        let mut left = RuntimeState::new(7);
        let mut right = RuntimeState::new(7);
        for step in &steps {
            assert_eq!(left.apply_step(step), right.apply_step(step));
            assert_eq!(left.snapshot(), right.snapshot());
        }
    }

    #[test]
    fn tampered_observation_reports_first_trace_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("message-delivered", "message-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered report must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "trace");
                assert_eq!(divergence.step, Some(2));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn record_profile_captures_clock_random_effect_log() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let observed_effect_log = effect_log_from_observations(&report.observations).expect("observed effect log");
        let rendered_log = to_text(&super::schema::effect_log_value(&report.effect_log)).expect("render effect log");

        assert_eq!(report.effect_log, observed_effect_log);
        assert_eq!(report.effect_log.len(), 2);
        assert!(rendered_log.contains("<effect-request \"clock\" \"producer\" 0>"));
        assert!(rendered_log.contains("<effect-response \"clock\" \"producer\" 0"));
        assert!(rendered_log.contains("<effect-request \"random\" \"producer\" 1 100>"));
        assert!(rendered_log.contains("<effect-response \"random\" \"producer\" 1 100"));

        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay with recorded log");
        assert_eq!(run.report_ref, replayed.report_ref);
    }

    #[test]
    fn replay_profile_injects_recorded_clock_random_effects() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = parse_report(&run.report_value).expect("parse report");
        let parsed_suite = parse_suite(&report.suite_value).expect("parse embedded suite");
        let replayed = run_suite_with_effect_log(&parsed_suite, &report.effect_log).expect("replay with effect log");
        let rendered = to_text(&replayed.report_value).expect("render replayed report");

        assert_eq!(run.report_ref, replayed.report_ref);
        assert_eq!(run.final_state_hash, replayed.final_state_hash);
        assert!(rendered.contains("time-random-handler-receipt-v1"));
        assert!(rendered.contains("deny-by-default-bypassed-only-by-local-test-handler"));
    }

    #[test]
    fn tampered_effect_response_reports_effect_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("effect-response", "effect-response-tampered", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered report must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "effect-response");
                assert_eq!(divergence.step, Some(3));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn tampered_effect_log_request_reports_effect_request_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(
            "<effect-entry 1 <effect-request \"random\" \"producer\" 1 100>",
            "<effect-entry 1 <effect-request \"random\" \"producer\" 1 99>",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered effect log must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "effect-request");
                assert_eq!(divergence.step, Some(4));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn missing_effect_log_entry_reports_effect_log_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.find("<effect-log-v1").expect("effect log start");
        let relative_end = report_text[start..].find("]>").expect("effect log end") + 2;
        let end = start + relative_end;
        let tampered_text = format!(
            "{}<effect-log-v1 \"molten.harness.effect-log.v1\" []>{}",
            &report_text[..start],
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("missing effect log must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "effect-log");
                assert_eq!(divergence.step, Some(3));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn tampered_runtime_predicate_reports_runtime_predicate_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(
            "molten.trellis-runtime.turn-commit-rollback.v1",
            "molten.trellis-runtime.turn-commit-rollback-tampered.v1",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered runtime predicate must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "runtime-predicate");
                assert_eq!(divergence.step, Some(0));
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn report_validation_rejects_missing_runtime_predicate_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen("runtime-predicate-receipt-v1", "runtime-predicate-missing-v1", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error =
            validate_report_value(&tampered_report).expect_err("missing runtime predicate must fail validation");
        assert!(error.to_string().contains("runtime predicate"));
    }

    #[test]
    fn tampered_final_state_reports_state_divergence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(
            &run.final_state_hash,
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            1,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = replay_report_value(&tampered_report).expect_err("tampered report must diverge");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "final-state");
                assert_eq!(divergence.step, None);
            }
            other => panic!("expected divergence, got {other}"),
        }
    }

    #[test]
    fn report_validation_rejects_missing_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let tampered_text = report_text.replacen(&run.final_state_hash, "missing-final-state", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("missing evidence should fail closed");
        assert!(error.to_string().contains("expected canonical content ref"));
    }

    #[test]
    fn too_many_steps_fails_resource_budget() {
        let steps = (0..65).map(|_| "<send \"a\" \"b\" \"m\">".to_string()).collect::<Vec<_>>().join(" ");
        let suite = parse_text(&format!(
            "<harness-suite-v1 \"molten.harness.suite.v1\" \"too-many-steps\" 1 \
             <budget-v1 \"molten.harness.budget.v1\" <limits 64 16 256 65536>> \
             <actor-registry-v1 \"molten.harness.actor-registry.v1\" [<actor \"a\" \"native\"> <actor \"b\" \"native\">]> \
             <capabilities-v1 \"molten.harness.capabilities.v1\" [<grant #f \"send\" #f #f>]> [{steps}]>"
        ))
        .expect("parse suite");
        let error = run_suite_value(&suite).expect_err("step budget should fail");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "resource");
                assert_eq!(divergence.detail, "suite step count exceeds budget");
            }
            other => panic!("expected resource divergence, got {other}"),
        }
    }

    #[test]
    fn too_many_effects_fails_resource_budget() {
        let steps = (0..17).map(|_| "<clock \"a\">".to_string()).collect::<Vec<_>>().join(" ");
        let suite = parse_text(&format!(
            "<harness-suite-v1 \"molten.harness.suite.v1\" \"too-many-effects\" 1 \
             <budget-v1 \"molten.harness.budget.v1\" <limits 64 16 256 65536>> \
             <actor-registry-v1 \"molten.harness.actor-registry.v1\" [<actor \"a\" \"native\">]> \
             <capabilities-v1 \"molten.harness.capabilities.v1\" [<grant \"a\" \"clock\" #f #f>]> [{steps}]>"
        ))
        .expect("parse suite");
        let error = run_suite_value(&suite).expect_err("effect budget should fail");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "resource");
                assert_eq!(divergence.step, Some(16));
                assert_eq!(divergence.detail, "effect count exceeds budget");
            }
            other => panic!("expected resource divergence, got {other}"),
        }
    }

    #[test]
    fn report_validation_requires_budget_evidence() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.rfind("budget-v1").expect("report budget marker");
        let tampered_text = format!("{}budget-v0{}", &report_text[..start], &report_text[start + "budget-v1".len()..]);
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("bad budget evidence should fail");
        assert!(error.to_string().contains("expected <budget-v1"));
    }

    #[test]
    fn report_validation_requires_budget_gate_preflight() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("budget-gate-v1"));
        let tampered_text = report_text.replacen("budget-gate-v1", "budget-gate-missing", 1);
        let tampered_report = parse_text(&tampered_text).expect("parse missing budget gate report");
        let error = validate_report_value(&tampered_report).expect_err("missing resource preflight fails closed");
        assert!(error_contains_any(&error, &["budget gate", "actor-registry"]));
    }

    #[test]
    fn report_validation_rejects_tampered_budget_gate_ref() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let marker = "<budget-ref \"";
        let start = report_text.find(marker).expect("budget ref marker") + marker.len();
        let end = start + "blake3:".len() + 64;
        let tampered_text = format!(
            "{}{}{}",
            &report_text[..start],
            "blake3:0000000000000000000000000000000000000000000000000000000000000000",
            &report_text[end..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered budget gate report");
        let error = validate_report_value(&tampered_report).expect_err("tampered budget gate ref fails validation");
        assert!(error_contains_any(&error, &[
            "budget gate ref mismatch",
            "Nickel resource policy budget ref does not match budget gate ref",
            "Basalt resource preflight budget ref does not match budget gate ref",
        ]));
    }

    #[test]
    fn report_validation_rejects_tampered_nickel_resource_export() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        assert!(report_text.contains("export-json"));
        let tampered_text = report_text.replacen(
            "molten.harness.budget.nickel-static.v1",
            "molten.harness.budget.nickel-static.tampered",
            2,
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered resource export report");
        let error = validate_report_value(&tampered_report).expect_err("tampered resource export fails closed");
        assert!(error_contains_any(&error, &[
            "Nickel resource policy export JSON does not match source normalization",
            "Nickel resource policy export ref mismatch",
            "unsupported Nickel resource policy schema",
        ]));
    }

    #[test]
    fn report_validation_rejects_tampered_basalt_resource_receipt() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let budget_gate_start = report_text.find("budget-gate-v1").expect("budget gate");
        let reason_relative = report_text[budget_gate_start..].find("<reason \"accepted\">").expect("resource reason");
        let start = budget_gate_start + reason_relative;
        let tampered_text = format!(
            "{}<reason \"tampered\">{}",
            &report_text[..start],
            &report_text[start + "<reason \"accepted\">".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered resource receipt report");
        let error = validate_report_value(&tampered_report).expect_err("tampered resource receipt fails closed");
        assert!(error.to_string().contains("unsupported Basalt resource preflight reason tampered"));
    }

    #[test]
    fn old_suite_shape_with_explicit_fixtures_uses_standard_budget() {
        let suite_value = parse_text(OLD_SHAPE_TWO_ACTOR_SUITE).expect("parse old shape suite");
        let suite = parse_suite(&suite_value).expect("parse old shape suite schema");
        assert_eq!(suite.budget.max_steps, 64);
        assert_eq!(suite.budget.max_effects, 16);
        assert!(suite.budget_explicit);
        assert!(suite.actors_explicit);
        assert!(suite.capabilities_explicit);
        let run = run_suite_value(&suite_value).expect("old shape suite should run with explicit standard budget");
        validate_report_value(&run.report_value).expect("old shape report validates with explicit budget evidence");
    }

    #[test]
    fn omitted_budget_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "default-budget" 1
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a">]>"#,
        )
        .expect("parse omitted budget suite");
        let parsed = parse_suite(&suite).expect("parse omitted budget suite schema");
        assert!(!parsed.budget_explicit);
        assert!(parsed.actors_explicit);
        assert!(parsed.capabilities_explicit);
        let error = run_suite_value(&suite).expect_err("default budget cannot execute evidence-bearing suites");
        assert!(error.to_string().contains("missing explicit budget fixture"));
    }

    #[test]
    fn omitted_actor_registry_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "inferred-actors" 1
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a">]>"#,
        )
        .expect("parse inferred actor suite");
        let parsed = parse_suite(&suite).expect("parse inferred actor suite schema");
        assert!(!parsed.actors_explicit);
        assert_eq!(parsed.actors.len(), 1);
        let error = run_suite_value(&suite).expect_err("inferred actors cannot execute evidence-bearing suites");
        assert!(error.to_string().contains("missing explicit actor registry fixture"));
    }

    #[test]
    fn explicit_empty_actor_registry_keeps_unknown_actor_failure() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "empty-registry" 1
              <actor-registry-v1 "molten.harness.actor-registry.v1" []>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a">]>"#,
        )
        .expect("parse empty actor registry suite");
        let parsed = parse_suite(&suite).expect("parse empty actor registry suite schema");
        assert!(parsed.actors_explicit);
        let error = run_suite_value(&suite).expect_err("explicit empty registry cannot cover actor steps");
        assert!(error.to_string().contains("unknown actor a"));
    }

    #[test]
    fn report_validation_rejects_legacy_inferred_actor_registry() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = super::schema::parse_report(&run.report_value).expect("parse report");
        let legacy_suite_value = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "two-actor" 7
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f "service.ready">
                <grant "producer" "assert" #f "service.ready">
                <grant "producer" "send" "consumer" #f>
                <grant "producer" "clock" #f #f>
                <grant "producer" "random" #f #f>
                <grant "producer" "retract" #f "service.ready">
              ]>
              [
                <observe "consumer" "service.ready">
                <assert "producer" "service.ready">
                <send "producer" "consumer" "hello">
                <clock "producer">
                <random "producer" 100>
                <retract "producer" "service.ready">
              ]>"#,
        )
        .expect("parse legacy inferred actor suite");
        let legacy_suite = parse_suite(&legacy_suite_value).expect("parse legacy inferred actor suite schema");
        assert!(!legacy_suite.actors_explicit);
        let legacy_report = super::schema::report_value(super::schema::ReportValueInput {
            suite: &legacy_suite,
            suite_ref: canonical_hash(&legacy_suite_value).expect("legacy suite hash"),
            initial_state_hash: report.initial_state_hash,
            final_state_hash: report.final_state_hash,
            policy_gate: report.policy_gate.expect("policy gate").value,
            capability_gate: report.capability_gate.expect("capability gate").value,
            budget_gate: report.budget_gate.expect("budget gate").value,
            observations: report.observations.iter().map(|observation| observation.value.clone()).collect(),
            effect_log: report.effect_log,
            budget: &legacy_suite.budget,
            usage: &report.budget.usage,
        });
        let error = validate_report_value(&legacy_report).expect_err("legacy inferred actor report fails validation");
        assert!(error.to_string().contains("missing explicit actor registry fixture"));
    }

    #[test]
    fn report_validation_rejects_legacy_default_budget() {
        let suite = parse_text(TWO_ACTOR_SUITE).expect("parse suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report = super::schema::parse_report(&run.report_value).expect("parse report");
        let legacy_suite_value = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "two-actor" 7
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "consumer" "native">
                <actor "producer" "native">
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "consumer" "observe" #f "service.ready">
                <grant "producer" "assert" #f "service.ready">
                <grant "producer" "send" "consumer" #f>
                <grant "producer" "clock" #f #f>
                <grant "producer" "random" #f #f>
                <grant "producer" "retract" #f "service.ready">
              ]>
              [
                <observe "consumer" "service.ready">
                <assert "producer" "service.ready">
                <send "producer" "consumer" "hello">
                <clock "producer">
                <random "producer" 100>
                <retract "producer" "service.ready">
              ]>"#,
        )
        .expect("parse legacy default budget suite");
        let legacy_suite = parse_suite(&legacy_suite_value).expect("parse legacy default budget suite schema");
        assert!(!legacy_suite.budget_explicit);
        assert!(legacy_suite.actors_explicit);
        assert!(legacy_suite.capabilities_explicit);
        let legacy_report = super::schema::report_value(super::schema::ReportValueInput {
            suite: &legacy_suite,
            suite_ref: canonical_hash(&legacy_suite_value).expect("legacy suite hash"),
            initial_state_hash: report.initial_state_hash,
            final_state_hash: report.final_state_hash,
            policy_gate: report.policy_gate.expect("policy gate").value,
            capability_gate: report.capability_gate.expect("capability gate").value,
            budget_gate: report.budget_gate.expect("budget gate").value,
            observations: report.observations.iter().map(|observation| observation.value.clone()).collect(),
            effect_log: report.effect_log,
            budget: &legacy_suite.budget,
            usage: &report.budget.usage,
        });
        let error = validate_report_value(&legacy_report).expect_err("legacy default budget report fails validation");
        assert!(error.to_string().contains("missing explicit budget fixture"));
    }

    #[test]
    fn omitted_capability_fixture_fails_closed() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "implicit-authority" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              [<clock "a">]>"#,
        )
        .expect("parse implicit authority suite");
        let parsed = parse_suite(&suite).expect("parse implicit authority suite schema");
        assert!(parsed.budget_explicit);
        assert!(parsed.actors_explicit);
        assert!(!parsed.capabilities_explicit);
        let error = run_suite_value(&suite).expect_err("implicit authority cannot execute evidence-bearing suites");
        assert!(error.to_string().contains("missing explicit capability fixture"));
    }

    #[test]
    fn suite_configured_effect_budget_fails() {
        let suite = parse_text(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "low-effect-budget" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 1 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "clock" #f #f>]>
              [<clock "a"> <clock "a">]>"#,
        )
        .expect("parse low effect budget suite");
        let error = run_suite_value(&suite).expect_err("configured effect budget should fail");
        match error {
            MoltenError::HarnessDivergence(divergence) => {
                assert_eq!(divergence.kind, "resource");
                assert_eq!(divergence.step, Some(1));
                assert_eq!(divergence.detail, "effect count exceeds budget");
            }
            other => panic!("expected resource divergence, got {other}"),
        }
    }

    #[test]
    fn report_budget_limits_must_match_embedded_suite() {
        let suite = parse_text(OLD_SHAPE_TWO_ACTOR_SUITE).expect("parse old shape suite");
        let run = run_suite_value(&suite).expect("run suite");
        let report_text = to_text(&run.report_value).expect("render report");
        let start = report_text.rfind("<limits 64 16 256 65536>").expect("report budget limits");
        let tampered_text = format!(
            "{}<limits 64 15 256 65536>{}",
            &report_text[..start],
            &report_text[start + "<limits 64 16 256 65536>".len()..]
        );
        let tampered_report = parse_text(&tampered_text).expect("parse tampered report");
        let error = validate_report_value(&tampered_report).expect_err("budget mismatch should fail");
        assert!(error.to_string().contains("report budget limits do not match embedded suite budget"));
    }

    #[test]
    fn suite_parser_rejects_unknown_step() {
        let suite = parse_text(r#"<harness-suite-v1 "molten.harness.suite.v1" "bad" 1 [<unknown>] >"#)
            .expect("parse suite text");
        let error = parse_suite(&suite).expect_err("unknown step should fail");
        assert!(error.to_string().contains("unknown harness step"));
    }
}
