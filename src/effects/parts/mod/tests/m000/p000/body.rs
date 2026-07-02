    use super::*;

    fn fake_ref(label: &str) -> String {
        canonical_hash(&record("fake-ref", vec![string(label)])).expect("hash fake ref")
    }

    fn scope(actor_ref: Option<String>) -> EffectScope {
        EffectScope {
            run_ref: fake_ref("run"),
            session_ref: fake_ref("session"),
            actor_ref,
            turn_ref: Some(fake_ref("turn")),
        }
    }

    fn declared_effect(effect_id: &str, operation: &str) -> DeclaredEffect {
        DeclaredEffect {
            effect_id: effect_id.to_string(),
            operation: operation.to_string(),
            input_schema_ref: fake_ref("input-schema"),
            output_schema_ref: fake_ref("output-schema"),
            evidence_refs: vec![fake_ref("effect-evidence")],
        }
    }

    fn manifest_profile_and_request(effect_id: &str, operation: &str) -> (IoValue, IoValue, IoValue) {
        let artifact_ref = fake_ref("artifact");
        let manifest = effect_manifest_value(&EffectManifestInput {
            artifact_kind: "wasm".to_string(),
            artifact_ref: artifact_ref.clone(),
            executor_kind: "wasm".to_string(),
            declared_effects: vec![declared_effect(effect_id, operation)],
            policy_refs: vec![fake_ref("policy")],
            evidence_refs: vec![fake_ref("manifest-evidence")],
        })
        .expect("effect manifest");
        let profile = handler_profile_value(&HandlerProfileInput {
            profile: HANDLER_PROFILE_LOCAL.to_string(),
            handler_binding_refs: vec![fake_ref("handler-binding")],
            policy_ref: fake_ref("policy"),
            capability_context_ref: fake_ref("capability"),
            resource_refs: vec![fake_ref("resource")],
            evidence_refs: vec![fake_ref("profile-evidence")],
        })
        .expect("handler profile");
        let request = effect_request_value(&EffectRequestInput {
            artifact_ref,
            effect_id: effect_id.to_string(),
            operation: operation.to_string(),
            handler_profile: HANDLER_PROFILE_LOCAL.to_string(),
            input_ref: fake_ref("input"),
            capability_refs: vec![fake_ref("capability")],
            evidence_refs: vec![fake_ref("request-evidence")],
        })
        .expect("effect request");
        (manifest, profile, request)
    }

    fn binding_and_handle(operation: &str) -> (IoValue, IoValue, EffectScope, String, String) {
        let actor_ref = fake_ref("actor-a");
        let scope = scope(Some(actor_ref.clone()));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_refs = vec![fake_ref("resource")];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "local".to_string(),
            scope: scope.clone(),
            adapter_kind: "hostcall".to_string(),
            adapter_ref: fake_ref("adapter"),
            executor_preflight_ref: Some(fake_ref("executor-preflight")),
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec![operation.to_string()],
            evidence_refs: vec![fake_ref("evidence")],
        })
        .expect("handler binding");
        let binding_ref = canonical_hash(&binding).expect("binding ref");
        let handle = effect_handle_value(&EffectHandleInput {
            kind: "hostcall".to_string(),
            scope: scope.clone(),
            handler_binding_ref: binding_ref,
            operations: vec![operation.to_string()],
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: vec![fake_ref("revocation")],
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("evidence")],
        })
        .expect("effect handle");
        (binding, handle, scope, policy_ref, capability_ref)
    }

    fn storage_pair(
        scope: &EffectScope,
        policy_ref: &str,
        capability_ref: &str,
        suffix: &str,
    ) -> (IoValue, IoValue, Vec<String>) {
        let resource_refs = vec![fake_ref(&format!("resource-{suffix}"))];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "local".to_string(),
            scope: scope.clone(),
            adapter_kind: "storage".to_string(),
            adapter_ref: fake_ref(&format!("storage-{suffix}")),
            executor_preflight_ref: None,
            policy_ref: policy_ref.to_string(),
            capability_context_ref: capability_ref.to_string(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["read".to_string()],
            evidence_refs: vec![fake_ref(&format!("evidence-{suffix}"))],
        })
        .expect("storage binding");
        let handle = effect_handle_value(&EffectHandleInput {
            kind: "storage".to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("storage binding ref"),
            operations: vec!["read".to_string()],
            capability_context_ref: capability_ref.to_string(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: None,
            expires_at: None,
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref(&format!("evidence-{suffix}"))],
        })
        .expect("storage handle");
        (binding, handle, resource_refs)
    }

    #[test]
    fn effect_manifest_profile_request_and_response_roundtrip() {
        let (manifest, profile, request) = manifest_profile_and_request("dataspace.send", "send");
        let parsed_manifest = parse_effect_manifest(&manifest).expect("parse manifest");
        assert_eq!(parsed_manifest.declared_effects[0].effect_id, "dataspace.send");
        assert_eq!(parsed_manifest.executor_kind, "wasm");
        assert!(parsed_manifest.checks.iter().any(|check| check == "deny-undeclared-effects"));
        let parsed_profile = parse_handler_profile(&profile).expect("parse profile");
        assert_eq!(parsed_profile.profile, HANDLER_PROFILE_LOCAL);
        let parsed_request = parse_effect_request(&request).expect("parse request");
        assert_eq!(parsed_request.operation, "send");
        let response = effect_response_value(&EffectResponseInput {
            request_ref: parsed_request.request_ref.clone(),
            decision: "pass".to_string(),
            output_ref: Some(fake_ref("output")),
            diagnostics: Vec::new(),
            evidence_refs: vec![fake_ref("response-evidence")],
        })
        .expect("effect response");
        let parsed_response = parse_effect_response(&response).expect("parse response");
        assert_eq!(parsed_response.decision, "pass");
        assert_eq!(parsed_response.request_ref, parsed_request.request_ref);
    }

    #[test]
    fn effect_binding_receipt_denies_undeclared_effects() {
        let (manifest, profile, request) = manifest_profile_and_request("dataspace.send", "send");
        let pass = admit_effect_request(&manifest, &profile, &request, &[fake_ref("admission-evidence")])
            .expect("admit declared effect");
        assert_eq!(pass.decision, "pass");
        assert!(pass.checks.iter().any(|check| check == "deny-undeclared-effects"));

        let request = effect_request_value(&EffectRequestInput {
            artifact_ref: parse_effect_manifest(&manifest).expect("manifest").artifact_ref,
            effect_id: "blob.get".to_string(),
            operation: "get".to_string(),
            handler_profile: HANDLER_PROFILE_LOCAL.to_string(),
            input_ref: fake_ref("input"),
            capability_refs: vec![fake_ref("capability")],
            evidence_refs: vec![fake_ref("request-evidence")],
        })
        .expect("undeclared request");
        let deny = admit_effect_request(&manifest, &profile, &request, &[fake_ref("admission-evidence")])
            .expect("deny undeclared effect");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("not declared")));
    }

    #[test]
    fn effect_manifest_rejects_duplicate_and_malformed_effect_ids() {
        let artifact_ref = fake_ref("artifact");
        let duplicate = effect_manifest_value(&EffectManifestInput {
            artifact_kind: "steel".to_string(),
            artifact_ref: artifact_ref.clone(),
            executor_kind: "steel".to_string(),
            declared_effects: vec![
                declared_effect("storage.read", "read"),
                declared_effect("storage.read", "read"),
            ],
            policy_refs: vec![fake_ref("policy")],
            evidence_refs: vec![fake_ref("manifest-evidence")],
        })
        .expect_err("duplicate effect denied");
        assert!(duplicate.to_string().contains("duplicate declared effect"), "{duplicate}");
        let malformed = effect_manifest_value(&EffectManifestInput {
            artifact_kind: "steel".to_string(),
            artifact_ref,
            executor_kind: "steel".to_string(),
            declared_effects: vec![declared_effect("Storage.Read", "read")],
            policy_refs: vec![fake_ref("policy")],
            evidence_refs: vec![fake_ref("manifest-evidence")],
        })
        .expect_err("malformed effect id denied");
        assert!(malformed.to_string().contains("effect id"), "{malformed}");
    }

    #[test]
    fn handle_identity_is_canonical_and_replayable() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let resource_refs = parse_effect_handle(&handle).expect("parse handle").resource_refs;
        let request = EffectHandleRequest {
            kind: "hostcall",
            operation: "send",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        };
        let validation = validate_handle_for_request(&binding, &handle, &request).expect("validate handle");
        assert_eq!(validation.handler_binding_ref, canonical_hash(&binding).expect("binding ref"));
        assert_eq!(validation.handle_ref, canonical_hash(&handle).expect("handle ref"));
        assert!(validation.checks.iter().any(|check| check == "handle-not-authority"));
    }

    #[test]
    fn handle_ref_alone_does_not_grant_wrong_operation() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let resource_refs = parse_effect_handle(&handle).expect("parse handle").resource_refs;
        let request = EffectHandleRequest {
            kind: "hostcall",
            operation: "assert",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            context_ref: None,
            resource_refs: &resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        };
        let error = validate_handle_for_request(&binding, &handle, &request).expect_err("wrong operation denied");
        assert!(error.to_string().contains("operation"), "{error}");
    }

    #[test]
    fn same_kind_handles_in_one_scope_are_disambiguated_by_refs() {
        let scope = scope(Some(fake_ref("actor-a")));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let (binding_a, handle_a, resource_a) = storage_pair(&scope, &policy_ref, &capability_ref, "a");
        let (binding_b, handle_b, _) = storage_pair(&scope, &policy_ref, &capability_ref, "b");

        assert_ne!(canonical_hash(&handle_a).unwrap(), canonical_hash(&handle_b).unwrap());
        let request_a = EffectHandleRequest {
            kind: "storage",
            operation: "read",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            context_ref: None,
            resource_refs: &resource_a,
            logical_time: 0,
            remote_use: false,
            revoked_refs: &[],
        };
        validate_handle_for_request(&binding_a, &handle_a, &request_a).expect("storage a handle validates");
        let error = validate_handle_for_request(&binding_b, &handle_b, &request_a)
            .expect_err("storage b cannot satisfy storage a request refs");
        assert!(error.to_string().contains("resource refs"), "{error}");
    }

    #[test]
    fn compound_dynamic_attenuation_and_cleanup_artifacts_parse() {
        let bundle = bundle();
        check_profile(&bundle);
        check_dynamic(&bundle.seed);
        check_cleanup(&bundle.child);
    }
