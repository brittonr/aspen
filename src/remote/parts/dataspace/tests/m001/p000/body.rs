    use super::*;

    fn init(root: &Path, node_id: &str) -> crate::node_state::DirectoryRoot {
        crate::node_daemon::init_local(&crate::node_daemon::InitInput { state_root: root, node_id })
            .expect("initialize actual node identity");
        crate::node_state::DirectoryRoot::open_existing(root).expect("initialized node root")
    }

    fn input(operation: Operation, payload: IoValue) -> EnvelopeInput {
        EnvelopeInput {
            from_peer: "node:recorded-sender".to_owned(), from_actor: "producer".to_owned(),
            to_peer: "node:recorded-receiver".to_owned(), topic: "recorded-services".to_owned(),
            operation, payload, content_refs: Vec::new(), capability_refs: Vec::new(), evidence_refs: Vec::new(),
        }
    }

    fn assert_denied(input: EnvelopeInput, view: Option<&SenderSessionView<'_>>, expected: &str) {
        let _ = assert_build_denied(build_envelope_with_evidence(input, view).expect("build decision"), Some(expected));
    }

    fn assert_source_refused(input: EnvelopeInput, view: &SenderSessionView<'_>) {
        let _ = assert_build_denied(build_envelope_with_evidence(input, Some(view)).expect("source decision"), None);
    }

    fn assert_build_denied(outcome: EnvelopeBuild, expected: Option<&str>) -> Vec<String> {
        match outcome {
            EnvelopeBuild::Denied { receipt_value, diagnostics } => {
                if let Some(expected) = expected {
                    assert!(diagnostics.iter().any(|diagnostic| diagnostic.contains(expected)), "{diagnostics:?}");
                }
                let fields = receipt_value.collect_simple_record("remote-dataspace-envelope-build-denial-v1", Some(7))
                    .expect("canonical build denial");
                assert_eq!(fields[1].as_string().as_deref(), Some("deny"));
                let packed = canonical_bytes(&receipt_value).expect("canonical refusal bytes");
                assert_eq!(parse_canonical_bytes(&packed).expect("decode refusal"), receipt_value);
                diagnostics
            }
            EnvelopeBuild::Built(_) => panic!("unexpected envelope publication candidate"),
        }
    }

    /// An untrusted peer message carrying an unintroduced reference is denied
    /// with a canonical receipt and leaves the receiver state untouched.
    fn assert_unknown_message_not_committed(
        store: &RecordedSessionStore, view: &SenderSessionView<'_>, generation: u64, message: EnvelopeInput,
        reference: &str,
    ) {
        let before = view.snapshot().expect("actual receiver state before unknown message");
        let unknown = incoming_envelope_fixture(message).expect("untrusted peer message");
        match store.receive_with_evidence(generation, &unknown).expect("receiver admission") {
            RecordedSessionOutcome::Denied { receipt_value, diagnostics } => {
                assert!(diagnostics.iter().any(|diagnostic| diagnostic.contains(reference)));
                assert!(receipt_value.collect_simple_record("remote-recorded-session-denial-v1", Some(8)).is_some());
            }
            RecordedSessionOutcome::Committed(_) => panic!("unknown message must not commit"),
        }
        assert_eq!(view.snapshot().expect("receiver state after denial"), before);
    }

    /// Republishing an envelope built under a since-withdrawn introduction is
    /// refused with a canonical denial and writes nothing to the transport.
    fn assert_stale_build_publication_denied(built: &Envelope, view: &SenderSessionView<'_>, reference: &str) {
        let denied_transport = crate::test_support::process_workspace("recorded-denied-publication").expect("workspace");
        let denied_root = open_capability_dataspace_root(&denied_transport).expect("transport capability");
        let receipt = match publish_local_gossip_with_root_and_evidence(&denied_root, built, "node:recorded-sender", Some(view))
            .expect("publication decision") {
            EnvelopePublication::Denied { receipt_value } => receipt_value,
            EnvelopePublication::Published(_) => panic!("an earlier build cannot outlive its introduction"),
        };
        let fields = receipt.collect_simple_record("remote-dataspace-envelope-build-denial-v1", Some(7)).expect("canonical denial");
        assert_eq!(fields[5].as_string().as_deref(), Some(built.envelope_ref.as_str()));
        let diagnostics = fields[6].collect_sequence().expect("denial diagnostics");
        assert!(diagnostics.iter().any(|value| value.as_string().is_some_and(|text| text.contains(reference))));
        assert!(!denied_root.root().try_exists(&envelope_store_path(&built.topic, &built.envelope_ref).expect("path"))
            .expect("observe no publication"));
    }

    #[test]
    fn sender_build_uses_committed_assertions_retractions_and_generation() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        // r[verify molten.runtime_spine.reference_introduction_rule]
        let sender_space = crate::test_support::process_workspace("recorded-sender").expect("sender workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-receiver").expect("receiver workspace");
        let transport = crate::test_support::process_workspace("recorded-transport").expect("transport workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let reference = store_content_blob(&transport, b"committed recorded payload").expect("real content blob");
        let payload = record("ready", vec![string(&reference)]);
        let message = || {
            let mut message = input(Operation::Message, record("use", vec![string(&reference)]));
            // Carried authority/evidence metadata cannot establish peer knowledge.
            message.capability_refs.push(reference.clone());
            message.evidence_refs.push(reference.clone());
            message
        };
        assert_denied(message(), None, &reference);
        let store = RecordedSessionStore::open(&receiver).expect("receiver owner");
        let admitted = store.admit_local(&sender, "recorded-services").expect("capability-bound local admission");
        let view = store.sender_view(&sender, "recorded-services", admitted.generation).expect("live sender view");
        assert_unknown_message_not_committed(&store, &view, admitted.generation, message(), &reference);
        let assertion = build_envelope(input(Operation::Assert, payload.clone()), None).expect("introducing assertion");
        // Merely building an introducing assertion is not receiver commitment.
        assert_denied(message(), Some(&view), &reference);
        store.receive(admitted.generation, &assertion).expect("receiver commits assertion");
        let mut carrying = message();
        carrying.content_refs.push(reference.clone());
        let built = build_envelope(carrying, Some(&view)).expect("introduced message builds");
        publish_local_gossip(&transport, &built, "node:recorded-sender", Some(&view)).expect("actual local publication");
        let delivered = deliver_local_gossip(&transport, "recorded-services", &built.envelope_ref, "node:recorded-receiver")
            .expect("actual local delivery");
        store.receive(admitted.generation, &delivered.envelope).expect("receiver commits known message");
        assert!(view.snapshot().expect("committed receiver state").messages.iter().any(|message|
            message.to == "node:recorded-receiver:inbox" && message.body.as_iovalue() == &delivered.envelope.payload));
        store.commit("node:recorded-sender", "recorded-services", admitted.generation, Operation::Retract, &payload)
            .expect("committed retraction");
        assert_denied(message(), Some(&view), &reference);
        assert_stale_build_publication_denied(&built, &view, &reference);
        store.close("node:recorded-sender", "recorded-services", admitted.generation).expect("close generation");
        assert_source_refused(input(Operation::Message, record("plain", vec![])), &view);
        let next = store.admit_local(&sender, "recorded-services").expect("readmission");
        assert_eq!(next.generation, admitted.generation + 1);
        store.commit("node:recorded-sender", "recorded-services", next.generation, Operation::Assert, &payload)
            .expect("new generation explicitly reintroduces the same object");
        assert_denied(message(), Some(&view), &reference);
        {
            let current = store.sender_view(&sender, "recorded-services", next.generation).expect("new generation view");
            build_envelope(message(), Some(&current)).expect("only the new generation may use its introduction");
        }
        store.commit("node:recorded-sender", "recorded-services", next.generation, Operation::Retract, &payload)
            .expect("withdraw new-generation introduction before reopening");
        drop(view);
        drop(store);
        let reopened = RecordedSessionStore::open(&receiver).expect("reopen durable recording");
        let next_view = reopened.sender_view(&sender, "recorded-services", next.generation).expect("current view");
        assert_denied(message(), Some(&next_view), &reference);
        let bootstrap = recorded_identity(&receiver).expect("real receiver identity").identity_ref;
        build_envelope(input(Operation::Message, record("identity", vec![string(bootstrap)])), Some(&next_view))
            .expect("established bootstrap reference flows without an assertion");
    }

    #[test]
    fn sender_keeps_shared_reference_until_last_assertion_retracts() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        let sender_space = crate::test_support::process_workspace("recorded-shared-sender").expect("workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-shared-receiver").expect("workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let store = RecordedSessionStore::open(&receiver).expect("store");
        let opened = store.admit_local(&sender, "recorded-services").expect("admission");
        let reference = content_ref_from_bytes(b"shared live object");
        let first = record("first", vec![string(&reference)]);
        let second = record("second", vec![string(&reference)]);
        for payload in [&first, &second] {
            store.commit("node:recorded-sender", "recorded-services", opened.generation, Operation::Assert, payload)
                .expect("commit distinct live assertion");
        }
        let view = store.sender_view(&sender, "recorded-services", opened.generation).expect("view");
        let message = || input(Operation::Message, record("use", vec![string(&reference)]));
        store.commit("node:recorded-sender", "recorded-services", opened.generation, Operation::Retract, &first)
            .expect("withdraw first introducing assertion");
        build_envelope(message(), Some(&view)).expect("second live assertion still introduces the object");
        store.commit("node:recorded-sender", "recorded-services", opened.generation, Operation::Retract, &second)
            .expect("withdraw final introducing assertion");
        assert_denied(message(), Some(&view), &reference);
    }

    #[test]
    fn sender_rejects_history_gaps_and_peer_rebinding() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        use redb::ReadableTable;
        let sender_space = crate::test_support::process_workspace("recorded-gap-sender").expect("workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-gap-receiver").expect("workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let store = RecordedSessionStore::open(&receiver).expect("store");
        let opened = store.admit_local(&sender, "recorded-services").expect("admission");
        let view = store.sender_view(&sender, "recorded-services", opened.generation).expect("view");
        let mut wrong_peer = input(Operation::Message, record("plain", vec![]));
        wrong_peer.to_peer = "node:other".to_owned();
        assert_source_refused(wrong_peer, &view);
        let write = store.database.begin_write().expect("corruption transaction");
        {
            let mut table = write.open_table(RECORDED_SESSIONS).expect("table");
            let mut data = {
                let entry = table.get(view.key.as_str()).expect("read").expect("session");
                decode_recorded(entry.value()).expect("decode")
            };
            data.observations[0].sequence = 1;
            table.insert(view.key.as_str(), encode_recorded(&data).expect("encode").as_slice()).expect("inject gap");
        }
        write.commit().expect("commit corruption");
        assert_source_refused(input(Operation::Message, record("plain", vec![])), &view);
    }
