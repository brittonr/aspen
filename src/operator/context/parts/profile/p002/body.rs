
#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs() -> ContextRefSet {
        ContextRefSet {
            policy_refs: vec![local_ref("policy")],
            capability_refs: vec![local_ref("capability")],
            authority_refs: vec![local_ref("authority")],
            resource_refs: vec![local_ref("resource")],
            evidence_refs: vec![local_ref("evidence")],
            redaction_refs: vec![local_ref("redaction")],
            retention_refs: vec![local_ref("retention")],
        }
    }

    fn profile() -> ContextProfileInput {
        ContextProfileInput {
            profile_id: "operator:node-control".to_string(),
            profile_tier: "pilot".to_string(),
            refs: refs(),
            allowed_operations: vec!["node.status".to_string(), "node.install".to_string()],
            caveats: vec!["pilot only".to_string()],
        }
    }

    fn requirements(operation: &str) -> OperationRequirements {
        OperationRequirements {
            operation: operation.to_string(),
            require_policy: true,
            require_authority: true,
            require_resource: true,
            require_evidence: true,
            require_retention: false,
        }
    }

    fn empty_overrides() -> ContextOverrideInput {
        ContextOverrideInput {
            policy_refs: Vec::new(),
            authority_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            retention_refs: Vec::new(),
        }
    }

    // r[verify molten.operator_workflow.context_profile.artifact]
    // r[verify molten.operator_workflow.context_profile.expansion]
    // r[verify molten.operator_workflow.context_profile.overrides]
    // r[verify molten.operator_workflow.context_profile.evidence_only]
    #[test]
    fn context_profile_artifact_and_expansion_pass_for_valid_refs() {
        let artifact = build_context_profile_artifact(&profile()).expect("profile artifact");
        assert_eq!(artifact.decision, DECISION_PASS);
        let expansion = expand_context_profile(&profile(), &requirements("node.status"), &empty_overrides())
            .expect("context expansion");
        assert_eq!(expansion.decision, DECISION_PASS);
        assert_eq!(expansion.expanded_refs.policy_refs, refs().policy_refs);
    }

    #[test]
    fn context_profile_denies_malformed_refs_and_unsupported_scope() {
        let mut profile = profile();
        profile.refs.authority_refs = vec!["not-a-ref".to_string()];
        let expansion = expand_context_profile(&profile, &requirements("retention.delete"), &empty_overrides())
            .expect("context expansion");
        assert_eq!(expansion.decision, DECISION_DENY);
        assert!(
            expansion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("stale-ref:authority:not-a-ref"))
        );
        assert!(
            expansion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "unsupported-operation-scope:retention.delete")
        );
    }

    #[test]
    fn context_profile_allows_additive_evidence_but_denies_conflicting_authority() {
        let mut overrides = empty_overrides();
        overrides.evidence_refs = vec![local_ref("extra-evidence")];
        let expansion =
            expand_context_profile(&profile(), &requirements("node.install"), &overrides).expect("context expansion");
        assert_eq!(expansion.decision, DECISION_PASS);
        assert!(expansion.expanded_refs.evidence_refs.contains(&local_ref("extra-evidence")));

        let mut conflicting = empty_overrides();
        conflicting.authority_refs = vec![local_ref("other-authority")];
        let denied = expand_context_profile(&profile(), &requirements("node.install"), &conflicting)
            .expect("denied context expansion");
        assert_eq!(denied.decision, DECISION_DENY);
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic == "conflicting-authority-override"));
    }

    #[test]
    fn context_profile_presence_cannot_authorize_mutation_by_itself() {
        let artifact = build_context_profile_artifact(&profile()).expect("profile artifact");
        let decision = evaluate_context_profile_authorization_use(&artifact.profile_ref, "node.install", &[])
            .expect("authorization use");
        assert_eq!(decision.decision, DECISION_DENY);
        assert!(decision.diagnostics.iter().any(|diagnostic| diagnostic == "context-profile-is-not-authority"));
        assert!(
            decision
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-expanded-authority:node.install")
        );
    }
}
