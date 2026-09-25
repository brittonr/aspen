    use super::*;

    #[test]
    fn operation_identity_is_canonical_and_payload_sensitive() {
        const PAYLOAD_SENSITIVE_SEQUENCE: u64 = 7;
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let payload_a = fake_ref("payload-a");
        let payload_b = fake_ref("payload-b");
        let left = derive_operation_id(OperationIdInput {
            scope_ref: scope.clone(),
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: PAYLOAD_SENSITIVE_SEQUENCE,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_a.clone(),
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("left operation");
        let right = derive_operation_id(OperationIdInput {
            scope_ref: scope.clone(),
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: PAYLOAD_SENSITIVE_SEQUENCE,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_a,
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("right operation");
        let changed = derive_operation_id(OperationIdInput {
            scope_ref: scope,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: PAYLOAD_SENSITIVE_SEQUENCE,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_b,
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("changed operation");
        assert_eq!(left.operation_ref, right.operation_ref);
        assert_ne!(left.operation_ref, changed.operation_ref);
    }

    #[test]
    fn pure_decision_law_classifies_delivery_outcomes_without_store_io() {
        // r[impl molten.delivery_state_machine_proof.first_commit_duplicate_suppression]
        // r[verify molten.delivery_state_machine_proof.first_commit_duplicate_suppression]
        // r[verify molten.delivery_state_machine_proof.denial_no_side_effect]
        const FIRST_SEQUENCE: u64 = 1;
        const ADVANCED_NEXT_SEQUENCE: u64 = 3;
        const GAP_SEQUENCE: u64 = 4;
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("evidence")];
        let operation = operation_for(&scope, "peer:a/producer", FIRST_SEQUENCE, "payload", &policy_refs);
        let initial_window = parsed_window(&scope, FIRST_SEQUENCE, FIRST_SEQUENCE, &policy_refs);
        let first = classify_law(&operation, &initial_window, None, &evidence_refs, GapPolicy::Deny)
        .expect("first law");
        assert_eq!(first.kind, IdempotencyDecisionKind::First);
        assert!(first.kind.should_commit_side_effect());
        assert!(first.should_commit_side_effect);

        let entry = entry_for(&operation, &evidence_refs, "first-receipt", "semantic-result");
        let duplicate = classify_law(&operation, &initial_window, Some(&entry), &evidence_refs, GapPolicy::Deny)
        .expect("duplicate law");
        assert_eq!(duplicate.kind, IdempotencyDecisionKind::Duplicate);
        assert_eq!(duplicate.prior_receipt_ref.as_deref(), Some(entry.first_receipt_ref.as_str()));
        assert_eq!(duplicate.prior_semantic_result_ref.as_deref(), entry.semantic_result_ref.as_deref());
        assert!(!duplicate.should_commit_side_effect);

        let changed_operation = operation_for(&scope, "peer:a/producer", FIRST_SEQUENCE, "changed-payload", &policy_refs);
        let conflict = classify_law(&changed_operation, &initial_window, Some(&entry), &evidence_refs, GapPolicy::Deny)
        .expect("conflict law");
        assert_eq!(conflict.kind, IdempotencyDecisionKind::Conflict);
        assert!(conflict.diagnostics.iter().any(|diagnostic| diagnostic.contains("different payload")));
        assert!(!conflict.should_commit_side_effect);

        let advanced_window = parsed_window(&scope, ADVANCED_NEXT_SEQUENCE, FIRST_SEQUENCE, &policy_refs);
        let stale_operation = operation_for(&scope, "peer:c/producer", FIRST_SEQUENCE, "stale-payload", &policy_refs);
        let stale = classify_law(&stale_operation, &advanced_window, None, &evidence_refs, GapPolicy::Deny)
        .expect("stale law");
        assert_eq!(stale.kind, IdempotencyDecisionKind::Stale);
        assert!(!stale.should_commit_side_effect);

        let gap_operation = operation_for(&scope, "peer:a/producer", GAP_SEQUENCE, "gap-payload", &policy_refs);
        let gap = classify_law(&gap_operation, &advanced_window, None, &evidence_refs, GapPolicy::Deny)
        .expect("gap law");
        assert_eq!(gap.kind, IdempotencyDecisionKind::Gap);
        let retry = classify_law(&gap_operation, &advanced_window, None, &evidence_refs, GapPolicy::Retry)
        .expect("retry law");
        assert_eq!(retry.kind, IdempotencyDecisionKind::Retry);
    }

    fn classify_law(
        operation: &OperationId,
        window: &Window,
        existing_entry: Option<&DedupEntry>,
        evidence_refs: &[String],
        gap_policy: GapPolicy,
    ) -> Result<IdempotencyDecisionLaw> {
        classify_idempotency_decision(DecisionLawInput {
            operation,
            window,
            existing_entry,
            evidence_refs,
            gap_policy,
        })
    }

    #[test]
    fn malformed_delivery_operation_fails_before_store_side_effects() {
        // r[verify molten.delivery_state_machine_proof.denial_no_side_effect]
        const FIRST_SEQUENCE: u64 = 1;
        const MALFORMED_PAYLOAD_REF: &str = "not-a-content-ref";
        let root = temp_dir("delivery-malformed");
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("evidence")];
        let error = check(CheckInput {
            root: &root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: FIRST_SEQUENCE,
            intent: "remote-dataspace-assert",
            payload_ref: MALFORMED_PAYLOAD_REF,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            semantic_result_ref: None,
            gap_policy: GapPolicy::Deny,
        })
        .expect_err("malformed operation ref fails closed");
        assert!(error.to_string().contains("delivery operation payload ref"));
        assert!(!root.join(STORE_FILE).exists());
    }

    #[test]
    fn bounded_generated_delivery_trace_preserves_idempotency_invariants() {
        // r[verify molten.delivery_state_machine_proof.first_commit_duplicate_suppression]
        // r[verify molten.delivery_state_machine_proof.denial_no_side_effect]
        // r[verify molten.delivery_state_machine_proof.generated_delivery_traces]
        const EXPECTED_COMMITTED_SIDE_EFFECTS: usize = 2;
        const EXPECTED_SUPPRESSED_SIDE_EFFECTS: usize = 5;
        let root = temp_dir("delivery-generated-trace");
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let policy_refs = vec![fake_ref("policy")];
        let trace = generated_trace();
        let mut committed_side_effects = 0_usize;
        let mut suppressed_side_effects = 0_usize;
        for step in trace {
            let evidence_refs = vec![fake_ref(step.evidence_label)];
            let semantic_result_ref = fake_ref(&format!("result-{}", step.payload_label));
            let decision = check(CheckInput {
                root: &root,
                scope_profile: SCOPE_REMOTE_TOPIC,
                scope_ref: &scope,
                producer: step.producer,
                consumer: "peer:b",
                sequence: step.sequence,
                intent: "remote-dataspace-assert",
                payload_ref: &fake_ref(step.payload_label),
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: Some(&semantic_result_ref),
                gap_policy: step.gap_policy,
            })
            .expect(step.expected_decision);
            assert_eq!(decision.receipt.decision, step.expected_decision);
            assert_eq!(decision.should_commit_side_effect, step.should_commit_side_effect);
            if decision.should_commit_side_effect {
                committed_side_effects += 1;
            } else {
                suppressed_side_effects += 1;
                assert_eq!(decision.receipt.side_effect, "suppress");
                if decision.receipt.decision != "duplicate" {
                    assert!(!decision.receipt.diagnostics.is_empty());
                }
            }
        }
        assert_eq!(committed_side_effects, EXPECTED_COMMITTED_SIDE_EFFECTS);
        assert_eq!(suppressed_side_effects, EXPECTED_SUPPRESSED_SIDE_EFFECTS);
    }

    /// A first commit, duplicate, conflict, gap, retry, second commit, and stale producer, in that order.
    fn generated_trace() -> [TraceStep; 7] {
        const FIRST_SEQUENCE: u64 = 1;
        const SECOND_SEQUENCE: u64 = 2;
        const GAP_SEQUENCE: u64 = 4;
        [
            trace_step("peer:a/producer", FIRST_SEQUENCE, "payload-a", "evidence-a", GapPolicy::Deny, "first", true),
            trace_step(
                "peer:a/producer",
                FIRST_SEQUENCE,
                "payload-a",
                "evidence-a",
                GapPolicy::Deny,
                "duplicate",
                false,
            ),
            trace_step(
                "peer:a/producer",
                FIRST_SEQUENCE,
                "changed-payload",
                "evidence-a",
                GapPolicy::Deny,
                "conflict",
                false,
            ),
            trace_step("peer:a/producer", GAP_SEQUENCE, "gap-payload", "evidence-a", GapPolicy::Deny, "gap", false),
            trace_step(
                "peer:a/producer",
                GAP_SEQUENCE,
                "gap-payload",
                "evidence-a",
                GapPolicy::Retry,
                "retry",
                false,
            ),
            trace_step(
                "peer:a/producer",
                SECOND_SEQUENCE,
                "payload-b",
                "evidence-b",
                GapPolicy::Deny,
                "first",
                true,
            ),
            trace_step("peer:c/producer", FIRST_SEQUENCE, "payload-c", "evidence-c", GapPolicy::Deny, "stale", false),
        ]
    }

    #[test]
    fn duplicate_delivery_suppresses_second_side_effect_and_returns_prior_result() {
        let root = temp_dir("delivery-duplicate");
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let policy_refs = vec![fake_ref("policy")];
        let evidence_refs = vec![fake_ref("evidence")];
        let result_ref = fake_ref("semantic-result");
        let first = check(CheckInput {
            root: &root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: 1,
            intent: "remote-dataspace-assert",
            payload_ref: &fake_ref("payload"),
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            semantic_result_ref: Some(&result_ref),
            gap_policy: GapPolicy::Deny,
        })
        .expect("first delivery");
        assert_eq!(first.receipt.decision, "first");
        assert!(first.should_commit_side_effect);
        let duplicate = check(CheckInput {
            root: &root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: 1,
            intent: "remote-dataspace-assert",
            payload_ref: &fake_ref("payload"),
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
            semantic_result_ref: Some(&result_ref),
            gap_policy: GapPolicy::Deny,
        })
        .expect("duplicate delivery");
        assert_eq!(duplicate.receipt.decision, "duplicate");
        assert!(!duplicate.should_commit_side_effect);
        assert_eq!(duplicate.prior_semantic_result_ref.as_deref(), Some(result_ref.as_str()));
    }

    #[test]
    fn conflict_stale_gap_and_retry_are_canonical_denials() {
        let case = negative_case();
        assert_first(&case);
        assert_conflict(&case);
        assert_denied(&case, attempt(0, "stale", None, GapPolicy::Deny), "stale");
        assert_denied(&case, attempt(4, "gap", None, GapPolicy::Deny), "gap");
        assert_denied(&case, attempt(4, "retry", None, GapPolicy::Retry), "retry");
    }
