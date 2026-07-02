
    #[test]
    fn stale_bootstrap_or_missing_capability_evidence_denies_before_side_effects() {
        let root = temp_dir("remote-dataspace-negative-admission");
        let capability_ref = fake_ref("capability-required");
        let bootstrap_ref = fake_ref("bootstrap-required");
        let envelope = build_envelope(EnvelopeInput {
            from_peer: "peer:a".to_owned(),
            from_actor: "producer".to_owned(),
            to_peer: "peer:b".to_owned(),
            topic: "services".to_owned(),
            operation: Operation::Assert,
            payload: record("service-ready", vec![string("db")]),
            content_refs: Vec::new(),
            capability_refs: vec![capability_ref.clone()],
            evidence_refs: vec![bootstrap_ref.clone()],
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let mut stale_bootstrap = evidence_fixture();
        stale_bootstrap.capability_refs = vec![capability_ref.clone()];
        let stale = admit_and_apply_delivered_envelope(&mut RuntimeState::new(1), &delivery, &stale_bootstrap)
            .expect_err("missing declared bootstrap evidence denies");
        assert!(stale.to_string().contains("evidence ref"));
        let mut missing_capability = evidence_fixture();
        missing_capability.peer_bootstrap_refs = vec![bootstrap_ref];
        let denied = admit_and_apply_delivered_envelope(&mut RuntimeState::new(1), &delivery, &missing_capability)
            .expect_err("missing declared capability evidence denies");
        assert!(denied.to_string().contains("capability evidence"));
    }

    fn evidence_fixture() -> DeliveryEvidence {
        DeliveryEvidence {
            peer_bootstrap_refs: vec![fake_ref("bootstrap")],
            capability_refs: vec![fake_ref("capability")],
            policy_refs: vec![fake_ref("policy")],
            resource_refs: vec![fake_ref("resource")],
            authority_refs: vec![fake_ref("authority")],
        }
    }

    fn fake_ref(label: &str) -> String {
        let value = record("fake-ref", vec![string(label)]);
        crate::preserves_rail::canonical_hash(&value).expect("fake ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
