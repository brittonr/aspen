
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

    fn fake_ref_value(label: &str) -> IoValue {
        record("fake-ref-value", vec![string(label)])
    }

    fn mismatched_idempotency_receipt(root: &Path) -> IoValue {
        const MISMATCH_SEQUENCE: u64 = 1;
        let store_root = root.join("mismatched-idempotency");
        let scope = crate::delivery_idempotency::remote_topic_scope_ref("other-services", "peer:b")
            .expect("mismatch scope");
        let policy_refs = vec![fake_ref("mismatch-policy")];
        let evidence_refs = vec![fake_ref("mismatch-evidence")];
        let result_ref = fake_ref("mismatch-result");
        crate::delivery_idempotency::check(crate::delivery_idempotency::CheckInput {
            root: &store_root,
            scope_profile: crate::delivery_idempotency::SCOPE_REMOTE_TOPIC,
            scope_ref: &scope,
            producer: "peer:z/producer",
            consumer: "peer:b",
            sequence: MISMATCH_SEQUENCE,
            intent: "remote-dataspace-assert",
            payload_ref: &fake_ref("mismatch-payload"),
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            semantic_result_ref: Some(&result_ref),
            gap_policy: crate::delivery_idempotency::GapPolicy::Deny,
        })
        .expect("mismatched idempotency receipt")
        .receipt
        .value
    }

    fn duplicate_receipt_without_prior(delivery: &Delivery) -> IoValue {
        let scope = crate::delivery_idempotency::remote_topic_scope_ref(&delivery.envelope.topic, &delivery.envelope.to_peer)
            .expect("delivery scope");
        record("delivery-idempotency-receipt-v1", vec![
            string(crate::preserves_rail::DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA),
            record("decision", vec![string("duplicate")]),
            record("operation", vec![string(&delivery.envelope.operation_ref)]),
            record("scope", vec![string(&scope)]),
            record("window", vec![string(fake_ref("missing-prior-window"))]),
            record("prior", vec![record("none", Vec::new())]),
            record("semantic-result", vec![record("none", Vec::new())]),
            record("side-effect", vec![string("suppress")]),
            record("diagnostics", vec![sequence(Vec::new())]),
            record("checks", vec![sequence(vec![record("check", vec![
                string("dedup-before-commit"),
                string("pass"),
            ])])]),
        ])
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
