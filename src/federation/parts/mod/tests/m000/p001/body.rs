
    #[test]
    fn receiver_policy_denial_does_not_import_remote_resource() {
        let source = temp_dir("federation-deny-source");
        let destination = temp_dir("federation-deny-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        let allowed = vec!["chain-link".to_string()];
        let pull = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &allowed,
        })
        .expect("pull");
        assert!(pull.imported_refs.is_empty());
        assert_eq!(pull.denied_refs, vec![imported.artifact_ref]);
        assert!(ledger::list_artifacts(&destination).expect("destination list").is_empty());
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("federation-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicCounter = AtomicCounter::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            remove_dir_all(&dir).expect("remove stale temp dir");
        }
        create_dir_all(&dir).expect("create temp dir");
        dir
    }
