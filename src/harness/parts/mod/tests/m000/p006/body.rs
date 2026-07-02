
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
        adapter_negative_security_preflights_fail_closed();
        remote_proxy_negative_security_preflights_fail_closed();
    }

    fn adapter_negative_security_preflights_fail_closed() {
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
    }

    fn remote_proxy_negative_security_preflights_fail_closed() {
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
        let gate = check_value(&run.report_value).expect("gate cross-kind conformance report");
        let receipt = parse_receipt(&receipt_value(&gate)).expect("parse conformance gate receipt");
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
