    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;
    type PathBuf = std::path::PathBuf;

    type TestCase = hegel::TestCase;

    use super::*;

    #[test]
    fn cache_hit_miss_output_integrity_and_no_name_key() {
        let root = temp_dir("eval-cache-hit");
        let key = key_input("schema-fingerprint", "input", &[]);
        let output = record("fingerprint", vec![string("ok")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[])).expect("put");
        let hit = get(&root, &put.key.key_ref, &GetInput::default()).expect("hit");
        assert_eq!(hit.output, Some(output));
        assert_eq!(hit.key.operation, "schema-fingerprint");
        let miss_key = key_value(&key_input("schema-fingerprint", "changed-input", &[])).expect("miss key");
        let miss = parse_key(&miss_key).expect("parse miss key");
        let error = get(&root, &miss.key_ref, &GetInput::default()).expect_err("miss denied");
        assert!(error.to_string().contains("miss"), "{error}");
        let renamed_key = KeyInput {
            assumption_refs: vec![test_ref("display-name-not-key")],
            ..key.clone()
        };
        let renamed_ref = canonical_hash(&key_value(&renamed_key).expect("renamed key")).expect("renamed ref");
        assert_ne!(put.key.key_ref, renamed_ref);
    }

    #[test]
    fn policy_current_revalidates_and_negative_results_require_key_evidence() {
        let root = temp_dir("eval-cache-policy-current");
        let denial_ref = test_ref("denial-input");
        let key = KeyInput {
            policy_refs: vec![test_ref("policy-v1")],
            assumption_refs: vec![denial_ref.clone()],
            ..key_input("schema-compat", "input", &[])
        };
        let output = record("denied", vec![string("policy")]);
        let policy_put = put(&root, &key, &ValueInput {
            tier: TIER_POLICY_CURRENT.to_string(),
            status: STATUS_DENY.to_string(),
            output: Some(output),
            dependency_refs: key.dependency_refs.clone(),
            policy_refs: key.policy_refs.clone(),
            evidence_refs: vec![denial_ref],
            diagnostics: vec!["policy denied".to_string()],
        })
        .expect("put policy current denial");
        let current = GetInput {
            current_policy_refs: key.policy_refs.clone(),
            semantic: true,
            ..GetInput::default()
        };
        get(&root, &policy_put.key.key_ref, &current).expect("policy current hit");
        let stale = GetInput {
            current_policy_refs: vec![test_ref("policy-v2")],
            semantic: true,
            ..GetInput::default()
        };
        let error = get(&root, &policy_put.key.key_ref, &stale).expect_err("stale denied");
        assert!(error.to_string().contains("stale"), "{error}");
        let bad = put(&root, &key_input("schema-compat", "bad-negative", &[]), &ValueInput {
            tier: TIER_PURE.to_string(),
            status: STATUS_DENY.to_string(),
            output: Some(record("denied", vec![string("bad")])),
            dependency_refs: Vec::new(),
            policy_refs: Vec::new(),
            evidence_refs: vec![test_ref("unbound-denial")],
            diagnostics: vec!["bad negative".to_string()],
        })
        .expect_err("unbound denial evidence rejected");
        assert!(bad.to_string().contains("negative cache result evidence refs"), "{bad}");
    }

    #[test]
    fn cache_hit_validity_denies_dependency_revocation_trace_and_output_drift() {
        let root = temp_dir("eval-cache-hit-validity");
        let dependency = test_ref("dependency-v1");
        let capability = test_ref("capability-v1");
        let key = KeyInput {
            capability_refs: vec![capability.clone()],
            ..key_input("artifact-closure", "input", std::slice::from_ref(&dependency))
        };
        let output = record("closure", vec![string("ok")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put cache value");
        let valid = evaluate_cache_hit_validity(CacheHitValidityInput {
            requested_dependency_refs: std::slice::from_ref(&dependency),
            expected_output_ref: Some(match &put.value.output {
                OutputRef::Inline { output_ref, .. } | OutputRef::ContentRef { output_ref, .. } => output_ref.as_str(),
                OutputRef::None => panic!("cache output missing"),
            }),
            ..cache_hit_validity_input(&put.key, &put.value)
        });
        assert_eq!(valid.decision, "pass");

        let changed_dependency_refs = vec![test_ref("dependency-v2")];
        let changed_dependency = evaluate_cache_hit_validity(CacheHitValidityInput {
            requested_dependency_refs: &changed_dependency_refs,
            ..cache_hit_validity_input(&put.key, &put.value)
        });
        assert_eq!(changed_dependency.decision, "deny");
        assert!(changed_dependency.diagnostics.iter().any(|value| value == "dependency-refs-changed"));

        let revoked = get(&root, &put.key.key_ref, &GetInput {
            current_revocation_refs: vec![capability],
            ..GetInput::default()
        })
        .expect_err("revoked capability denies hit");
        assert!(revoked.to_string().contains("validity"), "{revoked}");

        let wrong_output = evaluate_cache_hit_validity(CacheHitValidityInput {
            expected_output_ref: Some(&test_ref("other-output")),
            ..cache_hit_validity_input(&put.key, &put.value)
        });
        assert_eq!(wrong_output.decision, "deny");
        assert!(wrong_output.diagnostics.iter().any(|value| value == "output-ref-mismatch"));

        let trace_value = Value {
            tier: TIER_PRODUCTION_TRACE_ONLY.to_string(),
            ..put.value.clone()
        };
        let trace_only = evaluate_cache_hit_validity(CacheHitValidityInput {
            ..cache_hit_validity_input(&put.key, &trace_value)
        });
        assert_eq!(trace_only.decision, "deny");
        assert!(trace_only.diagnostics.iter().any(|value| value == "trace-only-not-semantic"));
    }

    #[test]
    fn policy_aware_cache_keys_and_hit_freshness_bind_admission_context() {
        // r[verify molten.eval_cache.policy_aware_validation]
        let root = temp_dir("eval-cache-policy-aware");
        let handler_profile = test_ref("handler-profile-v1");
        let evidence = test_ref("supporting-evidence-v1");
        let key = policy_aware_key();
        let same_key = key_value(&key).expect("policy-aware key");
        let same_key_again = key_value(&key).expect("same key");
        assert_eq!(canonical_hash(&same_key).expect("same key ref"), canonical_hash(&same_key_again).expect("same key ref again"));
        let changed_policy_key = KeyInput {
            policy_export_refs: vec![test_ref("policy-export-v2")],
            ..key.clone()
        };
        assert_ne!(
            canonical_hash(&same_key).expect("key ref"),
            canonical_hash(&key_value(&changed_policy_key).expect("changed policy key")).expect("changed key ref")
        );
        let put = put(
            &root,
            &key,
            &value_input(TIER_POLICY_CURRENT, STATUS_PASS, Some(record("valid", vec![string("ok")])), &key, std::slice::from_ref(&evidence)),
        )
        .expect("put policy-aware cache entry");
        let fresh_get = current_get_input(&key);
        get(&root, &put.key.key_ref, &fresh_get).expect("fresh policy-aware hit");
        let stale_get = GetInput {
            current_policy_export_refs: vec![test_ref("policy-export-v2")],
            ..fresh_get
        };
        let stale_policy = get(&root, &put.key.key_ref, &stale_get).expect_err("stale policy export denies hit");
        let policy_message = stale_policy.to_string();
        let is_stale_explained = policy_message.contains("validity") || policy_message.contains("stale");
        assert!(is_stale_explained, "unexpected stale policy denial: {policy_message}");

        let changed_handler = test_ref("handler-profile-v2");
        let changed_handler_input = CacheHitValidityInput {
            current_handler_profile_ref: Some(&changed_handler),
            ..current_validity_input(&key, &put.key, &put.value)
        };
        let denied_profile = evaluate_cache_hit_validity(changed_handler_input);
        assert_eq!(denied_profile.decision, "deny");
        assert!(denied_profile.diagnostics.iter().any(|diagnostic| diagnostic == "handler-profile-changed"));
        let compatibility_refs = vec![handler_profile, changed_handler.clone()];
        let compatible_profile = evaluate_cache_hit_validity(CacheHitValidityInput {
            compatibility_refs: &compatibility_refs,
            ..changed_handler_input
        });
        assert_eq!(compatible_profile.decision, "pass");
        let missing_evidence = crate::eval_cache::put(&root, &KeyInput { evidence_refs: Vec::new(), ..key }, &ValueInput {
            tier: TIER_PURE.to_string(),
            status: STATUS_DENY.to_string(),
            output: Some(record("bad", vec![string("missing-evidence")])),
            dependency_refs: Vec::new(),
            policy_refs: Vec::new(),
            evidence_refs: vec![evidence],
            diagnostics: Vec::new(),
        })
        .expect_err("missing evidence denied");
        assert!(missing_evidence.to_string().contains("negative cache result evidence refs"));
    }

    /// A normative-validation key bound to one ref for each policy, capability, resource, provenance, retention, and
    /// evidence context.
    fn policy_aware_key() -> KeyInput {
        let mut key = key_input("normative-validation", "input", &[]);
        key.policy_refs = vec![test_ref("policy-v1")];
        key.policy_export_refs = vec![test_ref("policy-export-v1")];
        key.capability_refs = vec![test_ref("capability-context-v1")];
        key.revocation_refs = vec![test_ref("revocation-epoch-v1")];
        key.resource_refs = vec![test_ref("resource-profile-v1")];
        key.handler_profile_ref = Some(test_ref("handler-profile-v1"));
        key.provenance_refs = vec![test_ref("provenance-v1")];
        key.source_gate_refs = vec![test_ref("source-gate-v1")];
        key.retention_refs = vec![test_ref("retention-v1")];
        key.evidence_refs = vec![test_ref("supporting-evidence-v1")];
        key
    }

    /// A get whose current admission context equals the context `key` was cached under.
    fn current_get_input(key: &KeyInput) -> GetInput {
        GetInput {
            current_policy_refs: key.policy_refs.clone(),
            current_policy_export_refs: key.policy_export_refs.clone(),
            current_capability_refs: key.capability_refs.clone(),
            current_revocation_refs: key.revocation_refs.clone(),
            current_resource_refs: key.resource_refs.clone(),
            current_handler_profile_ref: key.handler_profile_ref.clone(),
            current_provenance_refs: key.provenance_refs.clone(),
            current_source_gate_refs: key.source_gate_refs.clone(),
            current_retention_refs: key.retention_refs.clone(),
            current_evidence_refs: key.evidence_refs.clone(),
            ..GetInput::default()
        }
    }

    /// A hit-validity input whose current admission context equals the context `key` was cached under.
    fn current_validity_input<'a>(key: &'a KeyInput, cached: &'a Key, value: &'a Value) -> CacheHitValidityInput<'a> {
        CacheHitValidityInput {
            current_policy_refs: &key.policy_refs,
            current_policy_export_refs: &key.policy_export_refs,
            current_capability_refs: &key.capability_refs,
            current_revocation_refs: &key.revocation_refs,
            current_resource_refs: &key.resource_refs,
            current_handler_profile_ref: key.handler_profile_ref.as_deref(),
            current_provenance_refs: &key.provenance_refs,
            current_source_gate_refs: &key.source_gate_refs,
            current_retention_refs: &key.retention_refs,
            current_evidence_refs: &key.evidence_refs,
            ..cache_hit_validity_input(cached, value)
        }
    }

    #[test]
    fn trace_only_and_invalidation_fail_closed() {
        let root = temp_dir("eval-cache-trace");
        let dependency = test_ref("dependency");
        let trace_evidence = test_ref("trace-evidence");
        let key = KeyInput {
            evidence_refs: vec![trace_evidence.clone()],
            ..key_input("transcript-run", "trace", std::slice::from_ref(&dependency))
        };
        let trace = put(&root, &key, &ValueInput {
            tier: TIER_PRODUCTION_TRACE_ONLY.to_string(),
            status: STATUS_TRACE_ONLY.to_string(),
            output: None,
            dependency_refs: key.dependency_refs.clone(),
            policy_refs: Vec::new(),
            evidence_refs: vec![trace_evidence],
            diagnostics: vec!["production trace only".to_string()],
        })
        .expect("put trace-only");
        let error = get(&root, &trace.key.key_ref, &GetInput::default()).expect_err("trace-only semantic denied");
        assert!(error.to_string().contains("trace-only"), "{error}");
        let retention_evidence = retention_evidence(&root, "trace-invalidate");
        let apply_refs = vec![apply_ref(&root, &trace.key.key_ref, &retention_evidence)];
        let invalidated = invalidate(&root, &InvalidateInput {
            dependency_ref: Some(dependency),
            reason: "dependency changed".to_string(),
            retention_evidence,
            apply_refs,
            ..InvalidateInput::default()
        })
        .expect("invalidate dependency");
        assert!(invalidated.invalidated_key_refs.contains(&trace.key.key_ref));
        let miss = get(&root, &trace.key.key_ref, &GetInput {
            semantic: false,
            ..GetInput::default()
        })
        .expect_err("tombstone miss");
        assert!(miss.to_string().contains("tombstoned"), "{miss}");
    }
