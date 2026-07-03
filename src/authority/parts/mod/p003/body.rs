
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_records_do_not_grant_without_context() {
        let identity_value = identity_value(IdentityValueInput {
            identity_type: "principal",
            id: "alice",
            display_name: "Alice",
            key_refs: &[],
            parent_refs: &[],
            metadata_refs: &[],
        })
        .expect("identity");
        let identity = parse_identity(&identity_value).expect("parse identity");
        let receipt = receipt_value(ReceiptValueInput {
            operation: "identity-check",
            decision: "fail",
            context_ref: None,
            capability: "read",
            scope: "store",
            logical_time: 0,
            diagnostics: &["identity-only"],
        });
        assert_eq!(identity.identity_type, "principal");
        assert!(
            crate::preserves_rail::to_text(&identity.value)
                .expect("identity text")
                .contains("identity-alone-grants-no-authority")
        );
        assert!(crate::preserves_rail::to_text(&receipt).expect("receipt text").contains("identity-only"));
    }

    #[test]
    fn context_admits_scoped_capability_and_denies_attenuation_mismatch() {
        let subject = ref_for("principal");
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[Capability {
                capability: "read".to_string(),
                scope: "catalog:public".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: Some(10),
            revocation_refs: &[],
            key_refs: &[ref_for("key")],
            policy_refs: &[ref_for("policy")],
            evidence_refs: &[ref_for("evidence")],
        })
        .expect("context");
        let pass = admit_authority(&context_value, "read", "catalog:public", 1, &[]).expect("admit");
        assert_eq!(pass.decision, "pass");
        let fail = admit_authority(&context_value, "write", "catalog:public", 1, &[]).expect("deny");
        assert_eq!(fail.decision, "fail");
    }

    #[test]
    fn revocation_retracts_dependent_assertions_and_denies_future_effects() {
        let subject = ref_for("principal");
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[Capability {
                capability: "effect:clock".to_string(),
                scope: "actor:a".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        let context = parse_context(&context_value).expect("parse context");
        let revocation_value = revocation_value(RevocationValueInput {
            target_kind: "authority-context",
            target_ref: &context.context_ref,
            reason: "operator revoke",
            effective_at: 5,
            issuer_ref: &subject,
            evidence_refs: &[],
        })
        .expect("revocation");
        let before = admit_authority(&context_value, "effect:clock", "actor:a", 4, &[]).expect("before revoke");
        assert_eq!(before.decision, "pass");
        let after =
            admit_authority(&context_value, "effect:clock", "actor:a", 5, std::slice::from_ref(&revocation_value))
                .expect("after revoke");
        assert_eq!(after.decision, "fail");
        let assertion = RuntimeAssertion {
            actor: "authority".to_string(),
            value: RuntimeValue::new(record("authority-bound-assertion", vec![
                record("authority", vec![string(&context.context_ref)]),
                record("value", vec![string("visible")]),
            ]))
            .expect("assertion value"),
        };
        let (remaining, cleanup) = cleanup_for_revocation(&[assertion], &revocation_value, 5).expect("cleanup");
        assert!(remaining.is_empty());
        assert_eq!(cleanup.decision, "pass");
    }

    #[test]
    fn expiry_and_gatekeeper_live_refs_are_enforced() {
        let subject = ref_for("principal");
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[Capability {
                capability: "resolve".to_string(),
                scope: "service:db".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: Some(2),
            expires_at: Some(5),
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        let early = admit_authority(&context_value, "resolve", "service:db", 1, &[]).expect("early");
        assert_eq!(early.decision, "fail");
        let live = gatekeeper_resolve_live_ref(&context_value, "service:db", "resolve", 2, &[]).expect("live ref");
        assert_eq!(live.scope, "service:db");
        let expired = admit_authority(&context_value, "resolve", "service:db", 5, &[]).expect("expired");
        assert_eq!(expired.decision, "fail");
    }

    #[test]
    fn key_rotation_preserves_historical_replay_but_not_current_authority() {
        let subject = ref_for("principal");
        let old_key = ref_for("old-key");
        let new_key = ref_for("new-key");
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[Capability {
                capability: "sign".to_string(),
                scope: "receipt".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: std::slice::from_ref(&old_key),
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("old context");
        let context = parse_context(&context_value).expect("context");
        let historical = admit_authority(&context_value, "sign", "receipt", 1, &[]).expect("historical admission");
        replay_verify_receipt(&historical.receipt, &context_value).expect("historical replay");
        let new_key_refs = [new_key];
        let revoke_old_key = revocation_value(RevocationValueInput {
            target_kind: "key",
            target_ref: &old_key,
            reason: "rotate",
            effective_at: 2,
            issuer_ref: &subject,
            evidence_refs: &new_key_refs,
        })
        .expect("rotate");
        let current = admit_authority(&context_value, "sign", "receipt", 2, &[revoke_old_key]).expect("current denied");
        assert_eq!(current.decision, "fail");
        assert_eq!(historical.receipt.context_ref.as_deref(), Some(context.context_ref.as_str()));
    }

    #[test]
    fn storage_remote_catalog_contexts_share_admission_path() {
        let subject = ref_for("principal");
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[
                Capability {
                    capability: "storage:read".to_string(),
                    scope: "store:typed".to_string(),
                    attenuation: "scoped".to_string(),
                },
                Capability {
                    capability: "remote-sync:pull".to_string(),
                    scope: "catalog:public".to_string(),
                    attenuation: "scoped".to_string(),
                },
                Capability {
                    capability: "catalog:visible".to_string(),
                    scope: "catalog:public".to_string(),
                    attenuation: "scoped".to_string(),
                },
            ],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        assert_eq!(
            admit_authority(&context_value, "storage:read", "store:typed", 0, &[]).expect("storage").decision,
            "pass"
        );
        assert_eq!(
            admit_authority(&context_value, "remote-sync:pull", "catalog:public", 0, &[])
                .expect("remote")
                .decision,
            "pass"
        );
        assert_eq!(
            admit_authority(&context_value, "catalog:visible", "catalog:public", 0, &[])
                .expect("catalog")
                .decision,
            "pass"
        );
    }

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
            key_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("context");
        assert_eq!(admit_authority(&context_value, "read", &scope, salt, &[]).expect("same scope").decision, "pass");
        assert_eq!(
            admit_authority(&context_value, "read", "other-scope", salt, &[]).expect("other scope").decision,
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
        };
        let (remaining, _) = cleanup_for_revocation(&[assertion], &revocation, salt).expect("cleanup");
        assert!(remaining.is_empty());
    }

    #[test]
    fn grant_currentness_checks_scope_epoch_keys_and_revocation() {
        const VALID_START: u64 = 2;
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
        let context_value = context_value(ContextValueInput {
            subject_ref: &subject,
            capabilities: &[Capability {
                capability: "node-control:status".to_string(),
                scope: "node:control".to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: std::slice::from_ref(&delegation),
            not_before: Some(VALID_START),
            expires_at: Some(EXPIRES_AT),
            revocation_refs: &[],
            key_refs: std::slice::from_ref(&current_key),
            policy_refs: &[ref_for("policy")],
            evidence_refs: &[ref_for("evidence")],
        })
        .expect("context");
        let context = parse_context(&context_value).expect("parse context");
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
}
