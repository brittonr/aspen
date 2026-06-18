    #[test]
    fn cli_receipt_ledger_and_repro_exchange_commands_work() {
        let dir = temp_dir("receipt-ledger-iroh");
        let suite = dir.join("suite.preserves");
        let report = dir.join("report.preserves");
        let gate_receipt = dir.join("gate.preserves");
        write_file(
            &suite,
            r#"<harness-suite-v1 "molten.harness.suite.v1" "cli-evidence" 1
              <budget-v1 "molten.harness.budget.v1" <limits 16 4 64 65536>>
              <actor-registry-v1 "molten.harness.actor-registry.v1" [<actor "a" "native">]>
              <capabilities-v1 "molten.harness.capabilities.v1" [<grant "a" "assert" #f "ready">]>
              [<assert "a" "ready">]>"#,
        )
        .expect("write suite");
        run_test_command(TestCommand::Run {
            suite: suite.clone(),
            report_out: Some(report.clone()),
        })
        .expect("run suite");
        run_gate_command(GateCommand::Check {
            artifact: report.clone(),
            failure_out: None,
            receipt_out: Some(gate_receipt.clone()),
        })
        .expect("write gate receipt");

        let signed = dir.join("signed.preserves");
        run_receipt_command(ReceiptCommand::Sign {
            receipt: gate_receipt.clone(),
            out: signed.clone(),
            signer: "local-signer".to_string(),
            purpose: PASS_EVIDENCE_PURPOSE.to_string(),
            trust_root: "local-trust-root".to_string(),
            key: "local-dev-key".to_string(),
            parents: Vec::new(),
        })
        .expect("sign receipt");
        run_receipt_command(ReceiptCommand::Verify {
            signed_receipt: signed.clone(),
            purpose: PASS_EVIDENCE_PURPOSE.to_string(),
            trust_root: "local-trust-root".to_string(),
            key: "local-dev-key".to_string(),
            key_ledger: None,
            key_ref: None,
            key_id: None,
            signer: Some("local-signer".to_string()),
            subject_ref: None,
        })
        .expect("verify signed receipt");

        let ledger = dir.join("ledger");
        let ledger_import_receipt = dir.join("ledger-import.preserves");
        run_ledger_command(LedgerCommand::Import {
            artifact: report.clone(),
            ledger: ledger.clone(),
            receipt_out: Some(ledger_import_receipt),
        })
        .expect("ledger import");
        let report_value = read_preserves_file(&report).expect("read report");
        let report_ref = molten::preserves_rail::canonical_hash(&report_value).expect("report ref");
        run_ledger_command(LedgerCommand::Pin {
            artifact_ref: report_ref.clone(),
            ledger: ledger.clone(),
        })
        .expect("ledger pin");
        run_ledger_command(LedgerCommand::Gc {
            ledger: ledger.clone(),
            dry_run: false,
            apply_refs: Vec::new(),
            retention: retention_cli_args("ledger-gc"),
            receipt_out: Some(dir.join("ledger-gc.preserves")),
        })
        .expect("ledger gc");
        run_ledger_command(LedgerCommand::Export {
            artifact_ref: report_ref,
            ledger,
            out: dir.join("report-export.preserves"),
            receipt_out: Some(dir.join("ledger-export.preserves")),
        })
        .expect("ledger export");

        let repro = dir.join("repro");
        run_repro_command(ReproCommand::Export {
            report: report.clone(),
            out: repro.clone(),
            profile: "deny-sensitive".to_string(),
            failure_out: None,
        })
        .expect("export repro");
        let refs = repro.join("refs.preserves");
        let store = dir.join("iroh-store");
        let publish_receipt = dir.join("publish.preserves");
        run_repro_command(ReproCommand::Publish {
            bundle: refs.clone(),
            store: store.clone(),
            node: "node:local".to_string(),
            receipt_out: Some(publish_receipt),
            failure_out: None,
        })
        .expect("publish repro");
        let bundle_ref = molten::preserves_rail::canonical_hash(&read_preserves_file(&refs).expect("read bundle"))
            .expect("bundle ref");
        run_repro_command(ReproCommand::Fetch {
            ticket: format!("iroh-local:{bundle_ref}"),
            store,
            out: Some(dir.join("fetched.preserves")),
            ledger: None,
            expected_bundle_ref: Some(bundle_ref),
            peer: "peer:local".to_string(),
            receipt_out: Some(dir.join("fetch.preserves")),
            failure_out: None,
        })
        .expect("fetch repro");
    }
