    #[test]
    fn runtime_config_command_accepts_typed_config_path() {
        let dir = temp_dir("runtime-config");
        let config = dir.join("runtime.json");
        write_file(
            &config,
            r#"{
                "source_language": "nickel",
                "actors": [{ "id": "actor:consumer", "kind": "native" }],
                "subscriptions": [{ "actor": "actor:consumer", "subject_preserves": "\"service.ready\"" }]
            }"#,
        )
        .expect("write config");

        run_runtime_command(RuntimeCommand::Config { config }).expect("runtime config command");
    }

    #[test]
    fn cli_run_writes_canonical_failure_file() {
        let dir = temp_dir("run-failure");
        let suite = dir.join("bad.preserves");
        let failure = dir.join("failure.preserves");
        write_file(
            &suite,
            r#"<harness-suite-v1 "molten.harness.suite.v1" "bad" 1
              <budget-v1 "molten.harness.budget.v1" <limits 64 16 256 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "producer" "native">]>
              [<send "producer" "missing" "hello">]>"#,
        )
        .expect("write suite");

        let error = run_test_command(TestCommand::Run {
            suite,
            report_out: Some(failure.clone()),
        })
        .expect_err("run should fail");
        assert!(error.to_string().contains("unknown actor missing"));
        let failure_value = read_preserves_file(&failure).expect("read failure");
        let failure = parse_failure(&failure_value).expect("parse failure");
        assert_eq!(failure.phase, "preflight");
        assert_eq!(failure.kind, "invalid-harness");
    }

    #[test]
    fn cli_gate_rejects_failure_artifact_with_canonical_failure() {
        let dir = temp_dir("gate-failure");
        let failure_artifact = dir.join("input.failure.preserves");
        let gate_failure = dir.join("gate.failure.preserves");
        let synthetic = failure_value("preflight", &MoltenError::invalid_harness("synthetic"), Vec::new());
        write_file(&failure_artifact, &to_text(&synthetic).expect("render failure")).expect("write failure");

        let error = run_gate_command(GateCommand::Check {
            artifact: failure_artifact,
            failure_out: Some(gate_failure.clone()),
            receipt_out: None,
        })
        .expect_err("gate should reject failure evidence");
        assert!(error.to_string().contains("cannot satisfy pass evidence gate"));
        let failure_value = read_preserves_file(&gate_failure).expect("read gate failure");
        let failure = parse_failure(&failure_value).expect("parse gate failure");
        assert_eq!(failure.phase, "validate");
        assert_eq!(failure.kind, "invalid-harness");
    }

    #[test]
    fn cli_repro_export_accepts_failure_artifact() {
        let dir = temp_dir("failure-repro");
        let failure_artifact = dir.join("input.failure.preserves");
        let out = dir.join("bundle");
        let synthetic = failure_value("execute", &MoltenError::invalid_harness("synthetic"), Vec::new());
        write_file(&failure_artifact, &to_text(&synthetic).expect("render failure")).expect("write failure");

        run_repro_command(ReproCommand::Export {
            report: failure_artifact,
            out: out.clone(),
            profile: "deny-sensitive".to_string(),
            failure_out: None,
        })
        .expect("export failure repro");
        let bundle = read_preserves_file(&out.join("refs.preserves")).expect("read refs");
        let parsed = parse_repro_bundle(&bundle).expect("parse bundle");
        assert_eq!(parsed.kind, molten::harness::HarnessReproBundleKind::Failure);
        assert!(out.join("failure.preserves").exists());
        assert!(out.join("commands.txt").exists());

        let verify_failure = dir.join("verify.failure.preserves");
        let verify_error = run_repro_command(ReproCommand::Verify {
            bundle: out.join("refs.preserves"),
            failure_out: Some(verify_failure.clone()),
            receipt_out: None,
        })
        .expect_err("failure repro verify should fail");
        assert!(verify_error.to_string().contains("diagnostic-only"));
        let verify_failure_value = read_preserves_file(&verify_failure).expect("read verify failure");
        let verify_failure = parse_failure(&verify_failure_value).expect("parse verify failure");
        assert_eq!(verify_failure.phase, "verify");

        let unpack_failure = dir.join("unpack.failure.preserves");
        let unpack_error = run_repro_command(ReproCommand::Unpack {
            bundle: out.join("refs.preserves"),
            out: dir.join("unpacked-failure"),
            reveal_receipts: Vec::new(),
            failure_out: Some(unpack_failure.clone()),
        })
        .expect_err("failure repro unpack should fail");
        let unpack_error_message = unpack_error.to_string();
        let expected_unpack_messages = ["diagnostic-only", "embedded report"];
        assert!(
            expected_unpack_messages.iter().any(|message| unpack_error_message.contains(message)),
            "unexpected unpack error: {unpack_error_message}"
        );
        let unpack_failure_value = read_preserves_file(&unpack_failure).expect("read unpack failure");
        let unpack_failure = parse_failure(&unpack_failure_value).expect("parse unpack failure");
        assert_eq!(unpack_failure.phase, "unpack");
    }
