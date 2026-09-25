
    #[test]
    fn alpn_registry_validates_uniqueness_owner_profile_and_non_authority() {
        let registry = default_iroh_alpn_registry().expect("default registry");
        assert_eq!(registry.decision, "pass");
        assert_eq!(registry.entries.len(), 1);
        let text = crate::preserves_rail::to_text(&registry.value).expect("registry text");
        assert!(text.contains("routing-evidence-only"));

        let duplicate_inputs = default_iroh_alpn_registry_inputs();
        let duplicate = validate_iroh_alpn_registry(&[duplicate_inputs[0].clone(), duplicate_inputs[0].clone()])
            .expect("duplicate validation");
        assert_eq!(duplicate.decision, "deny");
        assert!(duplicate.diagnostics.iter().any(|diagnostic| diagnostic.contains("duplicate ALPN")));

        let mut wrong_owner = router_input("install", ROUTER_GENERATION_ONE);
        wrong_owner.owner_namespace = "other-owner".to_string();
        let denied_owner = evaluate_router_operation(&empty_protocol_registry(), &wrong_owner).expect("wrong owner");
        assert_eq!(denied_owner.decision, "deny");
        assert!(denied_owner.diagnostics.iter().any(|diagnostic| diagnostic.contains("owner namespace")));

        let mut wrong_profile = router_input("install", ROUTER_GENERATION_ONE);
        wrong_profile.handler_profile = "other-profile".to_string();
        let denied_profile = evaluate_router_operation(&empty_protocol_registry(), &wrong_profile).expect("wrong profile");
        assert_eq!(denied_profile.decision, "deny");
        assert!(denied_profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("handler-profile")));
    }
