
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
