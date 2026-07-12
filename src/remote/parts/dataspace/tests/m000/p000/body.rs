    use super::*;
    use std::fs;

    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;

    #[test]
    fn local_gossip_roundtrip_preserves_envelope_identity() {
        // r[verify molten.chunk_store.cap_std_conversion_validation]
        let root_path = temp_dir("remote-dataspace-roundtrip");
        let root = CapabilityDataspaceRoot::open(&root_path).expect("open dataspace capability root");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        let published = publish_local_gossip_with_root(&root, &envelope, "peer:a").expect("publish");
        assert_eq!(published.envelope_ref, envelope.envelope_ref);
        let delivered =
            deliver_local_gossip_with_root(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        assert_eq!(delivered.envelope.envelope_ref, envelope.envelope_ref);
        assert_eq!(delivered.envelope.topic, "services");
        let receipt_ref = crate::preserves_rail::canonical_hash(&delivered.receipt_value).expect("receipt ref");
        crate::preserves_rail::validate_content_ref(&receipt_ref).expect("receipt ref is canonical");
    }

    #[test]
    fn remote_assertion_applies_through_local_observer_semantics() {
        let root = temp_dir("remote-dataspace-observe");
        let payload_value = record("service-ready", vec![string("db")]);
        let pattern = RuntimeValue::new(payload_value.clone()).expect("runtime value");
        let mut peer_b = RuntimeState::new(1);
        peer_b.apply_step(&RuntimeStep::Observe {
            actor: "consumer".to_owned(),
            pattern: pattern.clone(),
        });
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: payload_value,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let events = apply_delivered_envelope(&mut peer_b, &delivered.envelope).expect("apply delivered envelope");
        assert!(events.iter().any(|event| matches!(event, RuntimeEvent::AssertionCommitted { actor, value }
            if actor == "peer:a/producer" && value == &pattern)));
        assert!(events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { observer, owner, value }
            if observer == "consumer" && owner == "peer:a/producer" && value == &pattern)));
    }

    #[test]
    fn missing_or_tampered_content_ref_is_rejected_before_delivery() {
        let root = temp_dir("remote-dataspace-content-ref");
        let content_ref = store_content_blob(&root, b"large payload").expect("store content");
        let envelope = build_envelope(EnvelopeInput {
            from_peer: "peer:a".to_owned(),
            from_actor: "producer".to_owned(),
            to_peer: "peer:b".to_owned(),
            topic: "services".to_owned(),
            operation: Operation::Assert,
            payload: record("content-ref", vec![string(&content_ref)]),
            content_refs: vec![content_ref.clone()],
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish with valid content");
        fs::write(blob_path(&root, &content_ref).expect("blob path"), b"tampered").expect("tamper blob");
        let error = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b")
            .expect_err("tampered content rejects delivery");
        assert!(error.to_string().contains("content ref"));
    }

    #[test]
    fn refs_reject_malformed_content_refs() {
        for reference in [
            "blake3:short",
            "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "blake3:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        ] {
            let error = validate_ref(reference, "remote regression ref").expect_err("malformed ref must fail closed");
            assert!(error.to_string().contains("canonical content ref"));
        }
    }

    #[test]
    fn admitted_remote_delivery_binds_bootstrap_capability_resource_policy_and_turn_context() {
        let root = temp_dir("remote-dataspace-admission");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let evidence = evidence_fixture();
        let mut state = RuntimeState::new(1);
        let applied = admit_and_apply_delivered_envelope(&mut state, &delivery, &evidence).expect("admit and apply");
        assert!(!applied.events.is_empty());
        crate::preserves_rail::validate_content_ref(&applied.turn_journal_context_ref)
            .expect("turn journal context ref is canonical");
        assert_eq!(
            crate::ledger::artifact_kind(&applied.admission_receipt_value),
            "remote-dataspace-admission-receipt"
        );
        // r[verify molten.delivery_state_machine_proof.denial_no_side_effect]
        let missing =
            admit_and_apply_delivered_envelope(&mut RuntimeState::new(1), &delivery, &DeliveryEvidence::default())
                .expect_err("missing evidence denies before applying");
        assert!(missing.to_string().contains("peer bootstrap"));
    }

    #[test]
    fn idempotent_apply_handles_repeat_and_conflict() {
        let root = temp_dir("remote-dataspace-idempotency");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let evidence = evidence_fixture();
        let mut state = RuntimeState::new(1);
        let first = admit_and_apply_delivered_envelope_idempotent(
            &root,
            &mut state,
            &delivery,
            &evidence,
            crate::delivery_idempotency::GapPolicy::Deny,
        )
        .expect("first idempotent apply");
        assert!(!first.events.is_empty());
        assert_eq!(crate::ledger::artifact_kind(&first.idempotency_receipt_value), "delivery-idempotency-receipt");
        let duplicate = admit_and_apply_delivered_envelope_idempotent(
            &root,
            &mut state,
            &delivery,
            &evidence,
            crate::delivery_idempotency::GapPolicy::Deny,
        )
        .expect("duplicate idempotent apply");
        assert!(duplicate.events.is_empty());
        let first_admission_ref = canonical_hash(&first.admission_receipt_value).expect("first admission ref");
        assert_eq!(duplicate.prior_semantic_result_ref.as_deref(), Some(first_admission_ref.as_str()));
        let log = delivery_log_with_idempotency_receipts(
            std::slice::from_ref(&delivery),
            std::slice::from_ref(&first.idempotency_receipt_value),
            true,
        )
        .expect("idempotent delivery log");
        assert!(crate::preserves_rail::to_text(&log.value).expect("log text").contains("idempotency-receipt"));
        assert_conflict_case(&root, &mut state, &evidence);
    }

    fn assert_conflict_case(root: &Path, state: &mut RuntimeState, evidence: &DeliveryEvidence) {
        let changed = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: record("service-ready", vec![string("api")]),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("changed envelope");
        publish_local_gossip(root, &changed, "peer:a").expect("publish changed");
        let changed_delivery =
            deliver_local_gossip(root, "services", &changed.envelope_ref, "peer:b").expect("deliver changed");
        let conflict = admit_and_apply_delivered_envelope_idempotent(
            root,
            state,
            &changed_delivery,
            evidence,
            crate::delivery_idempotency::GapPolicy::Deny,
        )
        .expect("conflict receipt");
        assert!(conflict.events.is_empty());
        assert!(
            crate::preserves_rail::to_text(&conflict.idempotency_receipt_value)
                .expect("conflict text")
                .contains("conflict")
        );
    }

    #[test]
    fn recorded_delivery_log_replays_without_live_transport() {
        // r[verify molten.delivery_state_machine_proof.replay_log_equivalence]
        const REPLAY_SEED: u64 = 1;
        let root = temp_dir("remote-dataspace-delivery-log");
        let payload = record("service-ready", vec![string("db")]);
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload,
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let delivery = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let log = delivery_log(std::slice::from_ref(&delivery), true).expect("delivery log");
        assert_eq!(crate::ledger::artifact_kind(&log.value), "remote-dataspace-delivery-log");
        fs::remove_dir_all(root.join("gossip")).expect("remove live transport bytes");
        let mut observed_state = RuntimeState::new(REPLAY_SEED);
        let observed_events = apply_delivered_envelope(&mut observed_state, &delivery.envelope).expect("observed apply");
        let observed_state_ref = observed_state.snapshot().snapshot_ref().expect("observed state ref");
        let mut replay_state = RuntimeState::new(REPLAY_SEED);
        let replayed_events = replay_delivery_log(&mut replay_state, &log).expect("replay from recorded log");
        assert_eq!(replayed_events, observed_events);
        assert_eq!(replay_state.snapshot().snapshot_ref().expect("replayed state ref"), observed_state_ref);
        assert!(replayed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionCommitted { .. })));

        let non_replayable = delivery_log(std::slice::from_ref(&delivery), false).expect("non replayable log");
        let error = replay_delivery_log(&mut RuntimeState::new(REPLAY_SEED), &non_replayable)
            .expect_err("non replayable live run excluded");
        assert!(error.to_string().contains("non-replayable"));

        let mismatched_receipt = mismatched_idempotency_receipt(&root);
        let tampered = delivery_log_with_idempotency_receipts(
            std::slice::from_ref(&delivery),
            std::slice::from_ref(&mismatched_receipt),
            true,
        )
        .expect_err("mismatched idempotency receipt fails closed");
        assert!(tampered.to_string().contains("operation ref mismatch"));
        let missing_receipt = delivery_log_with_idempotency_receipts(
            std::slice::from_ref(&delivery),
            &[mismatched_receipt, fake_ref_value("extra-receipt")],
            true,
        )
        .expect_err("extra idempotency receipt fails closed");
        assert!(missing_receipt.to_string().contains("receipt count must match delivery count"));
        let missing_prior = delivery_log_with_idempotency_receipts(
            std::slice::from_ref(&delivery),
            std::slice::from_ref(&duplicate_receipt_without_prior(&delivery)),
            true,
        )
        .expect_err("duplicate receipt without prior fails closed");
        assert!(missing_prior.to_string().contains("missing prior receipt"));
    }

    #[test]
    fn live_gossip_bytes_use_same_receipt_boundary() {
        let root = temp_dir("remote-dataspace-live-bytes");
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: record("service-ready", vec![string("db")]),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        let bytes = canonical_bytes(&envelope.value).expect("envelope bytes");
        let delivered =
            deliver_live_gossip_bytes(&root, &bytes, "services", "peer:b", "endpoint:a").expect("deliver live bytes");
        assert_eq!(delivered.envelope.envelope_ref, envelope.envelope_ref);
        assert_eq!(crate::ledger::artifact_kind(&delivered.receipt_value), "remote-dataspace-transport-receipt");
    }

    #[test]
    fn two_peer_harness_records_replay_and_gate_receipt() {
        let root = temp_dir("remote-dataspace-two-peer-harness");
        let harness = two_peer_service_ready_harness(&root, evidence_fixture()).expect("two peer harness");
        assert!(harness.observed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));
        assert!(harness.replayed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));
        assert_eq!(crate::ledger::artifact_kind(&harness.receipt_value), "remote-dataspace-gate-receipt");
    }

    #[test]
    fn wrong_topic_wrong_peer_and_tampered_envelope_are_rejected() {
        let root = temp_dir("remote-dataspace-negative-routing");
        let envelope = assert_envelope(AssertEnvelopeInput {
            from_peer: "peer:a",
            from_actor: "producer",
            to_peer: "peer:b",
            topic: "services",
            payload: record("service-ready", vec![string("db")]),
            capability_refs: Vec::new(),
            evidence_refs: Vec::new(),
        })
        .expect("envelope");
        publish_local_gossip(&root, &envelope, "peer:a").expect("publish");
        let wrong_topic = deliver_local_gossip(&root, "other", &envelope.envelope_ref, "peer:b")
            .expect_err("wrong topic has no stored envelope");
        assert!(wrong_topic.to_string().contains("io error"));
        let wrong_peer =
            deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:c").expect_err("wrong peer rejects");
        assert!(wrong_peer.to_string().contains("target"));
        fs::write(envelope_path(&root, "services", &envelope.envelope_ref).expect("envelope path"), b"not-preserves")
            .expect("tamper envelope bytes");
        let tampered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b")
            .expect_err("tampered envelope rejects");
        assert!(tampered.to_string().contains("preserves"));
    }
