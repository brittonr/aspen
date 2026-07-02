
    fn setup() -> Env {
        Env {
            scope: scope(Some(fake_ref("actor-chaos-profile"))),
            policy_ref: fake_ref("chaos-policy"),
            capability_ref: fake_ref("chaos-capability"),
            resource_refs: vec![fake_ref("chaos-resource-bound")],
        }
    }

    fn chaos_binding(env: &Env) -> IoValue {
        handler_binding_value(&HandlerBindingInput {
            profile: HANDLER_PROFILE_CHAOS.to_string(),
            scope: env.scope.clone(),
            adapter_kind: ADAPTER_KIND_DATASPACE.to_string(),
            adapter_ref: fake_ref("chaos-dataspace-adapter"),
            executor_preflight_ref: None,
            policy_ref: env.policy_ref.clone(),
            capability_context_ref: env.capability_ref.clone(),
            context_ref: None,
            resource_refs: env.resource_refs.clone(),
            operations: vec!["delay".to_string(), "partition".to_string(), "reorder".to_string()],
            evidence_refs: vec![fake_ref("chaos-schedule-evidence")],
        })
        .expect("chaos binding")
    }

    fn chaos_handle(env: &Env, binding_ref: String) -> IoValue {
        effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_DATASPACE.to_string(),
            scope: env.scope.clone(),
            handler_binding_ref: binding_ref,
            operations: vec!["delay".to_string(), "partition".to_string(), "reorder".to_string()],
            capability_context_ref: env.capability_ref.clone(),
            context_ref: None,
            resource_refs: env.resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(100),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("bounded-chaos-handle")],
        })
        .expect("chaos handle")
    }

    fn assert_chaos_variant(env: &Env) {
        let binding = chaos_binding(env);
        let binding_ref = canonical_hash(&binding).expect("chaos binding ref");
        let profile = handler_profile_value(&HandlerProfileInput {
            profile: HANDLER_PROFILE_CHAOS.to_string(),
            handler_binding_refs: vec![binding_ref.clone()],
            policy_ref: env.policy_ref.clone(),
            capability_context_ref: env.capability_ref.clone(),
            resource_refs: env.resource_refs.clone(),
            evidence_refs: vec![fake_ref("bounded-chaos-profile")],
        })
        .expect("chaos profile");
        assert_eq!(parse_handler_profile(&profile).expect("parse chaos profile").profile, HANDLER_PROFILE_CHAOS);
        let handle = chaos_handle(env, binding_ref);
        validate_handle_for_request(&binding, &handle, &EffectHandleRequest {
            kind: ADAPTER_KIND_DATASPACE,
            operation: "delay",
            run_ref: &env.scope.run_ref,
            session_ref: &env.scope.session_ref,
            actor_ref: env.scope.actor_ref.as_deref(),
            turn_ref: env.scope.turn_ref.as_deref(),
            policy_ref: &env.policy_ref,
            capability_context_ref: &env.capability_ref,
            context_ref: None,
            resource_refs: &env.resource_refs,
            logical_time: 50,
            remote_use: false,
            revoked_refs: &[],
        })
        .expect("bounded chaos delay validates");
    }

    fn metric_binding(env: &Env) -> IoValue {
        handler_binding_value(&HandlerBindingInput {
            profile: HANDLER_PROFILE_PROFILING.to_string(),
            scope: env.scope.clone(),
            adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
            adapter_ref: fake_ref("profiling-storage-adapter"),
            executor_preflight_ref: None,
            policy_ref: env.policy_ref.clone(),
            capability_context_ref: env.capability_ref.clone(),
            context_ref: None,
            resource_refs: env.resource_refs.clone(),
            operations: vec!["count".to_string(), "payload-bytes".to_string()],
            evidence_refs: vec![fake_ref("profiling-evidence")],
        })
        .expect("profiling binding")
    }

    fn assert_metric_variant(env: &Env) {
        let binding = metric_binding(env);
        let profile = handler_profile_value(&HandlerProfileInput {
            profile: HANDLER_PROFILE_PROFILING.to_string(),
            handler_binding_refs: vec![canonical_hash(&binding).expect("profiling binding ref")],
            policy_ref: env.policy_ref.clone(),
            capability_context_ref: env.capability_ref.clone(),
            resource_refs: env.resource_refs.clone(),
            evidence_refs: vec![fake_ref("profiling-profile-evidence")],
        })
        .expect("profiling profile");
        assert_eq!(
            parse_handler_profile(&profile).expect("parse profiling profile").profile,
            HANDLER_PROFILE_PROFILING
        );
        let record = dynamic_operation_record_value(&DynamicOperationRecordInput {
            operation: "count".to_string(),
            adapter_ref: fake_ref("profiling-storage-adapter"),
            callable_ref: fake_ref("profiling-callable"),
            request_ref: fake_ref("profiling-request"),
            response_ref: fake_ref("profiling-response"),
            policy_ref: env.policy_ref.clone(),
            capability_context_ref: env.capability_ref.clone(),
            resource_refs: env.resource_refs.clone(),
            evidence_refs: vec![fake_ref("effect-counts-and-payload-bytes")],
        })
        .expect("profiling record");
        assert_eq!(parse_dynamic_operation_record(&record).expect("parse profiling record").operation, "count");
    }

    #[test]
    fn chaos_and_profiling_profiles_are_bounded_evidence_only() {
        let env = setup();
        assert_chaos_variant(&env);
        assert_metric_variant(&env);
    }

    #[test]
    fn adapter_profile_kinds_bind_storage_blob_network_remote_and_replay_handles() {
        for kind in [
            ADAPTER_KIND_STORAGE,
            ADAPTER_KIND_BLOB,
            ADAPTER_KIND_NETWORK,
            ADAPTER_KIND_REMOTE_SYNC,
            ADAPTER_KIND_REPLAY_RECORD,
        ] {
            let scope = scope(Some(fake_ref(&format!("actor-{kind}"))));
            let policy_ref = fake_ref(&format!("policy-{kind}"));
            let capability_ref = fake_ref(&format!("capability-{kind}"));
            let resource_refs = vec![fake_ref(&format!("resource-{kind}"))];
            let binding = handler_binding_value(&HandlerBindingInput {
                profile: format!("{kind}-adapter-profile"),
                scope: scope.clone(),
                adapter_kind: kind.to_string(),
                adapter_ref: fake_ref(&format!("adapter-{kind}")),
                executor_preflight_ref: None,
                policy_ref: policy_ref.clone(),
                capability_context_ref: capability_ref.clone(),
                context_ref: None,
                resource_refs: resource_refs.clone(),
                operations: vec!["open".to_string()],
                evidence_refs: vec![fake_ref(&format!("evidence-{kind}"))],
            })
            .expect("adapter binding");
            let handle = effect_handle_value(&EffectHandleInput {
                kind: kind.to_string(),
                scope: scope.clone(),
                handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
                operations: vec!["open".to_string()],
                capability_context_ref: capability_ref.clone(),
                context_ref: None,
                resource_refs: resource_refs.clone(),
                not_before: None,
                expires_at: None,
                revocation_refs: Vec::new(),
                transfer: TRANSFER_LOCAL_ONLY.to_string(),
                parent_handle_ref: None,
                evidence_refs: vec![fake_ref(&format!("handle-evidence-{kind}"))],
            })
            .expect("adapter handle");
            validate_handle_for_request(&binding, &handle, &EffectHandleRequest {
                kind,
                operation: "open",
                run_ref: &scope.run_ref,
                session_ref: &scope.session_ref,
                actor_ref: scope.actor_ref.as_deref(),
                turn_ref: scope.turn_ref.as_deref(),
                policy_ref: &policy_ref,
                capability_context_ref: &capability_ref,
                context_ref: None,
                resource_refs: &resource_refs,
                logical_time: 0,
                remote_use: false,
                revoked_refs: &[],
            })
            .expect("adapter handle validates");
        }
    }

    #[test]
    fn remote_proxy_handles_require_transfer_profile_and_explicit_refs() {
        let actor_ref = fake_ref("actor-a");
        let scope = scope(Some(actor_ref));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_refs = vec![fake_ref("resource")];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "remote-proxy".to_string(),
            scope: scope.clone(),
            adapter_kind: ADAPTER_KIND_REMOTE_SYNC.to_string(),
            adapter_ref: fake_ref("remote-adapter"),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["sync".to_string()],
            evidence_refs: vec![
                fake_ref("peer-agreement"),
                fake_ref("node-identity"),
                fake_ref("revocation-policy"),
            ],
        })
        .expect("remote binding");
        let handle = effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_REMOTE_SYNC.to_string(),
            scope: scope.clone(),
            handler_binding_ref: canonical_hash(&binding).expect("binding ref"),
            operations: vec!["sync".to_string()],
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(5),
            revocation_refs: vec![fake_ref("remote-revocation")],
            transfer: TRANSFER_REMOTE_PROXY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![
                fake_ref("peer-agreement"),
                fake_ref("node-identity"),
                fake_ref("revocation-policy"),
            ],
        })
        .expect("remote handle");
        validate_handle_for_request(&binding, &handle, &EffectHandleRequest {
            kind: ADAPTER_KIND_REMOTE_SYNC,
            operation: "sync",
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
        })
        .expect("remote proxy handle validates for remote use");
    }
