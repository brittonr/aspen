fn value(label: &str) -> crate::system_extension::NativeCallbackValue {
    let bytes = crate::preserves_rail::canonical_bytes(&crate::preserves_rail::string(label)).expect("canonical value");
    crate::system_extension::NativeCallbackValue {
        value_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        bytes,
    }
}

fn observation() -> crate::system_extension::HostedObservation {
    let reference = value("reference").value_ref;
    let invocation = crate::system_extension::CallbackInvocation {
        callback: crate::system_extension::CallbackKind::Request,
        generation: 1,
        incarnation: 0,
        sequence: 1,
        event_ref: reference.clone(),
        payload_ref: None,
        logical_tick: 1,
        deadline_tick: 2,
    };
    let context = crate::system_extension::CallbackContext {
        manifest_ref: reference.clone(),
        executable_ref: reference.clone(),
        instance_id: "publication-test".to_string(),
        extension_id: "publication-test".to_string(),
        service_id: "publication-test".to_string(),
        state_ref: None,
        policy_refs: vec![reference.clone()],
        resource_ref: reference,
        port_binding_refs: Vec::new(),
    };
    crate::system_extension::HostedObservation {
        envelope: crate::system_extension::canonical_callback_envelope(
            &context,
            &invocation,
            &crate::system_extension::CallbackInputs {
                payload: None,
                state: None,
            },
        )
        .expect("envelope"),
        execution: crate::wasm_component::ExecutionOutcome {
            decision: crate::wasm_component::Decision::Pass,
            output: None,
            receipts: Vec::new(),
            diagnostics: Vec::new(),
        },
        host_records: crate::wasm_component::consumer::host::Recording::empty().snapshot(),
        materialized_outcome: None,
        publication_refs: Vec::with_capacity(super::super::publication::MAX_CALLBACK_PUBLICATIONS),
        publications: Vec::with_capacity(super::super::publication::MAX_CALLBACK_PUBLICATIONS),
        is_publication_uncertain: false,
        is_committed: false,
    }
}

struct PublicationPort {
    mode: u8,
    calls: usize,
}

impl crate::system_extension::NativeCallbackValuePort for PublicationPort {
    fn materialize(
        &mut self,
        _: &str,
        _: u64,
    ) -> Result<crate::system_extension::NativeCallbackValue, crate::system_extension::ValuePortFailure> {
        unreachable!("publication does not materialize")
    }

    fn publish(
        &mut self,
        value: &crate::system_extension::NativeCallbackValue,
        _: u64,
    ) -> Result<crate::system_extension::ValuePublicationReceipt, crate::system_extension::ValuePortFailure> {
        self.calls += 1;
        match self.mode {
            0 => Err(crate::system_extension::ValuePortFailure::new(
                crate::system_extension::ValuePortFailureKind::RejectedBeforeAcceptance,
                "definitely not applied",
            )),
            1 => Err(crate::system_extension::ValuePortFailure::new(
                crate::system_extension::ValuePortFailureKind::UnknownAfterAcceptance,
                "accepted without completion",
            )),
            mode => Ok(crate::system_extension::ValuePublicationReceipt {
                value_ref: if mode == 2 {
                    self::value("another-value").value_ref
                } else {
                    value.value_ref.clone()
                },
                publication_ref: if mode == 3 {
                    "not-a-content-ref".to_string()
                } else {
                    self::value("publication").value_ref
                },
                byte_count: value.bytes.len() as u64 + u64::from(mode == 4),
            }),
        }
    }
}

#[test]
fn uncertain_or_invalid_publications_retain_attempt_and_stop_later_values() {
    let outcome = crate::system_extension::MaterializedCallbackOutcome {
        outputs: vec![value("first"), value("must-not-publish")],
        effects: Vec::new(),
        state: None,
        checkpoint: None,
        health: crate::system_extension::HealthState::Healthy,
    };
    for mode in 0..5 {
        let mut port = PublicationPort { mode, calls: 0 };
        let mut observed = observation();
        assert!(super::super::publication::publish_outcome_values(&mut port, &outcome, 65_536, &mut observed).is_err());
        assert_eq!(port.calls, 1, "a later value was published after failure");
        assert_eq!(observed.publications[0].value_ref, outcome.outputs[0].value_ref);
        assert_eq!(observed.publications[0].byte_count, outcome.outputs[0].bytes.len() as u64);
        assert_eq!(observed.publications[0].receipt.is_some(), mode >= 2);
        assert_eq!(
            observed.publications[0]
                .failure
                .as_ref()
                .expect("structured publication failure")
                .may_have_published(),
            mode != 0
        );
        assert_eq!(observed.is_publication_uncertain, mode != 0);
        assert!(observed.publication_refs.is_empty());
        assert!(!observed.is_committed);
    }
}
