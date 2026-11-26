
    /// Admission with caller-held principal and current keys, independent of the context's fields.
    fn admit_as(
        context_value: &IoValue,
        expected_principal_ref: &str,
        current_key_refs: &[String],
        requested_capability: &str,
        requested_scope: &str,
        logical_time: u64,
        revocation_values: &[IoValue],
    ) -> Result<Admission> {
        admit_authority(AuthorityAdmissionInput {
            context_value,
            expected_principal_ref,
            requested_capability,
            requested_scope,
            logical_time,
            minimum_epoch: AUTHORITY_DEFAULT_GRANT_EPOCH,
            current_key_refs,
            revocation_values,
        })
    }

    /// Admission whose caller observes the shared test key `ref_for("key")` as current.
    fn admit_with_key(
        context_value: &IoValue,
        expected_principal_ref: &str,
        requested_capability: &str,
        requested_scope: &str,
        logical_time: u64,
        revocation_values: &[IoValue],
    ) -> Result<Admission> {
        let current_key_refs = [ref_for("key")];
        admit_as(
            context_value,
            expected_principal_ref,
            &current_key_refs,
            requested_capability,
            requested_scope,
            logical_time,
            revocation_values,
        )
    }

    fn scoped_context(subject: &str, key_refs: &[String], not_before: Option<u64>, expires_at: Option<u64>) -> IoValue {
        context_value(ContextValueInput {
            subject_ref: subject,
            capabilities: &[Capability {
                capability: "read".to_string(),
                scope: "catalog:public".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before,
            expires_at,
            revocation_refs: &[],
            key_refs,
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context")
    }

    fn receipt_diagnostics(admission: &Admission) -> String {
        crate::preserves_rail::to_text(&admission.receipt.value).expect("receipt text")
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn admit_authority_compares_against_caller_principal_not_context_subject() {
        let subject = ref_for("context-subject");
        let caller = ref_for("authenticated-caller");
        let keys = [ref_for("key")];
        let context_value = scoped_context(&subject, &keys, None, None);
        let matching = admit_as(&context_value, &subject, &keys, "read", "catalog:public", 1, &[]).expect("matching");
        assert_eq!(matching.decision, "pass");
        let mismatched = admit_as(&context_value, &caller, &keys, "read", "catalog:public", 1, &[]).expect("mismatch");
        assert_eq!(mismatched.decision, "fail");
        assert!(receipt_diagnostics(&mismatched).contains("principal-mismatch"));
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn admit_authority_denies_contexts_without_signing_keys() {
        let subject = ref_for("principal-no-keys");
        let context_value = scoped_context(&subject, &[], None, None);
        let denied = admit_as(&context_value, &subject, &[], "read", "catalog:public", 1, &[]).expect("no keys");
        assert_eq!(denied.decision, "fail");
        assert!(receipt_diagnostics(&denied).contains("no-signing-keys"));
        let with_caller_keys = admit_with_key(&context_value, &subject, "read", "catalog:public", 1, &[])
            .expect("caller keys do not fill an empty context");
        assert_eq!(with_caller_keys.decision, "fail");
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn admit_authority_denies_any_stale_context_key() {
        let subject = ref_for("principal-two-keys");
        let current = ref_for("current-signing-key");
        let stale = ref_for("rotated-out-signing-key");
        let context_keys = [current.clone(), stale.clone()];
        let context_value = scoped_context(&subject, &context_keys, None, None);
        let all_current = admit_as(&context_value, &subject, &context_keys, "read", "catalog:public", 1, &[])
            .expect("all current");
        assert_eq!(all_current.decision, "pass");
        let partially_stale = admit_as(&context_value, &subject, std::slice::from_ref(&current), "read", "catalog:public", 1, &[])
            .expect("partially stale");
        assert_eq!(partially_stale.decision, "fail");
        assert!(receipt_diagnostics(&partially_stale).contains("key-not-current"));
        let none_current = admit_as(&context_value, &subject, &[], "read", "catalog:public", 1, &[]).expect("none");
        assert_eq!(none_current.decision, "fail");
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn admit_authority_applies_caller_minimum_epoch() {
        let subject = ref_for("principal-epoch");
        let keys = [ref_for("key")];
        let context_value = scoped_context(&subject, &keys, Some(2), None);
        let input = AuthorityAdmissionInput {
            context_value: &context_value,
            expected_principal_ref: &subject,
            requested_capability: "read",
            requested_scope: "catalog:public",
            logical_time: 4,
            minimum_epoch: 2,
            current_key_refs: &keys,
            revocation_values: &[],
        };
        assert_eq!(admit_authority(input).expect("current epoch").decision, "pass");
        let stale = admit_authority(AuthorityAdmissionInput {
            minimum_epoch: 3,
            ..input
        })
        .expect("stale epoch");
        assert_eq!(stale.decision, "fail");
        assert!(receipt_diagnostics(&stale).contains("stale-epoch"));
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn gatekeeper_refuses_live_ref_for_other_principal_or_stale_key() {
        let subject = ref_for("principal-gatekeeper");
        let keys = [ref_for("key")];
        let context_value = scoped_context(&subject, &keys, None, None);
        let input = AuthorityAdmissionInput {
            context_value: &context_value,
            expected_principal_ref: &subject,
            requested_capability: "read",
            requested_scope: "catalog:public",
            logical_time: 1,
            minimum_epoch: 0,
            current_key_refs: &keys,
            revocation_values: &[],
        };
        assert_eq!(gatekeeper_resolve_live_ref(input).expect("live ref").scope, "catalog:public");
        let other = ref_for("other-principal");
        assert!(
            gatekeeper_resolve_live_ref(AuthorityAdmissionInput {
                expected_principal_ref: &other,
                ..input
            })
            .is_err()
        );
        let rotated = [ref_for("rotated-key")];
        assert!(
            gatekeeper_resolve_live_ref(AuthorityAdmissionInput {
                current_key_refs: &rotated,
                ..input
            })
            .is_err()
        );
    }

    fn bound_assertion(authority_ref: &str) -> RuntimeAssertion {
        RuntimeAssertion {
            actor: "authority".to_string(),
            value: RuntimeValue::new(record("authority-bound-assertion", vec![
                record("authority", vec![string(authority_ref)]),
                record("value", vec![string("visible")]),
            ]))
            .expect("assertion value"),
            facet: None,
        }
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn cleanup_waits_for_revocation_effective_time() {
        let subject = ref_for("principal-cleanup-time");
        let context = parse_context(&scoped_context(&subject, &[ref_for("key")], None, None)).expect("context");
        let revocation = revocation_value(RevocationValueInput {
            target_kind: "authority-context",
            target_ref: &context.context_ref,
            reason: "scheduled revoke",
            effective_at: 10,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        let assertions = [bound_assertion(&context.context_ref)];
        let (early, early_receipt) = cleanup_for_revocation(&assertions, &revocation, 9).expect("early cleanup");
        assert_eq!(early.len(), 1, "future-dated revocation must not remove grants early");
        assert_eq!(early_receipt.decision, "fail");
        assert!(crate::preserves_rail::to_text(&early_receipt.value).expect("text").contains("revocation-not-yet-effective"));
        let (effective, receipt) = cleanup_for_revocation(&assertions, &revocation, 10).expect("effective cleanup");
        assert!(effective.is_empty());
        assert_eq!(receipt.decision, "pass");
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn cleanup_honors_revocation_target_kind() {
        let subject = ref_for("principal-cleanup-kind");
        let context = parse_context(&scoped_context(&subject, &[ref_for("key")], None, None)).expect("context");
        let assertions = [bound_assertion(&context.context_ref)];
        let revocation_of_kind = |target_kind: &str| {
            revocation_value(RevocationValueInput {
                target_kind,
                target_ref: &context.context_ref,
                reason: "revoke",
                effective_at: 0,
                issuer_ref: &subject,
                evidence_refs: &[],
            })
            .expect("revocation")
        };
        let (kept, receipt) = cleanup_for_revocation(&assertions, &revocation_of_kind("key"), 1).expect("key kind");
        assert_eq!(kept.len(), 1, "a key revocation must not remove a context-bound assertion by ref coincidence");
        assert!(crate::preserves_rail::to_text(&receipt.value).expect("text").contains("target-kind-not-assertion-bound"));
        let (removed, _) =
            cleanup_for_revocation(&assertions, &revocation_of_kind("authority-context"), 1).expect("context kind");
        assert!(removed.is_empty());
    }

    // r[verify molten.authority.independent_currentness_and_revocation_bounds]
    #[test]
    fn capability_scope_admission_checks_scope_validity_revocation_and_records_non_claim() {
        let subject = ref_for("principal-job");
        let context_value = scoped_context(&subject, &[], None, Some(5));
        let pass = admit_capability_scope(&context_value, "read", "catalog:public", 1, &[]).expect("scope pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(pass.receipt.operation, "capability-scope-admission");
        assert!(receipt_diagnostics(&pass).contains(CAPABILITY_SCOPE_NON_CLAIM));
        assert_eq!(
            admit_capability_scope(&context_value, "read", "catalog:private", 1, &[]).expect("other scope").decision,
            "fail"
        );
        assert_eq!(
            admit_capability_scope(&context_value, "write", "catalog:public", 1, &[]).expect("other capability").decision,
            "fail"
        );
        assert_eq!(
            admit_capability_scope(&context_value, "read", "catalog:public", 5, &[]).expect("expired").decision,
            "fail"
        );
        let context = parse_context(&context_value).expect("context");
        let revocation = revocation_value(RevocationValueInput {
            target_kind: "authority-context",
            target_ref: &context.context_ref,
            reason: "revoke",
            effective_at: 1,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        assert_eq!(
            admit_capability_scope(&context_value, "read", "catalog:public", 1, &[revocation]).expect("revoked").decision,
            "fail"
        );
    }
