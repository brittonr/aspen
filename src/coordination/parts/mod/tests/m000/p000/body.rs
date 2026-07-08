    type TestCase = hegel::TestCase;

    use super::*;

    const LOCAL_STALE_REGISTER_SEQUENCE: u64 = 1;
    const BATCH_ENQUEUE_SEQUENCE: u64 = 1;
    const BATCH_READ_SEQUENCE: u64 = 2;
    const STALE_COMPARE_SEQUENCE: u64 = 3;
    const BATCH_EXPECTED_RESULTS: usize = 2;

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn refs() -> (Vec<String>, Vec<String>, Vec<String>) {
        (vec![fixture_ref("auth")], vec![fixture_ref("resource")], vec![fixture_ref("policy")])
    }

    fn runtime() -> CoordinationRuntime {
        let manifest = coordination_fixture_manifest_value().expect("manifest");
        new_coordination_runtime(&manifest).expect("runtime")
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove stale temp root");
        }
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn request(
        service: &str,
        operation: &str,
        key: &str,
        session: &str,
        sequence: u64,
        payload: Option<IoValue>,
    ) -> IoValue {
        let (auth, resources, policies) = refs();
        let fixture_refs = CoordinationRefSlices {
            authority_refs: &auth,
            resource_refs: &resources,
            policy_refs: &policies,
        };
        fixture_request(FixtureRequestInput {
            service,
            operation,
            key,
            client_session: session,
            sequence,
            payload,
            refs: &fixture_refs,
        })
        .expect("request")
    }

    #[test]
    fn coordination_rejects_malformed_content_refs() {
        let (authority_refs, resource_refs, policy_refs) = refs();
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            let error = coordination_service_manifest_value(&CoordinationServiceManifestInput {
                service_id: DEFAULT_COORDINATION_SERVICE_ID.to_string(),
                services: vec![SERVICE_LOCK.to_string(), SERVICE_QUEUE.to_string()],
                control_group_ref: invalid.to_string(),
                queue_capacity: DEFAULT_COORDINATION_QUEUE_CAPACITY,
                semaphore_capacity: DEFAULT_COORDINATION_SEMAPHORE_CAPACITY,
                rate_limit: DEFAULT_COORDINATION_RATE_LIMIT,
                barrier_parties: DEFAULT_COORDINATION_BARRIER_PARTIES,
                policy_refs: policy_refs.clone(),
                resource_refs: resource_refs.clone(),
            })
            .expect_err("malformed manifest ref denied");
            assert!(error.to_string().contains("canonical blake3 content ref"), "unexpected error: {error}");

            let request_error = coordination_request_value(&CoordinationRequestInput {
                service: SERVICE_LOCK.to_string(),
                operation: OP_ACQUIRE.to_string(),
                key: "resource:test".to_string(),
                client_session: "session-malformed".to_string(),
                operation_id_ref: invalid.to_string(),
                read_consistency_mode: READ_CONSISTENCY_LINEARIZABLE.to_string(),
                payload: None,
                authority_refs: authority_refs.clone(),
                resource_refs: resource_refs.clone(),
                policy_refs: policy_refs.clone(),
            })
            .expect_err("malformed request ref denied");
            assert!(
                request_error.to_string().contains("canonical blake3 content ref"),
                "unexpected error: {request_error}"
            );
        }
    }

    #[test]
    fn lock_acquire_release_stale_fencing_and_duplicate_are_receipted() {
        let mut runtime = runtime();
        let acquire = request(SERVICE_LOCK, OP_ACQUIRE, "resource:test", "session-a", 1, None);
        let first = apply_coordination_request(&mut runtime, &acquire).expect("acquire");
        assert_eq!(first.receipt.decision, "pass");
        let token = first.token.as_ref().expect("token").token;
        assert_eq!(token, 1);
        assert_eq!(first.assertions.len(), 1);
        let duplicate = apply_coordination_request(&mut runtime, &acquire).expect("duplicate");
        assert_eq!(duplicate.receipt.receipt_ref, first.receipt.receipt_ref);
        assert_eq!(runtime.state.next_fencing_token, 2);
        let stale = request(
            SERVICE_LOCK,
            OP_RELEASE,
            "resource:test",
            "session-a",
            2,
            Some(record("token", vec![u64_value(0)])),
        );
        let stale = apply_coordination_request(&mut runtime, &stale).expect("stale deny");
        assert_eq!(stale.receipt.decision, "deny");
        assert!(stale.receipt.diagnostics.join(";").contains("stale fencing token"));
        let release = request(
            SERVICE_LOCK,
            OP_RELEASE,
            "resource:test",
            "session-a",
            3,
            Some(record("token", vec![u64_value(token)])),
        );
        let release = apply_coordination_request(&mut runtime, &release).expect("release");
        assert_eq!(release.receipt.decision, "pass");
        assert!(runtime.state.locks.is_empty());
    }

    #[test]
    fn queue_fifo_duplicate_overflow_and_resource_denial_are_receipted() {
        let mut coord_runtime = runtime();
        let one =
            request(SERVICE_QUEUE, OP_ENQUEUE, "queue:test", "producer", 1, Some(record("item", vec![string("one")])));
        let two =
            request(SERVICE_QUEUE, OP_ENQUEUE, "queue:test", "producer", 2, Some(record("item", vec![string("two")])));
        let first = apply_coordination_request(&mut coord_runtime, &one).expect("enqueue one");
        let duplicate = apply_coordination_request(&mut coord_runtime, &one).expect("duplicate one");
        assert_eq!(first.receipt.receipt_ref, duplicate.receipt.receipt_ref);
        apply_coordination_request(&mut coord_runtime, &two).expect("enqueue two");
        let dequeue = request(SERVICE_QUEUE, OP_DEQUEUE, "queue:test", "consumer", 3, None);
        let dequeue = apply_coordination_request(&mut coord_runtime, &dequeue).expect("dequeue");
        assert_eq!(dequeue.receipt.decision, "pass");
        assert_eq!(coord_runtime.state.queues.get("queue:test").expect("queue")[0], "two");

        let mut small = runtime();
        small.manifest.queue_capacity = 1;
        apply_coordination_request(
            &mut small,
            &request(SERVICE_QUEUE, OP_ENQUEUE, "queue:small", "p", 1, Some(record("item", vec![string("a")]))),
        )
        .expect("first");
        let overflow = apply_coordination_request(
            &mut small,
            &request(SERVICE_QUEUE, OP_ENQUEUE, "queue:small", "p", 2, Some(record("item", vec![string("b")]))),
        )
        .expect("overflow");
        assert_eq!(overflow.receipt.decision, "deny");
        assert!(overflow.receipt.diagnostics.join(";").contains("queue overflow"));

        let (auth, _resources, policies) = refs();
        let empty_resources = Vec::new();
        let denied_refs = CoordinationRefSlices {
            authority_refs: &auth,
            resource_refs: &empty_resources,
            policy_refs: &policies,
        };
        let denied = fixture_request(FixtureRequestInput {
            service: SERVICE_QUEUE,
            operation: OP_ENQUEUE,
            key: "queue:deny",
            client_session: "p",
            sequence: 9,
            payload: Some(record("item", vec![string("x")])),
            refs: &denied_refs,
        })
        .expect("request");
        let denied = apply_coordination_request(&mut small, &denied).expect("resource deny");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(denied.receipt.diagnostics.join(";").contains("missing resource"));
    }

    #[test]
    fn service_registry_updates_and_read_index_reads_are_control_plane_bound() {
        let mut runtime = runtime();
        let endpoint = fixture_ref("endpoint");
        let evidence = fixture_ref("evidence");
        let register = request(
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:coord",
            "registrar",
            1,
            Some(record("endpoint", vec![string(&endpoint), string(&evidence)])),
        );
        let register = apply_coordination_request(&mut runtime, &register).expect("register");
        assert_eq!(register.receipt.decision, "pass");
        assert!(register.receipt.raft_receipt_ref.is_some());
        let read = request(SERVICE_REGISTRY, OP_READ, "svc:coord", "reader", 2, None);
        let read = apply_coordination_request(&mut runtime, &read).expect("read");
        assert_eq!(read.receipt.decision, "pass");
        assert!(read.raft_read_receipt.is_some());
        assert_eq!(read.assertions.len(), 1);
        let assertion_text = to_text(&read.assertions[0].value).expect("assertion text");
        assert!(assertion_text.contains(&endpoint));
        assert!(assertion_text.contains("engine-currentness"));
        assert!(assertion_text.contains("engine-epoch"));
        assert!(to_text(&read.receipt.value).expect("receipt text").contains("normalized-consensus-evidence"));
    }

    #[test]
    fn coordination_engine_epoch_gate_denies_stale_or_inactive_consensus_evidence() {
        // r[verify molten.coordination.engine_agnostic_evidence]
        // r[verify molten.coordination.engine_switchover_gates]
        let stale = coordination_engine_epoch_admission(&crate::raft_control_plane::ConsensusEngineEpochGateInput {
            operation: OP_RELEASE.to_string(),
            active_profile: crate::raft_control_plane::CONSENSUS_PROFILE_RAFT.to_string(),
            active_engine_epoch: crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH.saturating_add(1),
            presented_profile: crate::raft_control_plane::CONSENSUS_PROFILE_RAFT.to_string(),
            presented_engine_epoch: crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH,
            activation_receipt_ref: Some(fixture_ref("activation")),
        })
        .expect("stale epoch gate");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.join(";").contains("stale engine epoch"));

        let inactive = coordination_engine_epoch_admission(&crate::raft_control_plane::ConsensusEngineEpochGateInput {
            operation: OP_READ.to_string(),
            active_profile: crate::raft_control_plane::CONSENSUS_PROFILE_RAFT.to_string(),
            active_engine_epoch: crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH,
            presented_profile: crate::raft_control_plane::CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL.to_string(),
            presented_engine_epoch: crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH.saturating_add(1),
            activation_receipt_ref: None,
        })
        .expect("inactive epoch gate");
        assert_eq!(inactive.decision, "deny");
        assert!(inactive.diagnostics.join(";").contains("inactive consensus engine profile"));
        assert!(inactive.diagnostics.join(";").contains("not activated"));
    }

    #[test]
    fn local_stale_reads_are_labeled_and_cannot_authorize_mutations() {
        // r[verify molten.coordination.read_consistency_modes]
        // r[verify molten.coordination.local_stale_boundaries]
        let mut runtime = runtime();
        let endpoint = fixture_ref("endpoint-stale");
        let evidence = fixture_ref("evidence-stale");
        let register = request(
            SERVICE_REGISTRY,
            OP_REGISTER,
            "svc:stale",
            "registrar",
            LOCAL_STALE_REGISTER_SEQUENCE,
            Some(record("endpoint", vec![string(&endpoint), string(&evidence)])),
        );
        let register = apply_coordination_request(&mut runtime, &register).expect("register");
        assert_eq!(register.receipt.decision, "pass");

        let (auth, resources, policies) = refs();
        let local_stale_read = coordination_request_value(&CoordinationRequestInput {
            service: SERVICE_REGISTRY.to_string(),
            operation: OP_READ.to_string(),
            key: "svc:stale".to_string(),
            client_session: "reader".to_string(),
            operation_id_ref: fixture_ref("local-stale-read"),
            read_consistency_mode: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            payload: None,
            authority_refs: auth.clone(),
            resource_refs: resources.clone(),
            policy_refs: policies.clone(),
        })
        .expect("local stale read request");
        let read = apply_coordination_request(&mut runtime, &local_stale_read).expect("local stale read");
        assert_eq!(read.receipt.decision, "pass");
        assert_eq!(read.receipt.read_consistency_mode, READ_CONSISTENCY_LOCAL_STALE);
        assert_eq!(read.assertions[0].read_consistency_mode, READ_CONSISTENCY_LOCAL_STALE);
        assert!(to_text(&read.receipt.value).expect("receipt text").contains("local-stale-non-authoritative"));

        let stale_mutation = coordination_request_value(&CoordinationRequestInput {
            service: SERVICE_LOCK.to_string(),
            operation: OP_ACQUIRE.to_string(),
            key: "resource:stale".to_string(),
            client_session: "owner".to_string(),
            operation_id_ref: fixture_ref("local-stale-mutation"),
            read_consistency_mode: READ_CONSISTENCY_LOCAL_STALE.to_string(),
            payload: None,
            authority_refs: auth,
            resource_refs: resources,
            policy_refs: policies,
        })
        .expect("stale mutation request");
        let denied = apply_coordination_request(&mut runtime, &stale_mutation).expect("stale mutation denied");
        assert_eq!(denied.receipt.decision, "deny");
        assert!(denied.receipt.diagnostics.join(";").contains("local-stale"));
    }

    #[test]
    fn batch_envelopes_apply_deterministically_and_deny_invalid_batches() {
        // r[verify molten.coordination.batched_control_plane_operations]
        let mut batch_runtime = runtime();
        let initial_state = snapshot_from_state(&batch_runtime.state).expect("initial snapshot");
        let enqueue = request(
            SERVICE_QUEUE,
            OP_ENQUEUE,
            "queue:batch",
            "producer",
            BATCH_ENQUEUE_SEQUENCE,
            Some(record("item", vec![string("batch-one")])),
        );
        let read = request(SERVICE_QUEUE, OP_READ, "queue:batch", "reader", BATCH_READ_SEQUENCE, None);
        let batch = coordination_batch_envelope_value(&CoordinationBatchEnvelopeInput {
            batch_id_ref: fixture_ref("batch-pass"),
            compare_state_ref: Some(initial_state.state_ref.clone()),
            requests: vec![enqueue.clone(), read],
            policy_refs: vec![fixture_ref("batch-policy")],
        })
        .expect("batch envelope");
        assert_eq!(crate::ledger::artifact_kind(&batch), "coordination-batch-envelope");
        let applied = apply_coordination_batch(&mut batch_runtime, &batch).expect("apply batch");
        assert_eq!(applied.decision, "pass");
        assert_eq!(applied.results.len(), BATCH_EXPECTED_RESULTS);
        assert_eq!(batch_runtime.state.queues.get("queue:batch").expect("queue")[QUEUE_FRONT_INDEX], "batch-one");

        let mut duplicate_runtime = runtime();
        let before_duplicate = snapshot_from_state(&duplicate_runtime.state).expect("before duplicate");
        let duplicate_batch = coordination_batch_envelope_value(&CoordinationBatchEnvelopeInput {
            batch_id_ref: fixture_ref("batch-duplicate"),
            compare_state_ref: Some(before_duplicate.state_ref.clone()),
            requests: vec![enqueue.clone(), enqueue],
            policy_refs: vec![fixture_ref("batch-policy")],
        })
        .expect("duplicate batch");
        let duplicate = apply_coordination_batch(&mut duplicate_runtime, &duplicate_batch).expect("duplicate batch deny");
        assert_eq!(duplicate.decision, "deny");
        assert!(duplicate.results.is_empty());
        assert_eq!(snapshot_from_state(&duplicate_runtime.state).expect("after duplicate").state_ref, before_duplicate.state_ref);

        let stale_compare_batch = coordination_batch_envelope_value(&CoordinationBatchEnvelopeInput {
            batch_id_ref: fixture_ref("batch-stale-compare"),
            compare_state_ref: Some(fixture_ref("stale-state")),
            requests: vec![request(
                SERVICE_QUEUE,
                OP_ENQUEUE,
                "queue:stale-compare",
                "producer",
                STALE_COMPARE_SEQUENCE,
                Some(record("item", vec![string("stale")])),
            )],
            policy_refs: vec![fixture_ref("batch-policy")],
        })
        .expect("stale compare batch");
        let stale_compare = apply_coordination_batch(&mut duplicate_runtime, &stale_compare_batch).expect("stale compare deny");
        assert_eq!(stale_compare.decision, "deny");
        assert!(stale_compare.results.is_empty());
    }

    #[test]
    fn semaphore_rate_election_barrier_and_registry_primitives_are_deterministic() {
        let mut runtime = runtime();
        let first =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "a", 1, None))
                .expect("sem a");
        let second =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "b", 2, None))
                .expect("sem b");
        let exhausted =
            apply_coordination_request(&mut runtime, &request(SERVICE_SEMAPHORE, OP_ACQUIRE, "sem:test", "c", 3, None))
                .expect("sem deny");
        assert_eq!(first.receipt.decision, "pass");
        assert_eq!(second.receipt.decision, "pass");
        assert_eq!(exhausted.receipt.decision, "deny");
        assert!(exhausted.receipt.diagnostics.join(";").contains("semaphore exhausted"));
        let rate_a = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "a", 4, None),
        )
        .expect("rate a");
        let rate_b = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "b", 5, None),
        )
        .expect("rate b");
        let rate_c = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_RATE_LIMIT, OP_ACQUIRE, "rate:test", "c", 6, None),
        )
        .expect("rate deny");
        assert_eq!(rate_a.receipt.decision, "pass");
        assert_eq!(rate_b.receipt.decision, "pass");
        assert_eq!(rate_c.receipt.decision, "deny");
        let elect = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_ELECTION, OP_ELECT, "election:test", "leader", 7, None),
        )
        .expect("elect");
        assert_eq!(elect.receipt.decision, "pass");
        assert!(elect.token.is_some());
        let barrier_wait = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_BARRIER, OP_ARRIVE, "barrier:test", "a", 8, None),
        )
        .expect("barrier wait");
        let barrier_release = apply_coordination_request(
            &mut runtime,
            &request(SERVICE_BARRIER, OP_ARRIVE, "barrier:test", "b", 9, None),
        )
        .expect("barrier release");
        assert_eq!(barrier_wait.receipt.decision, "pass");
        assert_eq!(barrier_release.receipt.decision, "pass");
        assert!(runtime.state.barriers.get("barrier:test").expect("barrier").is_released);
    }
