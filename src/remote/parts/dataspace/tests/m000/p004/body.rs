
    #[test]
    fn two_peer_harness_records_replay_and_gate_receipt() {
        let root = temp_dir("remote-dataspace-two-peer-harness");
        let harness = two_peer_service_ready_harness(&root, evidence_fixture()).expect("two peer harness");
        assert!(harness.observed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));
        assert!(harness.replayed_events.iter().any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })));
        assert_eq!(crate::ledger::artifact_kind(&harness.receipt_value), "remote-dataspace-gate-receipt");
    }

    // r[verify molten.runtime_spine.record_pattern_subset]
    #[test]
    fn the_service_ready_consumer_keeps_matching_a_record_with_an_extra_field() {
        let root = temp_dir("remote-dataspace-service-ready-subscription");
        let mut peer_b = RuntimeState::new(1);
        peer_b.apply_step(&RuntimeStep::Observe {
            actor: "consumer".to_owned(),
            pattern: RuntimeValue::new(
                crate::runtime::RuntimePattern::record(
                    "service-ready",
                    vec![crate::runtime::RecordPatternEntry {
                        position: 0,
                        pattern: Box::new(crate::runtime::RuntimePattern::wildcard("service")),
                    }],
                )
                .to_value(),
            )
            .expect("service ready pattern"),
        });
        let payload = record("service-ready", vec![string("db"), string("zone-a")]);
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
        let delivered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b").expect("deliver");
        let applied = admit_and_apply_delivered_envelope(&mut peer_b, &delivered, &evidence_fixture())
            .expect("apply delivered envelope");
        assert!(
            applied
                .events
                .iter()
                .any(|event| matches!(event, RuntimeEvent::AssertionObserved { .. })),
            "the subscription keeps matching a record that carries an extra field"
        );
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
        assert!(wrong_topic.to_string().contains("does not exist"), "{wrong_topic}");
        let wrong_peer =
            deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:c").expect_err("wrong peer rejects");
        assert!(wrong_peer.to_string().contains("target"));
        std::fs::write(envelope_path(&root, "services", &envelope.envelope_ref).expect("envelope path"), b"not-preserves")
            .expect("tamper envelope bytes");
        let tampered = deliver_local_gossip(&root, "services", &envelope.envelope_ref, "peer:b")
            .expect_err("tampered envelope rejects");
        assert!(tampered.to_string().contains("preserves"));
    }
