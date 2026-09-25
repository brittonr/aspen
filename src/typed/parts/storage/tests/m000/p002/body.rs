
    fn default_typed_ref_checks() -> &'static [&'static str] {
        &[
            "typed-durable-ref",
            "schema-ref-binding",
            "schema-identity-binding",
            "value-ref-binding",
            "producer-artifact-binding",
            "intended-consumer-binding",
            "handler-profile-binding",
            "capability-binding",
            "retention-binding",
            "provenance-binding",
            "evidence-binding",
            "decoder-artifact-admission",
            "handle-not-authority",
            "no-raw-memory-layout",
            "no-function-serialization",
        ]
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("typed-storage-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
