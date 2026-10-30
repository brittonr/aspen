    #[test]
    fn protocol_and_job_builders_require_introduced_semantic_fields() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let sender_space = crate::test_support::process_workspace("recorded-carrier-sender").expect("workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-carrier-receiver").expect("workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let store = RecordedSessionStore::open(&receiver).expect("store");
        let opened = store.admit_local(&sender, "recorded-services").expect("admission");
        let view = store.sender_view(&sender, "recorded-services", opened.generation).expect("view");
        let protocol_ref = content_ref_from_bytes(b"protocol artifact");
        let body_ref = content_ref_from_bytes(b"protocol body");
        let protocol_message = |body_or_ref| crate::protocol_session::protocol_message_value(&crate::protocol_session::ProtocolMessageInput {
            protocol_ref: protocol_ref.clone(), session_id: "session:recorded".to_owned(),
            from_role: "client".to_owned(), to_role: "server".to_owned(), label: "request".to_owned(),
            payload_tag: "content".to_owned(), body_or_ref, sequence: 0, evidence_refs: Vec::new(),
        }).expect("typed protocol message");
        let protocol = protocol_message(string(&body_ref));
        let protocol_input = || crate::protocol_session::ProtocolRemoteEnvelopeInput {
            from_peer: "node:recorded-sender".to_owned(), from_actor: "client".to_owned(),
            to_peer: "node:recorded-receiver".to_owned(), topic: "recorded-services".to_owned(),
            message: protocol.clone(), capability_refs: Vec::new(), evidence_refs: Vec::new(),
        };
        let mut metadata_only = protocol_input();
        metadata_only.message = protocol_message(record("plain", vec![]));
        assert!(matches!(crate::protocol_session::protocol_message_remote_envelope_with_evidence(metadata_only, None)
            .expect("protocol/schema metadata decision"), EnvelopeBuild::Built(_)));
        let diagnostics = assert_build_denied(
            crate::protocol_session::protocol_message_remote_envelope_with_evidence(protocol_input(), Some(&view))
                .expect("protocol sender decision"), Some(&body_ref));
        assert!(!diagnostics.iter().any(|diagnostic| diagnostic.contains(&protocol_ref)),
            "schema identity must not be diagnosed as an unknown capability");
        store.commit("node:recorded-sender", "recorded-services", opened.generation, Operation::Assert, &protocol)
            .expect("receiver commits typed introducing assertion");
        assert!(matches!(crate::protocol_session::protocol_message_remote_envelope_with_evidence(protocol_input(), Some(&view))
            .expect("introduced protocol sender decision"), EnvelopeBuild::Built(_)));
        assert_denied(input(Operation::Message, record("schema-as-object", vec![string(&protocol_ref)])),
            Some(&view), &protocol_ref);

        let job_ref = content_ref_from_bytes(b"job artifact");
        let sync_ref = content_ref_from_bytes(b"sync artifact");
        let admission_ref = content_ref_from_bytes(b"admission artifact");
        let execution_ref = content_ref_from_bytes(b"execution artifact");
        let request = crate::job_dag::job_worker_request_value(crate::job_dag::JobWorkerRequestValueInput {
            job_ref: &job_ref, target_peer: "node:recorded-receiver", stage_ids: &[], sync_ref: &sync_ref,
            admission_ref: &admission_ref, execution_request_ref: &execution_ref, authority_refs: &[],
            resource_refs: &[], peer_bootstrap_refs: &[], node_identity_refs: &[], evidence_refs: &[],
        }).expect("typed worker request");
        let worker_input = || crate::job_dag::JobWorkerEnvelopeInput {
            from_peer: "node:recorded-sender", from_actor: "worker", to_peer: "node:recorded-receiver",
            topic: "recorded-services", request_value: &request,
        };
        let diagnostics = assert_build_denied(
            crate::job_dag::job_worker_envelope_with_evidence(worker_input(), Some(&view)).expect("worker sender decision"),
            Some(&job_ref));
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.contains(&execution_ref)));
        assert!(!diagnostics.iter().any(|diagnostic| diagnostic.contains(&sync_ref)));
        assert!(!diagnostics.iter().any(|diagnostic| diagnostic.contains(&admission_ref)));
        store.commit("node:recorded-sender", "recorded-services", opened.generation, Operation::Assert, &request)
            .expect("receiver commits worker introduction");
        assert!(matches!(crate::job_dag::job_worker_envelope_with_evidence(worker_input(), Some(&view))
            .expect("introduced worker sender decision"), EnvelopeBuild::Built(_)));
        assert_denied(input(Operation::Message, record("receipt-as-object", vec![string(&sync_ref)])),
            Some(&view), &sync_ref);
    }

    #[test]
    fn full_live_history_can_close_and_readmit() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        use redb::ReadableTable;
        let sender_space = crate::test_support::process_workspace("recorded-full-sender").expect("workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-full-receiver").expect("workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let store = RecordedSessionStore::open(&receiver).expect("store");
        let opened = store.admit_local(&sender, "recorded-services").expect("admission");
        let key = recorded_key("node:recorded-sender", "node:recorded-receiver", "recorded-services").expect("key");
        // Install a valid near-bound history directly to keep setup linear;
        // public mutation/replay/retirement remain the behavior under test.
        let write = store.database.begin_write().expect("setup transaction");
        {
            let mut table = write.open_table(RECORDED_SESSIONS).expect("table");
            let mut data = {
                let entry = table.get(key.as_str()).expect("read").expect("session");
                decode_recorded(entry.value()).expect("decode")
            };
            while data.observations.len() < MAX_RECORDED_OBSERVATIONS - 1 {
                let mut observation = RecordedObservation {
                    sequence: data.observations.len() as u64,
                    predecessor: data.observations.last().expect("prior observation").observation_ref.clone(),
                    operation: "observe".to_owned(), payload: canonical_bytes(&record("ready", vec![])).expect("payload"),
                    observation_ref: String::new(), content_refs: Vec::new(),
                    source_envelope_ref: None,
                };
                observation.observation_ref = canonical_hash(&observation_value(&data, &observation).expect("observation"))
                    .expect("observation ref");
                data.observations.push(observation);
            }
            table.insert(key.as_str(), encode_recorded(&data).expect("bounded recording").as_slice()).expect("setup");
        }
        write.commit().expect("commit setup");
        match store.commit_with_evidence("node:recorded-sender", "recorded-services", opened.generation,
            Operation::Assert, &record("new", vec![])).expect("capacity decision") {
            RecordedSessionOutcome::Denied { receipt_value, .. } => {
                assert!(receipt_value.collect_simple_record("remote-recorded-session-denial-v1", Some(8)).is_some());
            }
            RecordedSessionOutcome::Committed(_) => panic!("terminal slot must remain available"),
        }
        store.close("node:recorded-sender", "recorded-services", opened.generation).expect("close at capacity");
        let next = store.admit_local(&sender, "recorded-services").expect("readmit after capacity close");
        assert_eq!(next.generation, opened.generation + 1);
    }

    #[test]
    fn lost_database_never_reinitializes_an_acknowledged_generation() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        let sender_space = crate::test_support::process_workspace("recorded-lost-sender").expect("workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-lost-receiver").expect("workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let store = RecordedSessionStore::open(&receiver).expect("store");
        store.admit_local(&sender, "recorded-services").expect("acknowledged admission");
        drop(store);
        let storage = receiver.control_service().expect("service root").open_subdir(
            &crate::node_state::RelativePath::parse("remote-recorded-sessions-v1").expect("path"),
        ).expect("recording scope");
        let file = storage.open_database_file(&crate::node_state::RelativePath::parse("sessions.redb").expect("path"))
            .expect("task-owned database");
        file.set_len(0).expect("inject lost recording");
        match RecordedSessionStore::open(&receiver) {
            Err(_) => {},
            Ok(_) => panic!("lost observations must not permit generation reuse"),
        }
        assert_eq!(file.metadata().expect("preserved failure artifact").len(), 0);
    }
