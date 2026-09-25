
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
