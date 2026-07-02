
    #[test]
    fn negative_security_denies_stale_revoked_wrong_scope_and_wrong_refs() {
        let (binding, handle, scope, policy_ref, capability_ref) = binding_and_handle("send");
        let parsed = parse_effect_handle(&handle).expect("parse handle");
        let base = EffectHandleRequest {
            kind: "hostcall",
            operation: "send",
            run_ref: &scope.run_ref,
            session_ref: &scope.session_ref,
            actor_ref: scope.actor_ref.as_deref(),
            turn_ref: scope.turn_ref.as_deref(),
            policy_ref: &policy_ref,
            capability_context_ref: &capability_ref,
            context_ref: None,
            resource_refs: &parsed.resource_refs,
            logical_time: 1,
            remote_use: false,
            revoked_refs: &[],
        };
        validate_handle_for_request(&binding, &handle, &base).expect("base request validates");

        let stale = EffectHandleRequest {
            logical_time: 10,
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &stale)
                .expect_err("expired handle denied")
                .to_string()
                .contains("expired")
        );

        let revoked = EffectHandleRequest {
            revoked_refs: &parsed.revocation_refs,
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &revoked)
                .expect_err("revoked handle denied")
                .to_string()
                .contains("revoked")
        );

        let wrong_actor = fake_ref("other-actor");
        let wrong_scope = EffectHandleRequest {
            actor_ref: Some(&wrong_actor),
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &wrong_scope)
                .expect_err("wrong actor denied")
                .to_string()
                .contains("actor scope")
        );

        let wrong_capability = fake_ref("other-capability");
        let wrong_refs = EffectHandleRequest {
            capability_context_ref: &wrong_capability,
            ..base.clone()
        };
        assert!(
            validate_handle_for_request(&binding, &handle, &wrong_refs)
                .expect_err("wrong capability denied")
                .to_string()
                .contains("capability context")
        );
    }

    #[test]
    fn local_only_handle_denies_remote_use() {
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
            remote_use: true,
            revoked_refs: &[],
        };
        let error = validate_handle_for_request(&binding, &handle, &request).expect_err("remote use denied");
        assert!(error.to_string().contains("local-only"), "{error}");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_effect_handle_identity_attenuation_and_replay_stability(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let is_write_operation = tc.draw(hegel::generators::booleans());
        let operation = if is_write_operation { "write" } else { "read" };
        let actor_ref = fake_ref(&format!("actor-{salt}"));
        let scope = EffectScope {
            run_ref: fake_ref(&format!("run-{salt}")),
            session_ref: fake_ref(&format!("session-{salt}")),
            actor_ref: Some(actor_ref),
            turn_ref: Some(fake_ref(&format!("turn-{salt}"))),
        };
        let policy_ref = fake_ref(&format!("policy-{salt}"));
        let capability_ref = fake_ref(&format!("capability-{salt}"));
        let resource_refs = vec![fake_ref(&format!("resource-{salt}"))];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "property".to_string(),
            scope: scope.clone(),
            adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
            adapter_ref: fake_ref(&format!("adapter-{salt}")),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["read".to_string(), "write".to_string()],
            evidence_refs: vec![fake_ref(&format!("evidence-{salt}"))],
        })
        .expect("binding");
        let parent = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_STORAGE.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
            operations: vec!["read".to_string(), "write".to_string()],
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref(&format!("parent-evidence-{salt}"))],
        })
        .expect("parent");
        let repeated_parent = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_STORAGE.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref again"),
            operations: vec!["read".to_string(), "write".to_string()],
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref(&format!("parent-evidence-{salt}"))],
        })
        .expect("repeated parent");
        assert_eq!(canonical_hash(&parent).unwrap(), canonical_hash(&repeated_parent).unwrap());
        let child = attenuated_handle_value(&parent, &HandleAttenuationInput {
            scope: scope.clone(),
            operations: vec![operation.to_string()],
            expires_at: Some(5),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            evidence_refs: vec![fake_ref(&format!("attenuation-{salt}"))],
        })
        .expect("attenuated child");
        validate_handle_for_request(&binding, &child, &EffectHandleRequest {
            kind: ADAPTER_KIND_STORAGE,
            operation,
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
        })
        .expect("attenuated child validates");
        let other = if is_write_operation { "read" } else { "write" };
        assert!(
            validate_handle_for_request(&binding, &child, &EffectHandleRequest {
                kind: ADAPTER_KIND_STORAGE,
                operation: other,
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
            })
            .is_err()
        );
    }
