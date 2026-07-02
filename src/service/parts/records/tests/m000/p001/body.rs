
    #[test]
    fn service_record_variants_roundtrip() {
        assert_variants(&case());
    }

    #[test]
    fn ledger_and_catalog_classify_service_records() {
        let dir = temp_dir("service-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let manifest = service_manifest_value(&manifest_input()).expect("manifest");
        let imported = crate::ledger::import_artifact(&ledger_root, &manifest).expect("ledger import");
        assert_eq!(imported.artifact_kind, "service-manifest");
        let listed = crate::catalog::list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("service-manifest".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list service manifest");
        assert_eq!(listed.items.len(), 1);
        let rendered = to_text(&listed.value).expect("render catalog result");
        assert!(rendered.contains("ledger-kind:service-manifest"));
        let request = crate::catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "service-manifest",
        )])])
        .expect("MCP request");
        let mcp = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list service manifest");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render MCP response").contains("service-manifest"));
    }

    #[test]
    fn service_summary_redacts_secret_markers_and_is_not_parseable_evidence() {
        let lifecycle = parse_text(
            "<service-lifecycle-receipt-v1 \"molten.service.lifecycle-receipt.v1\" \
             <operation \"fail\"> <decision \"diagnostic\"> <service-id \"svc:web\"> \
             <manifest <none>> <status <none>> <authority []> <resource []> <effect-profile []> \
             <supervision []> <diagnostics [\"<secret do-not-render>\"]> \
             <checks [<check \"canonical-receipt\" \"pass\"> <check \"decision-before-side-effects\" \"pass\"> \
             <check \"text-not-evidence\" \"pass\">]>>",
        )
        .expect("parse secret lifecycle");
        let summary = service_summary(&lifecycle).expect("service summary");
        assert!(summary.contains("redacted=true"));
        assert!(!summary.contains("do-not-render"));
        let summary_value = parse_text(&format!("\"{summary}\"")).expect("parse summary string");
        assert!(parse_service_record(&summary_value).is_err());
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_service_manifest_refs_are_stable_and_bounds_fail_closed(tc: TestCase) {
        let dependency_count = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(4));
        let dependency_count_usize = usize::try_from(dependency_count).expect("bounded dependency count");
        let mut input = manifest_input();
        input.dependencies = (0..dependency_count_usize).map(|index| format!("svc:dep-{index}")).collect::<Vec<_>>();
        let value = service_manifest_value(&input).expect("manifest value");
        let first_ref = canonical_hash(&value).expect("first ref");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(first_ref, canonical_hash(&reparsed).expect("second ref"));
        let mut too_many = input;
        too_many.dependencies = (0..=MAX_SERVICE_IDS).map(|index| format!("svc:overflow-{index}")).collect::<Vec<_>>();
        assert!(service_manifest_value(&too_many).is_err());
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
