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
        let policy = test_ref("policy-v1");
        let policy_export = test_ref("policy-export-v1");
        let capability = test_ref("capability-context-v1");
        let revocation_epoch = test_ref("revocation-epoch-v1");
        let resource = test_ref("resource-profile-v1");
        let handler_profile = test_ref("handler-profile-v1");
        let provenance = test_ref("provenance-v1");
        let source_gate = test_ref("source-gate-v1");
        let retention = test_ref("retention-v1");
        let evidence = test_ref("supporting-evidence-v1");
        let mut key = key_input("normative-validation", "input", &[]);
        key.policy_refs = vec![policy.clone()];
        key.policy_export_refs = vec![policy_export.clone()];
        key.capability_refs = vec![capability.clone()];
        key.revocation_refs = vec![revocation_epoch.clone()];
        key.resource_refs = vec![resource.clone()];
        key.handler_profile_ref = Some(handler_profile.clone());
        key.provenance_refs = vec![provenance.clone()];
        key.source_gate_refs = vec![source_gate.clone()];
        key.retention_refs = vec![retention.clone()];
        key.evidence_refs = vec![evidence.clone()];
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
        get(&root, &put.key.key_ref, &GetInput {
            current_policy_refs: vec![policy.clone()],
            current_policy_export_refs: vec![policy_export.clone()],
            current_capability_refs: vec![capability.clone()],
            current_revocation_refs: vec![revocation_epoch.clone()],
            current_resource_refs: vec![resource.clone()],
            current_handler_profile_ref: Some(handler_profile.clone()),
            current_provenance_refs: vec![provenance.clone()],
            current_source_gate_refs: vec![source_gate.clone()],
            current_retention_refs: vec![retention.clone()],
            current_evidence_refs: vec![evidence.clone()],
            ..GetInput::default()
        })
        .expect("fresh policy-aware hit");
        let stale_policy = get(&root, &put.key.key_ref, &GetInput {
            current_policy_refs: vec![policy.clone()],
            current_policy_export_refs: vec![test_ref("policy-export-v2")],
            current_capability_refs: vec![capability.clone()],
            current_revocation_refs: vec![revocation_epoch.clone()],
            current_resource_refs: vec![resource.clone()],
            current_handler_profile_ref: Some(handler_profile.clone()),
            current_provenance_refs: vec![provenance.clone()],
            current_source_gate_refs: vec![source_gate.clone()],
            current_retention_refs: vec![retention.clone()],
            current_evidence_refs: vec![evidence.clone()],
            ..GetInput::default()
        })
        .expect_err("stale policy export denies hit");
        assert!(stale_policy.to_string().contains("validity") || stale_policy.to_string().contains("stale"));

        let changed_handler = test_ref("handler-profile-v2");
        let denied_profile = evaluate_cache_hit_validity(CacheHitValidityInput {
            current_policy_refs: std::slice::from_ref(&policy),
            current_policy_export_refs: std::slice::from_ref(&policy_export),
            current_capability_refs: std::slice::from_ref(&capability),
            current_revocation_refs: std::slice::from_ref(&revocation_epoch),
            current_resource_refs: std::slice::from_ref(&resource),
            current_handler_profile_ref: Some(&changed_handler),
            current_provenance_refs: std::slice::from_ref(&provenance),
            current_source_gate_refs: std::slice::from_ref(&source_gate),
            current_retention_refs: std::slice::from_ref(&retention),
            current_evidence_refs: std::slice::from_ref(&evidence),
            ..cache_hit_validity_input(&put.key, &put.value)
        });
        assert_eq!(denied_profile.decision, "deny");
        assert!(denied_profile.diagnostics.iter().any(|diagnostic| diagnostic == "handler-profile-changed"));
        let compatibility_refs = vec![handler_profile, changed_handler.clone()];
        let compatible_profile = evaluate_cache_hit_validity(CacheHitValidityInput {
            current_policy_refs: std::slice::from_ref(&policy),
            current_policy_export_refs: std::slice::from_ref(&policy_export),
            current_capability_refs: std::slice::from_ref(&capability),
            current_revocation_refs: std::slice::from_ref(&revocation_epoch),
            current_resource_refs: std::slice::from_ref(&resource),
            current_handler_profile_ref: Some(&changed_handler),
            current_provenance_refs: std::slice::from_ref(&provenance),
            current_source_gate_refs: std::slice::from_ref(&source_gate),
            current_retention_refs: std::slice::from_ref(&retention),
            current_evidence_refs: std::slice::from_ref(&evidence),
            compatibility_refs: &compatibility_refs,
            ..cache_hit_validity_input(&put.key, &put.value)
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

    #[test]
    fn invalidation_requires_retention_pass_before_tombstone() {
        let root = temp_dir("eval-cache-retention");
        let key = key_input("schema-fingerprint", "retained-input", &[]);
        let output = record("fingerprint", vec![string("retained")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put retained cache value");
        crate::retention::pin_object(&root, crate::retention::PinInput {
            object_ref: put.key.key_ref.clone(),
            object_kind: "eval-cache-key".to_string(),
            retention_class: crate::retention::CLASS_EPHEMERAL_CACHE.to_string(),
            source: crate::retention::SOURCE_EVALUATION_CACHE.to_string(),
            reason: "cache hold".to_string(),
            owner_ref: test_ref("retention-owner"),
            expiry_ref: None,
            policy_refs: vec![test_ref("retention-policy")],
            evidence_refs: vec![test_ref("retention-evidence")],
            has_authority: true,
        })
        .expect("retention pin");
        let invalidated = invalidate(&root, &InvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "retained".to_string(),
            retention_evidence: retention_evidence(&root, "retained-invalidate"),
            ..InvalidateInput::default()
        })
        .expect("invalidate retained key");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        assert!(!invalidated.retention_receipt_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &GetInput::default()).expect("retained cache hit");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn invalidation_denies_missing_apply_ref_before_tombstone() {
        let root = temp_dir("eval-cache-missing-apply");
        let key = key_input("schema-fingerprint", "missing-apply", &[]);
        let output = record("fingerprint", vec![string("missing-apply")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put cache value");
        let invalidated = invalidate(&root, &InvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "missing apply".to_string(),
            retention_evidence: retention_evidence(&root, "missing-apply"),
            apply_refs: Vec::new(),
            ..InvalidateInput::default()
        })
        .expect("invalidate denied without apply");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        assert!(!invalidated.execution_gate_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &GetInput::default()).expect("cache value remains");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn invalidation_denies_missing_authority_evidence() {
        let root = temp_dir("eval-cache-missing-authority");
        let key = key_input("schema-fingerprint", "missing-authority", &[]);
        let output = record("fingerprint", vec![string("missing-authority")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put cache value");
        let mut retention_evidence = retention_evidence(&root, "missing-authority");
        retention_evidence.authority_refs.clear();
        let invalidated = invalidate(&root, &InvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "missing authority".to_string(),
            retention_evidence,
            ..InvalidateInput::default()
        })
        .expect("invalidate denied");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &GetInput::default()).expect("cache value remains");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn invalidation_denies_retained_reference_evidence() {
        let root = temp_dir("eval-cache-retained-ref");
        let key = key_input("schema-fingerprint", "retained-ref", &[]);
        let output = record("fingerprint", vec![string("retained-ref")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put cache value");
        let mut retention_evidence = retention_evidence(&root, "retained-ref");
        retention_evidence.retained_refs = vec![test_ref("retained-dependent-receipt")];
        let invalidated = invalidate(&root, &InvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "retained ref".to_string(),
            retention_evidence,
            ..InvalidateInput::default()
        })
        .expect("invalidate denied");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &GetInput::default()).expect("cache value remains");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn helper_keys_cover_schema_and_artifact_operations() {
        let shape = record("shape", vec![string("string")]);
        let (_normalized, shape_ref, fingerprint) =
            crate::schema_identity::structural_fingerprint(&shape).expect("fingerprint");
        let key = schema_fingerprint_key_input(&shape_ref, &test_ref("tool"), "v1", &[test_ref("policy")])
            .expect("schema fingerprint key");
        assert_eq!(key.operation, "schema-fingerprint");
        let compat = schema_compatibility_key_input(&SchemaCompatibilityKeyInput {
            expected_identity_ref: &test_ref("expected"),
            actual_identity_ref: &test_ref("actual"),
            alias_ref: Some(&test_ref("alias")),
            migration_ref: None,
            tool_ref: &test_ref("tool"),
            tool_version: "v1",
            policy_refs: &[test_ref("policy")],
        })
        .expect("compat key");
        assert_eq!(compat.operation, "schema-compat");
        assert!(compat.dependency_refs.len() >= 3);
        let closure = artifact_closure_key_input(&ArtifactClosureKeyInput {
            root_refs: &[test_ref("root")],
            closure_hash: &fingerprint,
            dependency_refs: &[test_ref("dep")],
            tool_ref: &test_ref("registry-tool"),
            tool_version: "v1",
            policy_refs: &[test_ref("policy")],
        })
        .expect("closure key");
        assert_eq!(closure.operation, "artifact-closure");
        let choreography = choreography_projection_key_input(&ChoreographyProjectionKeyInput {
            protocol_artifact_ref: &test_ref("protocol"),
            role_ref: &test_ref("role"),
            closure_hash: &fingerprint,
            dependency_refs: &[test_ref("protocol-dep")],
            projector_ref: &test_ref("trellis-projector"),
            projector_version: "v1",
            policy_refs: &[test_ref("projection-policy")],
        })
        .expect("choreography projection key");
        assert_eq!(choreography.operation, "choreography-projection");
        assert!(choreography.dependency_refs.contains(&test_ref("protocol-dep")));
        let wasm =
            wasm_inspection_key_placeholder(&test_ref("module"), &test_ref("inspector"), "v1").expect("wasm key");
        assert_eq!(wasm.operation, "wasm-inspection");
        let transcript = transcript_run_key_placeholder(&TranscriptRunKeyInput {
            transcript_ref: &test_ref("transcript"),
            closure_hash: &fingerprint,
            dependency_refs: &[test_ref("transcript-dep")],
            handler_profile_ref: &test_ref("handler-profile"),
            harness_ref: &test_ref("harness"),
            harness_version: "v1",
        })
        .expect("transcript key");
        assert_eq!(transcript.operation, "transcript-run");
        assert_eq!(transcript.handler_profile_ref, Some(test_ref("handler-profile")));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_key_determinism_dependency_invalidation_and_no_name_key(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dependency = test_ref(&format!("dep-{salt}"));
        let root = temp_dir("eval-cache-hegel");
        let key = key_input("artifact-closure", &format!("input-{salt}"), std::slice::from_ref(&dependency));
        let first_key_ref = canonical_hash(&key_value(&key).expect("first key")).expect("first key ref");
        let second_key_ref = canonical_hash(&key_value(&key).expect("second key")).expect("second key ref");
        assert_eq!(first_key_ref, second_key_ref);
        let output = record("closure", vec![string(&dependency)]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output), &key, &[])).expect("put");
        let retention_evidence = retention_evidence(&root, "hegel-invalidate");
        let apply_refs = vec![apply_ref(&root, &put.key.key_ref, &retention_evidence)];
        let invalidated = invalidate(&root, &InvalidateInput {
            dependency_ref: Some(dependency),
            reason: "property dependency invalidation".to_string(),
            retention_evidence,
            apply_refs,
            ..InvalidateInput::default()
        })
        .expect("invalidate");
        assert!(invalidated.invalidated_key_refs.contains(&put.key.key_ref));
        let display_name_key = KeyInput {
            assumption_refs: vec![test_ref(&format!("name-{salt}"))],
            ..key
        };
        let display_name_key_ref =
            canonical_hash(&key_value(&display_name_key).expect("display key")).expect("display key ref");
        assert_ne!(put.key.key_ref, display_name_key_ref);
        get(&root, &display_name_key_ref, &GetInput::default()).expect_err("name-only alias misses cache");
    }

    fn key_input(operation: &str, input_label: &str, dependency_refs: &[String]) -> KeyInput {
        let deps = dependency_refs.to_vec();
        let input_ref = test_ref(input_label);
        KeyInput {
            operation: operation.to_string(),
            version: "v1".to_string(),
            input_ref: input_ref.clone(),
            input_refs: vec![input_ref],
            dependency_closure_hash: canonical_hash(&record("test-closure", vec![refs_sequence(&deps)]))
                .expect("closure"),
            dependency_refs: deps,
            handler_profile_ref: None,
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            revocation_refs: Vec::new(),
            tool_ref: test_ref("tool"),
            tool_version: "test-v1".to_string(),
            assumption_refs: Vec::new(),
            ..KeyInput::default()
        }
    }

    fn cache_hit_validity_input<'a>(key: &'a Key, value: &'a Value) -> CacheHitValidityInput<'a> {
        CacheHitValidityInput {
            key,
            value,
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
            compatibility_refs: &[],
            requested_dependency_refs: &[],
            expected_output_ref: None,
            semantic: true,
        }
    }
