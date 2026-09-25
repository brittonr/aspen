
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
