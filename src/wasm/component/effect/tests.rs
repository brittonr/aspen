// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn carrier_fences_prevent_reusing_a_response_for_the_same_operation() {
    let case = Case::new();
    let first = super::super::decode_host_request(&case.bytes(7)).expect("first carrier");
    let next = super::super::decode_host_request(&case.bytes(8)).expect("next carrier");
    assert_eq!(first.effect().request_ref, next.effect().request_ref);
    assert_ne!(first.request_ref(), next.request_ref());
    assert_eq!((first.generation(), first.incarnation(), first.sequence()), (3, 5, 7));
    let output = crate::preserves_rail::string("blob contents");
    let response = response_value(&first.effect().request_ref, Some(&output));
    let bytes = super::super::encode_host_response(super::super::HostResponseInput {
        request: &first,
        response: &response,
        output: Some(&output),
    })
    .expect("bound response");
    let decoded = super::super::decode_host_response(&bytes, &first).expect("matching carrier");
    assert_eq!(decoded.request_ref(), first.request_ref());
    assert_eq!(decoded.response().request_ref, first.effect().request_ref);
    assert_eq!(decoded.output(), Some(&output));
    assert!(super::super::decode_host_response(&bytes, &next).is_err());
}

// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn closed_operation_and_materialized_identity_are_required_on_intake() {
    for index in 0..5 {
        let mut case = Case::new();
        match index {
            0 => case.grant.capability = "hostcall:filesystem".to_string(),
            1 => case.grant.policy_ref = "not-a-content-ref".to_string(),
            2 => case.operation.effect_id = "blob.put".to_string(),
            3 => case.operation.input_ref = reference("another-input"),
            4 => case.operation.capability_refs.clear(),
            _ => unreachable!("closed case inventory"),
        }
        let bytes = crate::preserves_rail::canonical_bytes(&case.raw(7)).expect("canonical malformed carrier");
        assert!(super::super::decode_host_request(&bytes).is_err(), "case {index}");
    }
}

// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn malformed_and_noncanonical_carriers_do_not_decode() {
    let case = Case::new();
    let canonical = case.bytes(7);
    let mut trailing = canonical.clone();
    trailing.extend_from_slice(&canonical);
    assert!(super::super::decode_host_request(&trailing).is_err());
    assert!(super::super::decode_host_request(&canonical[..canonical.len() - 1]).is_err());
    let mut noncanonical = canonical;
    let position = noncanonical.windows(2).position(|bytes| bytes == [0xb0, 1]).expect("one-byte integer");
    noncanonical[position + 1] = 2;
    noncanonical.insert(position + 2, 0);
    assert!(super::super::decode_host_request(&noncanonical).is_err());
}

// r[verify molten.wasm_component_adoption.hostcalls]
#[test]
fn deny_has_no_output_and_success_requires_the_exact_materialized_value() {
    let request = super::super::decode_host_request(&Case::new().bytes(7)).expect("request carrier");
    let deny = response_value(&request.effect().request_ref, None);
    let bytes = super::super::encode_host_response(super::super::HostResponseInput {
        request: &request,
        response: &deny,
        output: None,
    })
    .expect("deny carrier");
    let decoded = super::super::decode_host_response(&bytes, &request).expect("bound deny");
    assert_eq!(decoded.response().decision, "deny");
    assert!(decoded.output().is_none());
    let output = crate::preserves_rail::string("admitted output");
    let stale = crate::preserves_rail::string("different output");
    let pass = response_value(&request.effect().request_ref, Some(&output));
    for (response, materialized) in [(&pass, None), (&pass, Some(&stale)), (&deny, Some(&output))] {
        let raw = raw_response(&request, response, materialized);
        let bytes = crate::preserves_rail::canonical_bytes(&raw).expect("canonical invalid response");
        assert!(super::super::decode_host_response(&bytes, &request).is_err());
    }
    let another = response_value(&reference("unrelated-operation"), Some(&output));
    let raw = raw_response(&request, &another, Some(&output));
    let bytes = crate::preserves_rail::canonical_bytes(&raw).expect("canonical wrong nested binding");
    assert!(super::super::decode_host_response(&bytes, &request).is_err());
}

struct Case {
    grant: super::super::ImportGrant,
    operation: crate::effects::EffectRequestInput,
    payload: preserves::IOValue,
}

impl Case {
    fn new() -> Self {
        let payload = crate::preserves_rail::record("blob-get", vec![crate::preserves_rail::string(reference("blob"))]);
        Self {
            grant: super::super::ImportGrant {
                import: super::super::HOST_REQUEST_IMPORT.to_string(),
                capability: super::super::HOST_BLOB_CAPABILITY.to_string(),
                policy_ref: reference("policy"),
                authority_ref: reference("authority"),
                resource_ref: reference("resources"),
                recorded_effect_ref: reference("recording"),
            },
            operation: crate::effects::EffectRequestInput {
                artifact_ref: reference("artifact"),
                effect_id: "blob.get".to_string(),
                operation: "get".to_string(),
                handler_profile: crate::effects::HANDLER_PROFILE_PRODUCTION.to_string(),
                input_ref: crate::preserves_rail::canonical_hash(&payload).expect("input identity"),
                capability_refs: vec![reference("capability")],
                evidence_refs: Vec::new(),
            },
            payload,
        }
    }

    fn bytes(&self, sequence: u64) -> Vec<u8> {
        let value = crate::effects::effect_request_value(&self.operation).expect("operation request");
        super::super::encode_host_request(super::super::HostRequestInput {
            generation: 3,
            incarnation: 5,
            sequence,
            grant: &self.grant,
            effect: &value,
            payload: &self.payload,
        })
        .expect("request carrier")
    }

    fn raw(&self, sequence: u64) -> preserves::IOValue {
        let grant = crate::preserves_rail::record(
            "import-grant",
            [
                &self.grant.import,
                &self.grant.capability,
                &self.grant.policy_ref,
                &self.grant.authority_ref,
                &self.grant.resource_ref,
                &self.grant.recorded_effect_ref,
            ]
            .map(crate::preserves_rail::string)
            .to_vec(),
        );
        crate::preserves_rail::record("component-effect-request-v1", vec![
            crate::preserves_rail::string(super::super::HOST_REQUEST_SCHEMA),
            crate::preserves_rail::u64_value(3),
            crate::preserves_rail::u64_value(5),
            crate::preserves_rail::u64_value(sequence),
            grant,
            crate::effects::effect_request_value(&self.operation).expect("operation shape"),
            self.payload.clone(),
        ])
    }
}

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn response_value(request_ref: &str, output: Option<&preserves::IOValue>) -> preserves::IOValue {
    crate::effects::effect_response_value(&crate::effects::EffectResponseInput {
        request_ref: request_ref.to_string(),
        decision: if output.is_some() { "pass" } else { "deny" }.to_string(),
        output_ref: output.map(crate::preserves_rail::canonical_hash).transpose().expect("output identity"),
        diagnostics: Vec::new(),
        evidence_refs: vec![reference("response-evidence")],
    })
    .expect("canonical response")
}

fn raw_response(
    request: &super::super::HostRequest,
    response: &preserves::IOValue,
    output: Option<&preserves::IOValue>,
) -> preserves::IOValue {
    crate::preserves_rail::record("component-effect-response-v1", vec![
        crate::preserves_rail::string(super::super::HOST_RESPONSE_SCHEMA),
        crate::preserves_rail::string(request.request_ref()),
        response.clone(),
        match output {
            Some(value) => crate::preserves_rail::record("some", vec![value.clone()]),
            None => crate::preserves_rail::record("none", Vec::new()),
        },
    ])
}
