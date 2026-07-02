
    struct Seed {
        scope: EffectScope,
        policy_ref: String,
        capability_ref: String,
        resource_refs: Vec<String>,
        binding_ref: String,
    }

    struct Bundle {
        seed: Seed,
        parent: IoValue,
        child: IoValue,
    }

    fn seed() -> Seed {
        let scope = scope(Some(fake_ref("actor-a")));
        let policy_ref = fake_ref("policy");
        let capability_ref = fake_ref("capability");
        let resource_refs = vec![fake_ref("resource")];
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: "compound".to_string(),
            scope: scope.clone(),
            adapter_kind: ADAPTER_KIND_STORAGE.to_string(),
            adapter_ref: fake_ref("storage-adapter"),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: vec!["read".to_string(), "write".to_string()],
            evidence_refs: vec![fake_ref("binding-evidence")],
        })
        .expect("binding");
        let binding_ref = canonical_hash(&binding).expect("binding ref");
        Seed {
            scope,
            policy_ref,
            capability_ref,
            resource_refs,
            binding_ref,
        }
    }

    fn parent(seed: &Seed) -> IoValue {
        effect_handle_value(&EffectHandleInput {
            kind: ADAPTER_KIND_STORAGE.to_string(),
            scope: seed.scope.clone(),
            handler_binding_ref: seed.binding_ref.clone(),
            operations: vec!["read".to_string(), "write".to_string()],
            capability_context_ref: seed.capability_ref.clone(),
            context_ref: None,
            resource_refs: seed.resource_refs.clone(),
            not_before: Some(0),
            expires_at: Some(10),
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref("parent-evidence")],
        })
        .expect("parent handle")
    }

    fn child(seed: &Seed, parent: &IoValue) -> IoValue {
        attenuated_handle_value(parent, &HandleAttenuationInput {
            scope: seed.scope.clone(),
            operations: vec!["read".to_string()],
            expires_at: Some(5),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            evidence_refs: vec![fake_ref("attenuation-evidence")],
        })
        .expect("attenuated child")
    }

    fn bundle() -> Bundle {
        let seed = seed();
        let parent = parent(&seed);
        let child = child(&seed, &parent);
        Bundle { seed, parent, child }
    }

    fn check_profile(bundle: &Bundle) {
        let profile = compound_handler_profile_value(&CompoundHandlerProfileInput {
            profile: "storage-plus-trace".to_string(),
            scope: bundle.seed.scope.clone(),
            handler_binding_refs: vec![bundle.seed.binding_ref.clone()],
            child_handle_refs: vec![
                canonical_hash(&bundle.parent).expect("parent ref"),
                canonical_hash(&bundle.child).expect("child ref"),
            ],
            policy_ref: bundle.seed.policy_ref.clone(),
            capability_context_ref: bundle.seed.capability_ref.clone(),
            context_ref: None,
            resource_refs: bundle.seed.resource_refs.clone(),
            evidence_refs: vec![fake_ref("profile-evidence")],
        })
        .expect("compound profile");
        let parsed_profile = parse_compound_handler_profile(&profile).expect("parse profile");
        assert_eq!(parsed_profile.child_handle_refs.len(), 2);
    }

    fn check_dynamic(seed: &Seed) {
        let dynamic = dynamic_operation_record_value(&DynamicOperationRecordInput {
            operation: "read".to_string(),
            adapter_ref: fake_ref("storage-adapter"),
            callable_ref: fake_ref("callable"),
            request_ref: fake_ref("request"),
            response_ref: fake_ref("response"),
            policy_ref: seed.policy_ref.clone(),
            capability_context_ref: seed.capability_ref.clone(),
            resource_refs: seed.resource_refs.clone(),
            evidence_refs: vec![fake_ref("dynamic-evidence")],
        })
        .expect("dynamic operation");
        assert_eq!(parse_dynamic_operation_record(&dynamic).expect("parse dynamic").operation, "read");
    }

    fn check_cleanup(child: &IoValue) {
        let cleanup = handle_cleanup_receipt_value(
            &canonical_hash(child).expect("child ref"),
            "revoke-live-cache",
            false,
            true,
            &[fake_ref("cleanup-evidence")],
        )
        .expect("cleanup receipt");
        let parsed_cleanup = parse_handle_cleanup_receipt(&cleanup).expect("parse cleanup");
        assert!(!parsed_cleanup.live_usable);
        assert!(parsed_cleanup.preserve_artifact);
    }

    struct Case {
        profile: &'static str,
        adapter_kind: &'static str,
        operations: &'static [&'static str],
    }

    struct Material {
        scope: EffectScope,
        policy_ref: String,
        capability_ref: String,
        resource_refs: Vec<String>,
        operations: Vec<String>,
        binding: IoValue,
        binding_ref: String,
    }

    fn material_for(case: &Case) -> Material {
        let scope = scope(Some(fake_ref(&format!("actor-{}-{}", case.profile, case.adapter_kind))));
        let policy_ref = fake_ref(&format!("policy-{}-{}", case.profile, case.adapter_kind));
        let capability_ref = fake_ref(&format!("capability-{}-{}", case.profile, case.adapter_kind));
        let resource_refs = vec![fake_ref(&format!("resource-{}-{}", case.profile, case.adapter_kind))];
        let operations = case.operations.iter().map(|operation| (*operation).to_string()).collect::<Vec<_>>();
        let binding = handler_binding_value(&HandlerBindingInput {
            profile: case.profile.to_string(),
            scope: scope.clone(),
            adapter_kind: case.adapter_kind.to_string(),
            adapter_ref: fake_ref(&format!("adapter-{}-{}", case.profile, case.adapter_kind)),
            executor_preflight_ref: None,
            policy_ref: policy_ref.clone(),
            capability_context_ref: capability_ref.clone(),
            context_ref: None,
            resource_refs: resource_refs.clone(),
            operations: operations.clone(),
            evidence_refs: vec![fake_ref(&format!(
                "binding-evidence-{}-{}",
                case.profile, case.adapter_kind
            ))],
        })
        .expect("handler binding");
        let binding_ref = canonical_hash(&binding).expect("binding ref");
        Material {
            scope,
            policy_ref,
            capability_ref,
            resource_refs,
            operations,
            binding,
            binding_ref,
        }
    }

    fn assert_profile(case: &Case, material: &Material) {
        let handler_profile = handler_profile_value(&HandlerProfileInput {
            profile: case.profile.to_string(),
            handler_binding_refs: vec![material.binding_ref.clone()],
            policy_ref: material.policy_ref.clone(),
            capability_context_ref: material.capability_ref.clone(),
            resource_refs: material.resource_refs.clone(),
            evidence_refs: vec![fake_ref(&format!(
                "profile-evidence-{}-{}",
                case.profile, case.adapter_kind
            ))],
        })
        .expect("handler profile");
        assert_eq!(parse_handler_profile(&handler_profile).expect("parse handler profile").profile, case.profile);
    }

    fn handle_for(case: &Case, material: &Material) -> IoValue {
        effect_handle_value(&EffectHandleInput {
            kind: case.adapter_kind.to_string(),
            scope: material.scope.clone(),
            handler_binding_ref: material.binding_ref.clone(),
            operations: material.operations.clone(),
            capability_context_ref: material.capability_ref.clone(),
            context_ref: None,
            resource_refs: material.resource_refs.clone(),
            not_before: None,
            expires_at: None,
            revocation_refs: Vec::new(),
            transfer: TRANSFER_LOCAL_ONLY.to_string(),
            parent_handle_ref: None,
            evidence_refs: vec![fake_ref(&format!(
                "handle-evidence-{}-{}",
                case.profile, case.adapter_kind
            ))],
        })
        .expect("effect handle")
    }

    fn assert_operations(case: &Case, material: &Material, handle: &IoValue) {
        for operation in &material.operations {
            validate_handle_for_request(&material.binding, handle, &EffectHandleRequest {
                kind: case.adapter_kind,
                operation,
                run_ref: &material.scope.run_ref,
                session_ref: &material.scope.session_ref,
                actor_ref: material.scope.actor_ref.as_deref(),
                turn_ref: material.scope.turn_ref.as_deref(),
                policy_ref: &material.policy_ref,
                capability_context_ref: &material.capability_ref,
                context_ref: None,
                resource_refs: &material.resource_refs,
                logical_time: 0,
                remote_use: false,
                revoked_refs: &[],
            })
            .expect("handler operation validates");
        }
    }

    fn assert_case(case: &Case) {
        let material = material_for(case);
        assert_profile(case, &material);
        let handle = handle_for(case, &material);
        assert_operations(case, &material, &handle);
    }

    #[test]
    fn dataspace_blob_and_storage_handlers_bind_local_and_production_operations() {
        let cases = [
            Case {
                profile: HANDLER_PROFILE_LOCAL,
                adapter_kind: ADAPTER_KIND_DATASPACE,
                operations: &["send", "observe"],
            },
            Case {
                profile: HANDLER_PROFILE_PRODUCTION,
                adapter_kind: ADAPTER_KIND_DATASPACE,
                operations: &["send", "observe"],
            },
            Case {
                profile: HANDLER_PROFILE_LOCAL,
                adapter_kind: ADAPTER_KIND_BLOB,
                operations: &["get", "put"],
            },
            Case {
                profile: HANDLER_PROFILE_PRODUCTION,
                adapter_kind: ADAPTER_KIND_BLOB,
                operations: &["get", "put"],
            },
            Case {
                profile: HANDLER_PROFILE_LOCAL,
                adapter_kind: ADAPTER_KIND_STORAGE,
                operations: &["read", "write"],
            },
            Case {
                profile: HANDLER_PROFILE_PRODUCTION,
                adapter_kind: ADAPTER_KIND_STORAGE,
                operations: &["read", "write"],
            },
        ];
        for case in cases {
            assert_case(&case);
        }
    }

    struct Env {
        scope: EffectScope,
        policy_ref: String,
        capability_ref: String,
        resource_refs: Vec<String>,
    }
