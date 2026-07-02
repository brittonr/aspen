
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_identity_is_canonical_and_payload_sensitive() {
        let scope = remote_topic_scope_ref("services", "peer:b").expect("scope");
        let payload_a = fake_ref("payload-a");
        let payload_b = fake_ref("payload-b");
        let left = derive_operation_id(OperationIdInput {
            scope_ref: scope.clone(),
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 7,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_a.clone(),
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("left operation");
        let right = derive_operation_id(OperationIdInput {
            scope_ref: scope.clone(),
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 7,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_a,
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("right operation");
        let changed = derive_operation_id(OperationIdInput {
            scope_ref: scope,
            producer: "peer:a/producer".to_string(),
            consumer: "peer:b".to_string(),
            sequence: 7,
            intent: "remote-dataspace-assert".to_string(),
            payload_ref: payload_b,
            policy_refs: vec![fake_ref("policy")],
        })
        .expect("changed operation");
        assert_eq!(left.operation_ref, right.operation_ref);
        assert_ne!(left.operation_ref, changed.operation_ref);
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
}
