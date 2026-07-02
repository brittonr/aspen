    use super::*;

    type MoltenError = crate::error::MoltenError;

    fn canonical_bytes(value: &preserves::IOValue) -> crate::error::Result<Vec<u8>> {
        crate::preserves_rail::canonical_bytes(value)
    }

    fn canonical_hash(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(value)
    }

    fn effect_log_from_observations(
        observations: &[schema::Observation],
    ) -> crate::error::Result<Vec<schema::EffectLogEntry>> {
        schema::effect_log_from_observations(observations)
    }

    fn parse_report(value: &preserves::IOValue) -> crate::error::Result<schema::Report> {
        schema::parse_report(value)
    }

    fn parse_text(source: &str) -> crate::error::Result<preserves::IOValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn run_suite_with_effect_log(
        suite: &schema::Suite,
        effect_log: &[schema::EffectLogEntry],
    ) -> crate::error::Result<runner::HarnessRun> {
        runner::run_suite_with_effect_log(suite, effect_log)
    }

    fn snapshot_value(snapshot: &core::RuntimeSnapshot) -> preserves::IOValue {
        schema::snapshot_value(snapshot)
    }

    fn to_text(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::to_text(value)
    }

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
