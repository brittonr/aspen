
    struct Case {
        root: std::path::PathBuf,
        scope: String,
        policy_refs: Vec<String>,
        evidence_refs: Vec<String>,
        payload_ref: String,
        result_ref: String,
    }

    struct Attempt {
        sequence: u64,
        payload_ref: String,
        semantic_result_ref: Option<String>,
        gap_policy: GapPolicy,
    }

    fn negative_case() -> Case {
        Case {
            root: temp_dir("delivery-negative"),
            scope: remote_topic_scope_ref("services", "peer:b").expect("scope"),
            policy_refs: vec![fake_ref("policy")],
            evidence_refs: vec![fake_ref("evidence")],
            payload_ref: fake_ref("payload"),
            result_ref: fake_ref("result"),
        }
    }

    fn attempt(sequence: u64, payload_label: &str, result_label: Option<&str>, gap_policy: GapPolicy) -> Attempt {
        Attempt {
            sequence,
            payload_ref: fake_ref(payload_label),
            semantic_result_ref: result_label.map(fake_ref),
            gap_policy,
        }
    }

    fn assert_first(case: &Case) {
        let first = check_case(
            case,
            Attempt {
                sequence: 1,
                payload_ref: case.payload_ref.clone(),
                semantic_result_ref: Some(case.result_ref.clone()),
                gap_policy: GapPolicy::Deny,
            },
            "first",
        );
        assert_eq!(first.receipt.decision, "first");
    }

    fn assert_conflict(case: &Case) {
        let conflict =
            check_case(case, attempt(1, "changed-payload", Some("changed-result"), GapPolicy::Deny), "conflict");
        assert_eq!(conflict.receipt.decision, "conflict");
        assert!(!conflict.should_commit_side_effect);
    }

    fn assert_denied(case: &Case, attempt: Attempt, decision: &str) {
        let denied = check_case(case, attempt, decision);
        assert_eq!(denied.receipt.decision, decision);
    }

    fn check_case(case: &Case, attempt: Attempt, context: &str) -> Decision {
        check(CheckInput {
            root: &case.root,
            scope_profile: SCOPE_REMOTE_TOPIC,
            scope_ref: &case.scope,
            producer: "peer:a/producer",
            consumer: "peer:b",
            sequence: attempt.sequence,
            intent: "remote-dataspace-message",
            payload_ref: &attempt.payload_ref,
            policy_refs: &case.policy_refs,
            evidence_refs: &case.evidence_refs,
            semantic_result_ref: attempt.semantic_result_ref.as_deref(),
            gap_policy: attempt.gap_policy,
        })
        .expect(context)
    }

    #[derive(Debug, Clone, Copy)]
    struct TraceStep {
        producer: &'static str,
        sequence: u64,
        payload_label: &'static str,
        evidence_label: &'static str,
        gap_policy: GapPolicy,
        expected_decision: &'static str,
        should_commit_side_effect: bool,
    }

    fn trace_step(
        producer: &'static str,
        sequence: u64,
        payload_label: &'static str,
        evidence_label: &'static str,
        gap_policy: GapPolicy,
        expected_decision: &'static str,
        should_commit_side_effect: bool,
    ) -> TraceStep {
        TraceStep {
            producer,
            sequence,
            payload_label,
            evidence_label,
            gap_policy,
            expected_decision,
            should_commit_side_effect,
        }
    }

    fn operation_for(
        scope: &str,
        producer: &str,
        sequence: u64,
        payload_label: &str,
        policy_refs: &[String],
    ) -> OperationId {
        derive_operation_id(OperationIdInput {
            scope_ref: scope.to_string(),
            producer: producer.to_string(),
            consumer: "peer:b".to_string(),
            sequence,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: fake_ref(payload_label),
            policy_refs: policy_refs.to_vec(),
        })
        .expect("operation id")
    }

    fn parsed_window(scope: &str, next_sequence: u64, lowest_retained: u64, retention_refs: &[String]) -> Window {
        parse_window(
            &window_value(SCOPE_REMOTE_TOPIC, scope, next_sequence, lowest_retained, retention_refs)
                .expect("window value"),
        )
        .expect("window")
    }

    fn entry_for(
        operation: &OperationId,
        evidence_refs: &[String],
        receipt_label: &str,
        semantic_label: &str,
    ) -> DedupEntry {
        let semantic_result_ref = fake_ref(semantic_label);
        let dedup_key = dedup_key_ref(operation).expect("dedup key");
        parse_dedup_entry(
            &dedup_entry_value(DedupEntryValueInput {
                dedup_key: &dedup_key,
                operation,
                semantic_result_ref: Some(&semantic_result_ref),
                first_receipt_ref: &fake_ref(receipt_label),
                evidence_refs,
            })
            .expect("entry value"),
        )
        .expect("entry")
    }

    #[test]
    fn hegel_like_no_global_sequence_invariant_for_independent_scopes() {
        for sequence in 1..8_u64 {
            let root = temp_dir("delivery-scopes");
            let left_scope = remote_topic_scope_ref("services", "peer:left").expect("left scope");
            let right_scope = remote_topic_scope_ref("services", "peer:right").expect("right scope");
            let policy_refs = vec![fake_ref("policy")];
            let evidence_refs = vec![fake_ref("evidence")];
            let left = check(CheckInput {
                root: &root,
                scope_profile: SCOPE_REMOTE_TOPIC,
                scope_ref: &left_scope,
                producer: "peer:a/producer",
                consumer: "peer:left",
                sequence,
                intent: "remote-dataspace-assert",
                payload_ref: &fake_ref("payload-left"),
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: Some(&fake_ref("left-result")),
                gap_policy: if sequence == 1 {
                    GapPolicy::Deny
                } else {
                    GapPolicy::Retry
                },
            })
            .expect("left delivery");
            let right = check(CheckInput {
                root: &root,
                scope_profile: SCOPE_REMOTE_TOPIC,
                scope_ref: &right_scope,
                producer: "peer:a/producer",
                consumer: "peer:right",
                sequence: 1,
                intent: "remote-dataspace-assert",
                payload_ref: &fake_ref("payload-right"),
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: Some(&fake_ref("right-result")),
                gap_policy: GapPolicy::Deny,
            })
            .expect("right delivery");
            assert_eq!(right.receipt.decision, "first");
            if sequence == 1 {
                assert_eq!(left.receipt.decision, "first");
            } else {
                assert_eq!(left.receipt.decision, "retry");
            }
        }
    }

    fn fake_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fake-ref", vec![
            crate::preserves_rail::string(label),
        ]))
        .expect("fake ref")
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
