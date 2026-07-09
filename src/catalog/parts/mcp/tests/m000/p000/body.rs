    use super::*;

    type ArtifactInstall = crate::artifacts::ArtifactInstall;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    #[test]
    fn readonly_list_view_search_match_catalog_core_and_bind_receipts() {
        let registry = temp_dir("catalog-mcp-readonly");
        let base = install_fixture(&registry, "schema", parse_text("<schema \"mcp\">").expect("schema"), &[], &[]);
        let doc = install_fixture(
            &registry,
            "doc",
            parse_text("<doc \"visible\">").expect("doc"),
            std::slice::from_ref(&base.artifact_ref),
            &[],
        );
        let request = mcp_request_value("catalog.search", vec![
            record("kind", vec![string("doc")]),
            record("dependency-ref", vec![string(&base.artifact_ref)]),
            record("text", vec![string("visible")]),
        ])
        .expect("request");
        let call = call(&registry, None, &request).expect("mcp call");
        assert_eq!(call.decision, "pass");
        assert!(call.catalog_receipt_ref.is_some());
        let response_text = to_text(&call.response_value).expect("response text");
        assert!(response_text.contains(&doc.artifact_ref));
        let receipt = parse_mcp_receipt(&call.receipt_value).expect("mcp receipt");
        assert_eq!(receipt.tool, "catalog.search");
    }

    #[test]
    fn chunk_store_tool_exposes_readonly_status_when_chunk_root_supplied() {
        let registry = temp_dir("catalog-mcp-chunk-registry");
        let chunks = temp_dir("catalog-mcp-chunks");
        let put = crate::chunk_store::put_bytes(&chunks, "artifact", b"aaaabbbb", 4).expect("chunk put");
        crate::chunk_store::pin_manifest(&chunks, &put.manifest_ref).expect("pin manifest");
        let request = mcp_request_value("catalog.chunk_store", Vec::new()).expect("chunk MCP request");
        let call = call_with_chunk_store(&registry, None, Some(&chunks), &request).expect("chunk MCP call");
        assert_eq!(call.decision, "pass");
        assert!(call.catalog_receipt_ref.is_some());
        let response_text = to_text(&call.response_value).expect("chunk MCP response");
        assert!(response_text.contains("chunk-store-catalog-v1"));
        assert!(response_text.contains(&put.manifest_ref));
        assert!(response_text.contains("chunk-store-pin:pinned"));
    }

    #[test]
    fn unison_named_tools_search_schema_effect_and_receipts() {
        let registry = temp_dir("catalog-mcp-unison-tools");
        let schema_ref = test_ref("schema-ref");
        let effect_ref = test_ref("effect-ref");
        let base = install_fixture(&registry, "schema", parse_text("<schema \"alias\">").expect("schema"), &[], &[]);
        let doc = crate::artifacts::install_artifact(&registry, &crate::artifacts::ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: parse_text("<doc \"alias-visible\">").expect("doc"),
            schema_refs: vec![schema_ref.clone()],
            dependency_refs: vec![base.artifact_ref.clone()],
            effect_manifest_ref: Some(effect_ref.clone()),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install doc");
        let schema_request =
            mcp_request_value("search_by_schema", vec![record("schema-ref", vec![string(&schema_ref)])])
                .expect("schema request");
        let schema_call = call(&registry, None, &schema_request).expect("schema call");
        assert!(to_text(&schema_call.response_value).expect("schema response").contains(&doc.artifact_ref));
        let effect_request =
            mcp_request_value("search_by_effect", vec![record("effect-ref", vec![string(&effect_ref)])])
                .expect("effect request");
        let effect_call = call(&registry, None, &effect_request).expect("effect call");
        assert!(to_text(&effect_call.response_value).expect("effect response").contains(&doc.artifact_ref));
        let deps_request =
            mcp_request_value("list_dependencies", vec![record("reference", vec![string(&doc.artifact_ref)])])
                .expect("deps request");
        let deps_call = call(&registry, None, &deps_request).expect("deps call");
        assert!(to_text(&deps_call.response_value).expect("deps response").contains(&base.artifact_ref));
        let receipts_request =
            mcp_request_value("view_receipts", vec![record("reference", vec![string(&doc.artifact_ref)])])
                .expect("receipts request");
        let receipts_call = call(&registry, None, &receipts_request).expect("receipts call");
        assert!(to_text(&receipts_call.response_value).expect("receipts response").contains("artifact-receipt-v1"));
        let short_request =
            mcp_request_value("short_id_resolve", vec![record("prefix", vec![string(&doc.artifact_ref[7..19])])])
                .expect("short request");
        let short_call = call(&registry, None, &short_request).expect("short call");
        assert_eq!(short_call.decision, "pass");
    }

    #[test]
    fn impact_query_tool_links_refs_redacts_hidden_and_denies_unison_compat_mutation() {
        // r[verify molten.catalog.unison_discovery_validation]
        let registry = temp_dir("catalog-mcp-impact");
        let base = install_fixture(&registry, "schema", parse_text("<schema \"impact\">").expect("schema"), &[], &[]);
        let doc = install_fixture(
            &registry,
            "doc",
            parse_text("<doc \"impact-doc\">").expect("doc"),
            std::slice::from_ref(&base.artifact_ref),
            &[],
        );
        let impact_request = mcp_request_value("impact_query", vec![
            record("reference", vec![string(&base.artifact_ref)]),
            record("transitive", vec![crate::preserves_rail::bool_value(true)]),
        ])
        .expect("impact request");
        let impact_call = call(&registry, None, &impact_request).expect("impact call");
        assert_eq!(impact_call.decision, "pass");
        let impact_text = to_text(&impact_call.response_value).expect("impact response");
        assert!(impact_text.contains("impact-query"));
        assert!(impact_text.contains(&doc.artifact_ref));
        assert!(parse_mcp_receipt(&impact_call.receipt_value)
            .expect("impact MCP receipt")
            .catalog_receipt_ref
            .is_some());

        let hidden_request = mcp_request_value("impact_query", vec![
            record("reference", vec![string(&base.artifact_ref)]),
            record("hidden-ref", vec![string(&doc.artifact_ref)]),
        ])
        .expect("hidden impact request");
        let hidden_call = call(&registry, None, &hidden_request).expect("hidden impact call");
        let hidden_text = to_text(&hidden_call.response_value).expect("hidden impact response");
        assert!(!hidden_text.contains(&doc.artifact_ref));
        assert!(hidden_text.contains("redacted"));

        let unison_mutation = mcp_request_value("ucm.update_alias", vec![record("name", vec![string("docs/main")])])
            .expect("ucm mutation request");
        let denied = call(&registry, None, &unison_mutation).expect("ucm mutation denied");
        assert_eq!(denied.decision, "deny");
        let denied_text = to_text(&denied.receipt_value).expect("denied receipt");
        assert!(denied_text.contains("mutating-tools-denied"));
        assert!(denied_text.contains("read-only-tool"));
    }

    #[test]
    fn provenance_named_tools_search_trust_state_and_decision() {
        let root = temp_dir("catalog-mcp-provenance");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        let artifact_ref = test_ref("provenance-artifact");
        let provenance_record = crate::provenance::synthetic_reviewed_record(&artifact_ref).expect("record");
        let evaluation = crate::provenance::evaluate(&crate::provenance::EvaluationInput {
            operation: "install",
            profile: "node-control",
            artifact_ref: &artifact_ref,
            provenance_values: std::slice::from_ref(&provenance_record),
            build_verification_values: &[],
            prior_diagnostics: &[],
        })
        .expect("evaluate provenance");
        crate::ledger::import_artifact(&ledger_root, &provenance_record).expect("import provenance record");
        crate::ledger::import_artifact(&ledger_root, &evaluation.receipt_value).expect("import provenance receipt");
        let record_request =
            mcp_request_value("list_provenance", vec![record("trust-state", vec![string("reviewed")])])
                .expect("provenance record request");
        let record_call = call(&registry, Some(&ledger_root), &record_request).expect("provenance record call");
        assert_eq!(record_call.decision, "pass");
        let record_text = to_text(&record_call.response_value).expect("provenance record response");
        assert!(record_text.contains("provenance:record"));
        let receipt_request = mcp_request_value("search_provenance", vec![record("decision", vec![string("pass")])])
            .expect("provenance receipt request");
        let receipt_call = call(&registry, Some(&ledger_root), &receipt_request).expect("provenance receipt call");
        assert_eq!(receipt_call.decision, "pass");
        let receipt_text = to_text(&receipt_call.response_value).expect("provenance receipt response");
        assert!(receipt_text.contains("provenance:receipt"));
    }

    #[test]
    fn replay_evidence_named_tool_searches_decision_divergence_and_refs() {
        let root = temp_dir("catalog-mcp-replay-evidence");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        let fixture = replay_case(&ledger_root);

        assert_replay_verify_search(&registry, &ledger_root, &fixture);
        assert_replay_divergence_search(&registry, &ledger_root, &fixture);
        assert_replay_rollup_search(&registry, &ledger_root);
        assert_replay_index_search(&registry, &ledger_root);
    }

    #[test]
    fn gc_named_tool_searches_audit_scope() {
        let root = temp_dir("catalog-mcp-retention-gc");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        let retention_root = root.join("retention");
        let fixture = gc_audit_fixture(&retention_root, "catalog-mcp-retention-gc", "chunk-gc");
        crate::ledger::import_artifact(&ledger_root, &fixture.audit.value).expect("import retention GC audit");

        let request = mcp_request_value("search_retention_gc", vec![
            record("stage", vec![string("audit")]),
            record("object-ref", vec![string(&fixture.object_ref)]),
            record("subsystem", vec![string("chunk-gc")]),
            record("execution-ref", vec![string(&fixture.execution_ref)]),
        ])
        .expect("retention GC search request");
        let call = call(&registry, Some(&ledger_root), &request).expect("retention GC search call");
        assert_eq!(call.decision, "pass");
        let text = to_text(&call.response_value).expect("retention GC search response");
        assert!(text.contains("retention-gc:audit"));
        assert!(text.contains(&fixture.execution_ref));
    }

    #[test]
    fn hidden_refs_stay_hidden_and_redacted_view_is_default() {
        let registry = temp_dir("catalog-mcp-hidden");
        let secret =
            install_fixture(&registry, "doc", parse_text("<doc <secret \"hidden-value\">>").expect("secret"), &[], &[]);
        let hidden_request = mcp_request_value("catalog.search", vec![
            record("text", vec![string("hidden-value")]),
            record("hidden-ref", vec![string(&secret.artifact_ref)]),
        ])
        .expect("hidden request");
        let hidden = call(&registry, None, &hidden_request).expect("hidden call");
        assert_eq!(hidden.decision, "pass");
        assert!(!to_text(&hidden.response_value).expect("hidden response").contains(&secret.artifact_ref));
        let view_request = mcp_request_value("catalog.view", vec![
            record("reference", vec![string(&secret.artifact_ref)]),
            record("payload", vec![crate::preserves_rail::bool_value(true)]),
        ])
        .expect("view request");
        let viewed = call(&registry, None, &view_request).expect("view call");
        let text = to_text(&viewed.response_value).expect("view response");
        assert!(text.contains("redaction-marker-v1"));
        assert!(!text.contains("hidden-value"));
    }

    #[test]
    fn short_id_ambiguity_denies_and_mutating_tools_fail_closed() {
        let registry = temp_dir("catalog-mcp-deny");
        let mut refs_by_first_hex = Vec::<(char, String)>::with_capacity(32);
        let mut shared_prefix = None;
        for index in 0..32 {
            let installed =
                install_fixture(&registry, "doc", parse_text(&format!("<doc {index}>")).expect("doc"), &[], &[]);
            let first_hex = installed.artifact_ref.as_bytes()[7] as char;
            if refs_by_first_hex.iter().any(|(hex, _)| *hex == first_hex) {
                shared_prefix = Some(installed.artifact_ref[7..8].to_string());
                break;
            }
            refs_by_first_hex.push((first_hex, installed.artifact_ref));
        }
        let short = mcp_request_value("catalog.short_id", vec![
            record("prefix", vec![string(shared_prefix.expect("fixture collision within hex alphabet"))]),
            record("min-length", vec![crate::preserves_rail::u64_value(0)]),
        ])
        .expect("short request");
        let short_call = call(&registry, None, &short).expect("short call");
        assert_eq!(short_call.decision, "deny");
        assert!(to_text(&short_call.response_value).expect("short response").contains("ambiguous"));
        let malformed_short = mcp_request_value("catalog.short_id", vec![record("prefix", vec![string("blake3:")])])
            .expect("malformed short request");
        let malformed_call = call(&registry, None, &malformed_short).expect("malformed short call");
        assert_eq!(malformed_call.decision, "deny");
        assert!(
            to_text(&malformed_call.response_value)
                .expect("malformed response")
                .contains("malformed full content ref")
        );
        let mutate = mcp_request_value("catalog.install", vec![record("kind", vec![string("doc")])]).expect("mutate");
        let denied = call(&registry, None, &mutate).expect("mutating call denied");
        assert_eq!(denied.decision, "deny");
        assert!(to_text(&denied.receipt_value).expect("denial receipt").contains("mutating-tools-denied"));
    }

    #[hegel::test(test_cases = 10)]
    fn hegel_mcp_calls_are_deterministic_and_readonly(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let registry = temp_dir("catalog-mcp-hegel");
        let label = format!("payload-{salt}");
        install_fixture(&registry, "doc", record("doc", vec![string(&label)]), &[], &[]);
        let request = mcp_request_value("catalog.list", vec![record("kind", vec![string("doc")])]).expect("request");
        let first = call(&registry, None, &request).expect("first call");
        let second = call(&registry, None, &request).expect("second call");
        assert_eq!(first.response_ref, second.response_ref);
        let denied = call(&registry, None, &mcp_request_value("catalog.delete", Vec::new()).expect("mutating request"))
            .expect("denied mutating");
        assert_eq!(denied.decision, "deny");
    }

    struct ReplayCase {
        expected_report_ref: String,
        final_state_ref: String,
        expected_ref: String,
        actual_ref: String,
        handler_profile_ref: String,
    }

    fn replay_case(ledger_root: &Path) -> ReplayCase {
        let fixture = ReplayCase {
            expected_report_ref: test_ref("replay-expected-report"),
            final_state_ref: test_ref("replay-final-state"),
            expected_ref: test_ref("replay-expected-effect"),
            actual_ref: test_ref("replay-actual-effect"),
            handler_profile_ref: test_ref("replay-handler-profile"),
        };
        let verify = replay_verify_record(&fixture, &test_ref("replay-actual-report"));
        let divergence = replay_divergence_record(&fixture);
        let rollup = replay_rollup(&verify);
        let index = replay_index(&verify, &rollup);
        crate::ledger::import_artifact(ledger_root, &verify).expect("import replay verify");
        crate::ledger::import_artifact(ledger_root, &divergence).expect("import first divergence");
        crate::ledger::import_artifact(ledger_root, &rollup.value).expect("import replay rollup");
        crate::ledger::import_artifact(ledger_root, &index.value).expect("import replay index");
        fixture
    }

    fn replay_verify_record(fixture: &ReplayCase, actual_report_ref: &str) -> IoValue {
        record("deterministic-replay-verify-v1", vec![
            string(crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
            string("deny"),
            record("expected-report-ref", vec![string(&fixture.expected_report_ref)]),
            record("actual-report-ref", vec![string(actual_report_ref)]),
            record("final-state-ref", vec![string(&fixture.final_state_ref)]),
            record("divergence", vec![string("effect-response")]),
            checks_value(&["evidence-only", "no-authority-grant"]),
        ])
    }

    fn replay_divergence_record(fixture: &ReplayCase) -> IoValue {
        record("deterministic-first-divergence-v1", vec![
            string(crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA),
            record("kind", vec![string("effect-response")]),
            record("turn-id", vec![string("turn:0001")]),
            record("actor-id", vec![string("actor:helper")]),
            record("log-position", vec![string("0")]),
            record("handler-profile-ref", vec![string(&fixture.handler_profile_ref)]),
            record("expected-ref", vec![string(&fixture.expected_ref)]),
            record("actual-ref", vec![string(&fixture.actual_ref)]),
            checks_value(&["evidence-only", "first-divergence"]),
        ])
    }
