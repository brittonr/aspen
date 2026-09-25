
    #[test]
    fn grant_currentness_checks_scope_epoch_keys_and_revocation() {
        const EXPIRES_AT: u64 = 8;
        const GRANT_EPOCH: u64 = 4;
        const MINIMUM_EPOCH: u64 = 3;
        const CURRENT_EPOCH: u64 = 5;
        const CURRENT_TIME: u64 = 5;
        const STALE_GRANT_EPOCH: u64 = 2;

        let subject = ref_for("principal-currentness");
        let current_key = ref_for("current-key");
        let other_key = ref_for("other-key");
        let delegation = ref_for("delegation");
        let context =
            currentness_context(&subject, std::slice::from_ref(&current_key), std::slice::from_ref(&delegation));
        let input = AuthorityGrantCurrentnessInput {
            context: &context,
            requested_principal_ref: &subject,
            requested_capability: "node-control",
            requested_operation: "status",
            requested_scope: "node:control",
            logical_time: CURRENT_TIME,
            grant_epoch: GRANT_EPOCH,
            minimum_epoch: MINIMUM_EPOCH,
            current_epoch: CURRENT_EPOCH,
            current_key_refs: std::slice::from_ref(&current_key),
            revocations: &[],
        };
        let current = authority_grant_currentness(input).expect("currentness");
        assert_eq!(current.decision, "pass");
        assert!(current.diagnostics.is_empty());

        let wrong_scope = AuthorityGrantCurrentnessInput {
            requested_scope: "node:other",
            ..input
        };
        assert_diagnostic(wrong_scope, "capability-denied");

        let expired = AuthorityGrantCurrentnessInput {
            logical_time: EXPIRES_AT,
            ..input
        };
        assert_diagnostic(expired, "expired");

        let stale_epoch = AuthorityGrantCurrentnessInput {
            grant_epoch: STALE_GRANT_EPOCH,
            ..input
        };
        assert_diagnostic(stale_epoch, "stale-epoch");

        let wrong_key = AuthorityGrantCurrentnessInput {
            current_key_refs: std::slice::from_ref(&other_key),
            ..input
        };
        assert_diagnostic(wrong_key, "key-not-current");

        let revocation_value = revocation_value(RevocationValueInput {
            target_kind: "delegation",
            target_ref: &delegation,
            reason: "operator revoke",
            effective_at: CURRENT_TIME,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        let revocation = parse_revocation(&revocation_value).expect("parse revocation");
        let revocations = [revocation];
        let revoked = AuthorityGrantCurrentnessInput {
            revocations: &revocations,
            ..input
        };
        assert_diagnostic(revoked, "revoked");
    }

    /// A scoped node-control status context for `subject`, delegated once and valid over [2, 8).
    fn currentness_context(subject: &str, key_refs: &[String], delegation_refs: &[String]) -> Context {
        const CONTEXT_VALID_START: u64 = 2;
        const CONTEXT_EXPIRES_AT: u64 = 8;
        let context_value = context_value(ContextValueInput {
            subject_ref: subject,
            capabilities: &[Capability {
                capability: "node-control:status".to_string(),
                scope: "node:control".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs,
            not_before: Some(CONTEXT_VALID_START),
            expires_at: Some(CONTEXT_EXPIRES_AT),
            revocation_refs: &[],
            key_refs,
            policy_refs: &[ref_for("policy")],
            evidence_refs: &[ref_for("evidence")],
        })
        .expect("context");
        parse_context(&context_value).expect("parse context")
    }

    fn assert_diagnostic(input: AuthorityGrantCurrentnessInput<'_>, expected: &str) {
        let currentness = authority_grant_currentness(input).expect("currentness");
        assert_eq!(currentness.decision, "fail");
        assert!(
            currentness.diagnostics.iter().any(|diagnostic| diagnostic == expected),
            "missing {expected} in {:?}",
            currentness.diagnostics
        );
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("authority-test-ref", vec![string(label)])).expect("test ref")
    }
