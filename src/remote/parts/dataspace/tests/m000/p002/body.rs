
    fn traversal_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    #[test]
    fn deterministic_traversal_plans_receiver_missing_set_and_denies_extra_refs() {
        let root_a = traversal_ref("root-a");
        let root_b = traversal_ref("root-b");
        let policy_ref = traversal_ref("policy");
        let evidence_ref = traversal_ref("evidence");
        let descriptor = TraversalDescriptor {
            traversal_kind: TRAVERSAL_ARTIFACT_CLOSURE.to_string(),
            root_refs: vec![root_b.clone(), root_a.clone()],
            visited_refs: Vec::new(),
            order: TRAVERSAL_ORDER_LEXICOGRAPHIC.to_string(),
            filters: Vec::new(),
            inline_policy: INLINE_POLICY_METADATA_ONLY.to_string(),
            resource_bound: MIN_TRAVERSAL_BOUND,
            replay_bound: MIN_TRAVERSAL_BOUND,
            policy_refs: vec![policy_ref],
            evidence_refs: vec![evidence_ref],
        };
        let inventory = LocalInventorySummary {
            verified_refs: vec![root_a.clone()],
            chunk_refs: Vec::new(),
        };
        let plan = plan_traversal(&descriptor, &inventory).expect("traversal plan");
        assert_eq!(plan.decision, "pass");
        assert_eq!(plan.already_present_refs, vec![root_a]);
        assert_eq!(plan.fetch_refs, vec![root_b.clone()]);
        assert!(plan.replayable);

        let response = validate_traversal_response(&TraversalResponseInput {
            plan: &plan,
            response_refs: std::slice::from_ref(&root_b),
            inline_data_refs: &[],
        })
        .expect("response pass");
        assert_eq!(response.decision, "pass");

        let extra_ref = traversal_ref("extra");
        let denied = validate_traversal_response(&TraversalResponseInput {
            plan: &plan,
            response_refs: &[root_b, extra_ref],
            inline_data_refs: &[],
        })
        .expect("response deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("unrequested")));
    }

    #[test]
    fn external_digest_mapping_requires_both_external_and_blake3_match() {
        let bytes = b"remote bytes";
        let content_ref = content_ref_from_bytes(bytes);
        let external_digest = external_digest_for(EXTERNAL_DIGEST_CID_SHA2_256, bytes);
        let evidence_ref = traversal_ref("digest-evidence");
        let admitted = validate_external_digest_mapping(&ExternalDigestMappingInput {
            algorithm: EXTERNAL_DIGEST_CID_SHA2_256,
            external_digest: &external_digest,
            bytes,
            expected_content_ref: &content_ref,
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("digest mapping pass");
        assert_eq!(admitted.decision, "pass");
        assert_eq!(admitted.content_ref, content_ref);

        let wrong_content_ref = traversal_ref("wrong-content");
        let denied = validate_external_digest_mapping(&ExternalDigestMappingInput {
            expected_content_ref: &wrong_content_ref,
            ..ExternalDigestMappingInput {
                algorithm: EXTERNAL_DIGEST_CID_SHA2_256,
                external_digest: &external_digest,
                bytes,
                expected_content_ref: &admitted.content_ref,
                evidence_refs: std::slice::from_ref(&evidence_ref),
            }
        })
        .expect("digest mapping deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("expected")));
        assert!(validate_external_digest_mapping(&ExternalDigestMappingInput {
            algorithm: "md5",
            external_digest: &external_digest,
            bytes,
            expected_content_ref: &admitted.content_ref,
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .is_err());
    }
