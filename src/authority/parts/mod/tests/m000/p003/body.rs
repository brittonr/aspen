    #[hegel::test(test_cases = 16)]
    fn hegel_attenuation_monotonicity_identity_no_authority_and_cleanup(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let subject = ref_for(&format!("principal-{salt}"));
        let identity_id = format!("p-{salt}");
        let identity_value = identity_value(IdentityValueInput {
            identity_type: "principal",
            id: &identity_id,
            display_name: "principal",
            key_refs: &[],
            parent_refs: &[],
            metadata_refs: &[],
        })
        .expect("identity");
        parse_identity(&identity_value).expect("identity parses");
        let no_context_receipt = receipt_value(ReceiptValueInput {
            operation: "identity-only",
            decision: "fail",
            context_ref: None,
            capability: "read",
            scope: "scope",
            logical_time: salt,
            diagnostics: &[],
        });
        assert!(crate::preserves_rail::to_text(&no_context_receipt).expect("receipt text").contains("fail"));

        let scope = format!("scope:{salt}");
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[Capability {
                capability: "read".to_string(),
                scope: scope.clone(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[ref_for("key")],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        assert_eq!(
            admit_with_key(&context_value, &subject, "read", &scope, salt, &[]).expect("same scope").decision,
            "pass"
        );
        assert_eq!(
            admit_with_key(&context_value, &subject, "read", "other-scope", salt, &[])
                .expect("other scope")
                .decision,
            "fail"
        );
        let context = parse_context(&context_value).expect("context");
        let revocation = revocation_value(RevocationValueInput {
            target_kind: "authority-context",
            target_ref: &context.context_ref,
            reason: "cleanup",
            effective_at: salt,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        let assertion = RuntimeAssertion {
            actor: "authority".to_string(),
            value: RuntimeValue::new(record("authority-bound-assertion", vec![
                record("authority", vec![string(&context.context_ref)]),
                record("value", vec![string("x")]),
            ]))
            .expect("assertion value"),
            facet: None,
        };
        let (remaining, _) = cleanup_for_revocation(&[assertion], &revocation, salt).expect("cleanup");
        assert!(remaining.is_empty());
    }
