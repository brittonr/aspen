
    #[test]
    fn trace_privacy_gates_sensitive_trace_and_snapshot_exports() {
        let input = TracePrivacyInput {
            trace_ref: DEFAULT_ARTIFACT_REF.to_string(),
            snapshot_ref: DEFAULT_INITIAL_STATE_REF.to_string(),
            requester_ref: DEFAULT_CAPABILITY_REF.to_string(),
            policy_ref: DEFAULT_POLICY_REF.to_string(),
            has_export_authority: false,
            contains_sensitive_refs: true,
        };
        let denied = trace_privacy_receipt(&input).expect("trace privacy deny");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.receipt_ref, canonical_hash(&denied.value).expect("privacy receipt ref"));
        let denied_text = to_text(&denied.value).expect("render denied privacy receipt");
        assert!(denied_text.contains("policy-admission-before-render"));
        assert!(denied_text.contains("sensitive-trace-gated"));

        let redacted = trace_privacy_receipt(&TracePrivacyInput {
            has_export_authority: true,
            ..input.clone()
        })
        .expect("trace privacy redacted");
        assert_eq!(redacted.decision, "redacted");
        let redacted_text = to_text(&redacted.value).expect("render redacted privacy receipt");
        assert!(redacted_text.contains("redacted-view-when-authorized-sensitive"));

        let public = trace_privacy_receipt(&TracePrivacyInput {
            contains_sensitive_refs: false,
            ..input
        })
        .expect("trace privacy public");
        assert_eq!(public.decision, "pass");
    }
