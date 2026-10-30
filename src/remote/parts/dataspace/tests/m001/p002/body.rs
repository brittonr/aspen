    #[test]
    fn introduction_is_not_a_live_receiver_acknowledgment() {
        // r[verify molten.remote_dataspace.recorded_session_source]
        let sender_space = crate::test_support::process_workspace("recorded-live-sender").expect("workspace");
        let receiver_space = crate::test_support::process_workspace("recorded-live-receiver").expect("workspace");
        let sender = init(&sender_space, "node:recorded-sender");
        let receiver = init(&receiver_space, "node:recorded-receiver");
        let store = RecordedSessionStore::open(&receiver).expect("store");
        let opened = store.admit_local(&sender, "recorded-services").expect("admission");
        let view = store.sender_view(&sender, "recorded-services", opened.generation).expect("view");
        let reference = content_ref_from_bytes(b"recorded live object");
        let payload = record("ready", vec![string(&reference)]);
        store
            .commit("node:recorded-sender", "recorded-services", opened.generation, Operation::Assert, &payload)
            .expect("committed local introduction");
        let built = build_envelope(input(Operation::Message, record("use", vec![string(&reference)])), Some(&view))
            .expect("locally introduced message builds");
        assert!(outbound_envelope_denial(&built, Some(&view)).expect("bound decision").is_none());
        // The unbound live entrypoint holds no receiver observation, so the local recording cannot vouch for it.
        let receipt = outbound_envelope_denial(&built, None)
            .expect("unbound decision")
            .expect("an unbound live publication must refuse a referenced envelope");
        let fields = receipt
            .collect_simple_record("remote-dataspace-envelope-build-denial-v1", Some(7))
            .expect("canonical denial");
        assert_eq!(fields[1].as_string().as_deref(), Some("deny"));
        let diagnostics = fields[6].collect_sequence().expect("denial diagnostics");
        assert!(diagnostics.iter().any(|value| value.as_string().is_some_and(|text| text.contains(&reference))));
        let plain = build_envelope(input(Operation::Message, record("plain", vec![])), None).expect("reference-free");
        assert!(outbound_envelope_denial(&plain, None).expect("reference-free decision").is_none());
    }
