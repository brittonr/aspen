    // Wasm harness determinism: NaN canonicalization and the synchronous single-call limit.

    fn single_assert_wasm_suite(name: &str, module_hex: &str, allowed: &str) -> preserves::IOValue {
        parse_text(&format!(
            r#"<harness-suite-v1 "molten.harness.suite.v1" "{name}" 5
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [
                <actor "module" "wasm" <wasm-executor-v1 "molten.runtime.wasm-executor.v1"
                  <module-hex "{module_hex}">
                  <wit "molten:hostcalls/runtime-spine@1.0.0">
                  <allowed-hostcalls [{allowed}]>>>
              ]>
              <capabilities-v1 "molten.harness.capabilities.v1" [
                <grant "module" "assert" #f "service.ready">
              ]>
              [<assert "module" "service.ready">]>"#,
        ))
        .expect("parse single-assert wasm suite")
    }

    fn wasm_abi_assert_body_hex(body: &str) -> String {
        let wat = format!(
            r#"(module
  (import "molten:hostcall" "assert" (func $assert (param i32 i32) (result i64)))
  (import "molten:hostcall" "send" (func $send (param i32 i32) (result i64)))
  (memory (export "memory") 1)
  (func (export "molten_alloc") (param $len i32) (result i32)
    i32.const 4096)
  (func (export "molten_dealloc") (param i32) (param i32))
  (func (export "molten_hostcall_assert") (param $ptr i32) (param $len i32) (result i64)
{body}))"#
        );
        bytes_to_hex(&wat::parse_str(wat).expect("compile single-call wat"))
    }

    #[test]
    fn wasm_second_hostcall_is_denied_before_dispatch() {
        // r[verify molten.wasm.harness_nan_canonicalization_and_sync_call_limit]
        // Each guest loops forever after its extra call, so only a synchronous
        // denial inside the trampoline can stop it before fuel runs out.
        let twice = wasm_abi_assert_body_hex(
            "    local.get $ptr\n    local.get $len\n    call $assert\n    drop\n    local.get $ptr\n    local.get $len\n    call $assert\n    drop\n    (loop $again br $again)\n    i64.const 0",
        );
        let other = wasm_abi_assert_body_hex(
            "    local.get $ptr\n    local.get $len\n    call $send\n    drop\n    (loop $again br $again)\n    i64.const 0",
        );
        let core_twice = bytes_to_hex(
            &wat::parse_str(
                r#"(module
  (import "molten:hostcall" "assert" (func $assert))
  (func (export "molten_hostcall_assert")
    call $assert
    call $assert
    (loop $again br $again)))"#,
            )
            .expect("compile core single-call wat"),
        );
        for (name, module_hex, allowed, denied) in [
            ("wasm-abi-second-call", twice, r#""assert" "send""#, "hostcall assert was denied before dispatch"),
            ("wasm-abi-other-call", other, r#""assert" "send""#, "hostcall send was denied before dispatch"),
            ("wasm-core-second-call", core_twice, r#""assert""#, "hostcall assert was denied before dispatch"),
        ] {
            let suite = single_assert_wasm_suite(name, &module_hex, allowed);
            let error = run_suite_value(&suite).expect_err("an extra hostcall fails closed");
            assert!(error.to_string().contains(denied), "{name}: {error}");
        }

        let once = single_assert_wasm_suite("wasm-abi-single-call", &wasm_abi_module_hex(&["assert"]), r#""assert""#);
        run_suite_value(&once).expect("exactly one hostcall is admitted");
    }

    #[test]
    fn wasm_nan_results_are_canonical_and_replay_identically() {
        // r[verify molten.wasm.harness_nan_canonicalization_and_sync_call_limit]
        // 0/0 computed at run time from guest memory: x86-64 produces the
        // negative default NaN unless the engine canonicalizes it. The guest
        // returns valid output only for the canonical f32 and f64 NaN bits.
        let output = parse_text("<wasm-output \"ok\">").expect("parse wasm output");
        let output_bytes = canonical_bytes(&output).expect("encode wasm output");
        let canonical_descriptor = (2048u64 << 32) | output_bytes.len() as u64;
        let other_descriptor = (2560u64 << 32) | 2;
        let output_data = wat_data(&output_bytes);
        let wat = format!(
            r#"(module
  (import "molten:hostcall" "assert" (func $assert (param i32 i32) (result i64)))
  (memory (export "memory") 1)
  (data (i32.const 2048) "{output_data}")
  (data (i32.const 2560) "\ff\00")
  (data (i32.const 3072) "\00\00\00\00\00\00\00\00")
  (func (export "molten_alloc") (param $len i32) (result i32)
    i32.const 4096)
  (func (export "molten_dealloc") (param i32) (param i32))
  (func (export "molten_hostcall_assert") (param $ptr i32) (param $len i32) (result i64)
    local.get $ptr
    local.get $len
    call $assert
    drop
    (f32.div (f32.load (i32.const 3072)) (f32.load (i32.const 3076)))
    i32.reinterpret_f32
    i32.const 0x7fc00000
    i32.eq
    (f64.div (f64.load (i32.const 3072)) (f64.load (i32.const 3072)))
    i64.reinterpret_f64
    i64.const 0x7ff8000000000000
    i64.eq
    i32.and
    (if (result i64)
      (then i64.const {canonical_descriptor})
      (else i64.const {other_descriptor}))))"#
        );
        let module_hex = bytes_to_hex(&wat::parse_str(wat).expect("compile nan wat"));
        let suite = single_assert_wasm_suite("wasm-nan-canonical", &module_hex, r#""assert""#);
        let first = run_suite_value(&suite).expect("canonical NaN bits select the valid output");
        let second = run_suite_value(&suite).expect("replayed run");
        assert_eq!(first.report_value, second.report_value);
    }
