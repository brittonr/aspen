#[cfg(test)]
mod tests {
    use super::*;

    const CONNECT_TICK: u64 = 4;

    #[test]
    fn lifecycle_reaches_connected_with_required_evidence() {
        let bootstrap_ref = test_ref("bootstrap");
        let authority_ref = test_ref("authority");
        let prior = record(StateKind::Admitted, vec![bootstrap_ref.clone()], vec![authority_ref.clone()]);
        let receipt = apply_transition(&TransitionInput {
            prior,
            event: EventKind::Connect,
            target: StateKind::Connected,
            observed_topic: "node-control".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: Some(bootstrap_ref),
            required_authority_ref: Some(authority_ref),
            required_recovery_ref: None,
            revocation_ref: None,
        })
        .expect("transition");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.session.state, StateKind::Connected);
    }

    #[test]
    fn wrong_topic_and_missing_authority_deny() {
        let prior = record(StateKind::Admitted, vec![test_ref("bootstrap")], Vec::new());
        let receipt = apply_transition(&TransitionInput {
            prior,
            event: EventKind::Connect,
            target: StateKind::Connected,
            observed_topic: "wrong-topic".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: None,
            required_authority_ref: Some(test_ref("authority")),
            required_recovery_ref: None,
            revocation_ref: None,
        })
        .expect("transition");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("wrong topic")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing authority")));
        assert_eq!(receipt.session.state, StateKind::Admitted);
    }

    #[test]
    fn transition_table_denies_skips_and_quarantine_bypass() {
        let bootstrap_ref = test_ref("bootstrap");
        let prior = record(StateKind::Discovered, vec![bootstrap_ref.clone()], vec![test_ref("authority")]);
        let skip = apply_transition(&TransitionInput {
            prior,
            event: EventKind::Connect,
            target: StateKind::Connected,
            observed_topic: "node-control".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: Some(bootstrap_ref),
            required_authority_ref: Some(test_ref("authority")),
            required_recovery_ref: None,
            revocation_ref: None,
        })
        .expect("skip denial");
        assert_eq!(skip.decision, "deny");
        assert!(skip.diagnostics.iter().any(|diagnostic| diagnostic.contains("reviewed table")));

        let quarantined = record(StateKind::Quarantined, Vec::new(), vec![test_ref("authority")]);
        let bypass = apply_transition(&TransitionInput {
            prior: quarantined,
            event: EventKind::Connect,
            target: StateKind::Connected,
            observed_topic: "node-control".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: None,
            required_authority_ref: Some(test_ref("authority")),
            required_recovery_ref: None,
            revocation_ref: None,
        })
        .expect("quarantine bypass denial");
        assert_eq!(bypass.decision, "deny");
        assert!(bypass.diagnostics.iter().any(|diagnostic| diagnostic.contains("recovery")));
    }

    #[test]
    fn transition_receipt_binds_state_event_and_guards() {
        let bootstrap_ref = test_ref("bootstrap");
        let prior = record(StateKind::Negotiated, vec![bootstrap_ref.clone()], Vec::new());
        let receipt = apply_transition(&TransitionInput {
            prior,
            event: EventKind::Admit,
            target: StateKind::Admitted,
            observed_topic: "node-control".to_string(),
            at_tick: CONNECT_TICK,
            required_bootstrap_ref: Some(bootstrap_ref.clone()),
            required_authority_ref: None,
            required_recovery_ref: None,
            revocation_ref: None,
        })
        .expect("admission");
        let text = crate::preserves_rail::to_text(&receipt.value).expect("receipt text");
        assert!(text.contains("prior-state"));
        assert!(text.contains("admit"));
        assert!(text.contains(&bootstrap_ref));
        assert!(text.contains("reviewed-transition-table"));
    }

    #[test]
    fn connected_state_is_not_authority() {
        let receipt = record_as_authority_denial(&test_ref("session"), "publish").expect("denial");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics[0].contains("not authority"));
    }

    fn record(state: StateKind, bootstrap_refs: Vec<String>, authority_refs: Vec<String>) -> Record {
        Record {
            peer_ref: test_ref("peer"),
            session_ref: test_ref("session"),
            topic: "node-control".to_string(),
            state,
            bootstrap_refs,
            capability_refs: Vec::new(),
            authority_refs,
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            diagnostics: Vec::new(),
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("peer-session-test-ref", vec![string(
            label,
        )]))
        .expect("test ref")
    }
}
