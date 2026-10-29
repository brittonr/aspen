struct ProtectedHost {
    calls: usize,
}

impl crate::wasm_component::EffectRequestHost for ProtectedHost {
    fn invoke(&mut self, _: &[u8]) -> crate::wasm_component::Result<Vec<u8>> {
        self.calls += 1;
        Ok(vec![0x81])
    }
}

fn reference(label: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
}

fn grant() -> crate::wasm_component::ImportGrant {
    crate::wasm_component::ImportGrant {
        import: crate::wasm_component::HOST_REQUEST_IMPORT.to_string(),
        capability: crate::wasm_component::HOST_BLOB_CAPABILITY.to_string(),
        policy_ref: reference("policy"),
        authority_ref: reference("authority"),
        resource_ref: reference("resources"),
        recorded_effect_ref: reference("recording"),
    }
}

fn request(grant: &crate::wasm_component::ImportGrant) -> Vec<u8> {
    let payload = crate::preserves_rail::record("blob-get", vec![crate::preserves_rail::string(reference("blob"))]);
    let effect = crate::effects::effect_request_value(&crate::effects::EffectRequestInput {
        artifact_ref: reference("artifact"),
        effect_id: "blob.get".to_string(),
        operation: "get".to_string(),
        handler_profile: crate::effects::HANDLER_PROFILE_PRODUCTION.to_string(),
        input_ref: crate::preserves_rail::canonical_hash(&payload).expect("payload identity"),
        capability_refs: vec![reference("capability")],
        evidence_refs: Vec::new(),
    })
    .expect("effect request");
    crate::wasm_component::encode_host_request(crate::wasm_component::HostRequestInput {
        generation: 1,
        incarnation: 0,
        sequence: 0,
        grant,
        effect: &effect,
        payload: &payload,
    })
    .expect("canonical host request")
}

#[test]
fn dispatch_requires_the_complete_outer_grant_before_any_host_effect() {
    let grant = grant();
    let bytes = request(&grant);
    let mut host = ProtectedHost { calls: 0 };
    assert_eq!(super::invoke_host(&mut host, &bytes, &grant).expect("matching grant"), vec![0x81]);
    for field in 0..6 {
        let mut outer = grant.clone();
        let target = match field {
            0 => &mut outer.import,
            1 => &mut outer.capability,
            2 => &mut outer.policy_ref,
            3 => &mut outer.authority_ref,
            4 => &mut outer.resource_ref,
            _ => &mut outer.recorded_effect_ref,
        };
        *target = reference("another-outer-binding");
        assert!(super::invoke_host(&mut host, &bytes, &outer).is_err(), "grant field {field}");
    }
    let mut trailing = bytes.clone();
    trailing.push(0x81);
    assert!(super::invoke_host(&mut host, &trailing, &grant).is_err());
    assert!(super::invoke_host(&mut host, &bytes[..bytes.len() - 1], &grant).is_err());
    assert_eq!(host.calls, 1, "unadmitted guest bytes reached the protected host");
}
