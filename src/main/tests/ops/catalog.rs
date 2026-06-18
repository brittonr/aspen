    #[test]
    fn cli_catalog_commands_work() {
        let dir = temp_dir("catalog-cli");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let base_payload = dir.join("catalog-base.preserves");
        let dep_payload = dir.join("catalog-dependent.preserves");
        let base_out = dir.join("catalog-base-artifact.preserves");
        let dep_out = dir.join("catalog-dependent-artifact.preserves");
        let list_receipt = dir.join("catalog-list-receipt.preserves");
        let view_receipt = dir.join("catalog-view-receipt.preserves");
        write_file(&base_payload, r#"<schema "catalog-base">"#).expect("write catalog base payload");
        write_file(&dep_payload, r#"<doc "catalog-text" ["searchable"]>"#).expect("write catalog dep payload");
        run_artifact_command(ArtifactCommand::Install {
            payload: base_payload,
            registry: registry.clone(),
            kind: "schema".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(base_out.clone()),
            receipt_out: Some(dir.join("catalog-base-install-receipt.preserves")),
        })
        .expect("install catalog base");
        let base =
            artifacts::parse_artifact_value(&read_preserves_file(&base_out).expect("read base")).expect("parse base");
        run_artifact_command(ArtifactCommand::Install {
            payload: dep_payload,
            registry: registry.clone(),
            kind: "doc".to_string(),
            dependencies: vec![base.artifact_ref.clone()],
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(dep_out.clone()),
            receipt_out: Some(dir.join("catalog-dep-install-receipt.preserves")),
        })
        .expect("install catalog dependent");
        let dep =
            artifacts::parse_artifact_value(&read_preserves_file(&dep_out).expect("read dep")).expect("parse dep");
        ledger::import_artifact(&ledger_root, &dep.value).expect("import dep artifact to ledger");
        run_catalog_command(CatalogCommand::List {
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            kind: Some("doc".to_string()),
            hidden_refs: Vec::new(),
            receipt_out: Some(list_receipt.clone()),
        })
        .expect("catalog list");
        run_catalog_command(CatalogCommand::View {
            reference: dep.artifact_ref.clone(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            payload_inclusion_enabled: true,
            redaction_enabled: true,
            hidden_refs: Vec::new(),
            receipt_out: Some(view_receipt.clone()),
        })
        .expect("catalog view");
        run_catalog_command(CatalogCommand::Search {
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            artifact_kind: Some("doc".to_string()),
            ledger_kind: None,
            schema_ref: None,
            structural_fingerprint: None,
            effect_ref: None,
            policy_ref: None,
            capability_ref: None,
            evidence_ref: None,
            dependency_ref: Some(base.artifact_ref.clone()),
            dependent_ref: None,
            receipt_operation: None,
            receipt_decision: None,
            transcript_status: None,
            upgrade_status: None,
            text: Some("searchable".to_string()),
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            dependent_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-search-receipt.preserves")),
        })
        .expect("catalog search");
        run_catalog_command(CatalogCommand::Deps {
            reference: dep.artifact_ref.clone(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            transitive: false,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-deps-receipt.preserves")),
        })
        .expect("catalog deps");
        run_catalog_command(CatalogCommand::Dependents {
            reference: base.artifact_ref.clone(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            transitive: false,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-dependents-receipt.preserves")),
        })
        .expect("catalog dependents");
        run_catalog_command(CatalogCommand::ShortId {
            prefix: dep.artifact_ref[7..19].to_string(),
            registry: registry.clone(),
            ledger: Some(ledger_root.clone()),
            min_length: 8,
            hidden_refs: Vec::new(),
            receipt_out: Some(dir.join("catalog-short-id-receipt.preserves")),
        })
        .expect("catalog short id");
        let mcp_request = dir.join("catalog-mcp-request.preserves");
        let mcp_response = dir.join("catalog-mcp-response.preserves");
        let mcp_receipt = dir.join("catalog-mcp-receipt.preserves");
        write_file(
            &mcp_request,
            &to_text(
                &catalog_mcp::mcp_request_value("catalog.search", vec![
                    record("kind", vec![string("doc")]),
                    record("dependency-ref", vec![string(&base.artifact_ref)]),
                    record("text", vec![string("searchable")]),
                ])
                .expect("mcp request"),
            )
            .expect("render mcp request"),
        )
        .expect("write mcp request");
        run_catalog_command(CatalogCommand::McpCall {
            request: mcp_request,
            registry,
            ledger: Some(ledger_root),
            chunks: None,
            out: Some(mcp_response.clone()),
            receipt_out: Some(mcp_receipt.clone()),
        })
        .expect("catalog mcp call");
        assert!(fs::read_to_string(&mcp_response).expect("read mcp response").contains(&dep.artifact_ref));
        run_catalog_command(CatalogCommand::Show { artifact: mcp_receipt }).expect("catalog show MCP receipt");
        run_catalog_command(CatalogCommand::Show { artifact: list_receipt }).expect("catalog show receipt");
        run_catalog_command(CatalogCommand::Show { artifact: view_receipt }).expect("catalog show view receipt");
    }

    #[test]
    fn cli_dogfood_local_node_commands_work() {
        let dir = temp_dir("dogfood-cli");
        let state_root = dir.join("state");
        let report = dir.join("dogfood-report.preserves");
        let release_gate = dir.join("release-gate.preserves");
        let replay_verify = dir.join("replay-verify.preserves");
        let replay_index = dir.join("replay-evidence-index.preserves");
        run_dogfood_command(DogfoodCommand::LocalNode {
            state_root: state_root.clone(),
            out: report.clone(),
            release_gate_out: Some(release_gate.clone()),
            replay_verify_out: Some(replay_verify.clone()),
            replay_index_out: Some(replay_index.clone()),
        })
        .expect("dogfood local node");
        let report_value = read_preserves_file(&report).expect("read dogfood report");
        let parsed = operator_dogfood::parse_dogfood_report(&report_value).expect("parse dogfood report");
        assert_eq!(parsed.decision, "pass");
        assert!(fs::read_to_string(&release_gate).expect("read release gate").contains("release-gate-receipt-v1"));
        assert!(
            fs::read_to_string(&replay_verify)
                .expect("read replay verify")
                .contains("deterministic-replay-verify-v1")
        );
        assert!(
            fs::read_to_string(&replay_index)
                .expect("read replay index")
                .contains("deterministic-replay-index-v1")
        );
        let ledger_root = state_root.join("ledger");
        run_receipts_command(ReceiptsCommand::List {
            ledger: ledger_root.clone(),
        })
        .expect("receipts list");
        run_receipts_command(ReceiptsCommand::Show {
            receipt_ref: parsed.report_ref.clone(),
            ledger: ledger_root.clone(),
        })
        .expect("receipts show dogfood report");
        run_receipts_command(ReceiptsCommand::Validate {
            receipt_ref: parsed.report_ref.clone(),
            ledger: ledger_root.clone(),
        })
        .expect("receipts validate dogfood report");
        let exported_report = dir.join("exported-dogfood-report.preserves");
        run_receipts_command(ReceiptsCommand::Export {
            receipt_ref: parsed.report_ref.clone(),
            ledger: ledger_root,
            out: exported_report.clone(),
            receipt_out: Some(dir.join("receipts-export.preserves")),
        })
        .expect("receipts export dogfood report");
        assert_eq!(
            canonical_hash(&read_preserves_file(&exported_report).expect("exported dogfood report"))
                .expect("exported ref"),
            parsed.report_ref
        );
        fs::write(
            dir.join("dogfood-summary.txt"),
            format!(
                "dogfood local-node decision=pass report={} release-gate={}\n",
                parsed.report_ref,
                canonical_hash(&read_preserves_file(&release_gate).expect("release gate value")).expect("release ref")
            ),
        )
        .expect("write summary");
        fs::write(dir.join("after-nextest.txt"), "/nix/store/test-molten-nextest\n").expect("write nextest marker");
        let nix_evidence = dir.join("nix-dogfood-evidence.preserves");
        let nix_verify = dir.join("nix-dogfood-verify.preserves");
        run_dogfood_command(DogfoodCommand::NixReleaseExport {
            output_path: dir.clone(),
            out: nix_evidence.clone(),
        })
        .expect("dogfood nix release export");
        run_dogfood_command(DogfoodCommand::NixReleaseVerify {
            output_path: dir.clone(),
            evidence: nix_evidence.clone(),
            receipt_out: nix_verify.clone(),
        })
        .expect("dogfood nix release verify");
        let verify_value = read_preserves_file(&nix_verify).expect("read nix verify");
        let verify = operator_dogfood::parse_nix_dogfood_verify_receipt(&verify_value).expect("parse nix verify");
        assert_eq!(verify.decision, "pass");
        fs::write(dir.join("after-nextest.txt"), "/nix/store/stale-molten-nextest\n").expect("tamper nextest marker");
        let stale_verify = dir.join("nix-dogfood-verify-stale.preserves");
        run_dogfood_command(DogfoodCommand::NixReleaseVerify {
            output_path: dir.clone(),
            evidence: nix_evidence.clone(),
            receipt_out: stale_verify.clone(),
        })
        .expect("dogfood nix release verify stale marker");
        let stale_verify_value = read_preserves_file(&stale_verify).expect("read stale nix verify");
        let stale_verify_receipt =
            operator_dogfood::parse_nix_dogfood_verify_receipt(&stale_verify_value).expect("parse stale nix verify");
        assert_eq!(stale_verify_receipt.decision, "deny");
        assert!(
            stale_verify_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("nextest-marker-ref mismatch"))
        );
        fs::write(dir.join("dogfood-report.preserves"), "<tampered-dogfood-report>\n").expect("tamper report");
        let tampered_verify = dir.join("nix-dogfood-verify-tampered.preserves");
        run_dogfood_command(DogfoodCommand::NixReleaseVerify {
            output_path: dir.clone(),
            evidence: nix_evidence.clone(),
            receipt_out: tampered_verify.clone(),
        })
        .expect("dogfood nix release verify tampered report");
        let tampered_verify_value = read_preserves_file(&tampered_verify).expect("read tampered nix verify");
        let tampered_verify_receipt = operator_dogfood::parse_nix_dogfood_verify_receipt(&tampered_verify_value)
            .expect("parse tampered nix verify");
        assert_eq!(tampered_verify_receipt.decision, "deny");
        assert!(
            tampered_verify_receipt
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("Nix dogfood output observation failed"))
        );
        fs::write(dir.join("dogfood-report.preserves"), to_text(&report_value).expect("report text"))
            .expect("restore report");
        run_dogfood_command(DogfoodCommand::Show { artifact: report }).expect("dogfood show report");
        run_dogfood_command(DogfoodCommand::Show { artifact: release_gate }).expect("dogfood show gate");
        run_dogfood_command(DogfoodCommand::Show { artifact: nix_evidence }).expect("dogfood show nix evidence");
        run_dogfood_command(DogfoodCommand::Show { artifact: nix_verify }).expect("dogfood show nix verify");
    }
