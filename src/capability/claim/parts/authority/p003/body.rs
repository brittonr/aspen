
#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn selector() -> ClaimSubjectSelector {
        ClaimSubjectSelector {
            selector_kind: SELECTOR_EXACT_REF.to_string(),
            selector_value: local_ref("subject"),
            subject_kind: "artifact".to_string(),
            policy_refs: refs("selector-policy"),
            resource_refs: refs("selector-resource"),
            caveats: Vec::new(),
        }
    }

    fn claim(selector_ref: String) -> AuthorityClaim {
        AuthorityClaim {
            issuer_ref: local_ref("issuer"),
            holder_ref: local_ref("holder"),
            session_ref: local_ref("session"),
            context_ref: local_ref("context"),
            subject_selector_ref: selector_ref,
            exact_subject_refs: refs("subject"),
            claim_kind: "class-membership".to_string(),
            claim_value_ref: local_ref("claim-value"),
            evidence_refs: refs("claim-evidence"),
            policy_refs: refs("claim-policy"),
            resource_refs: refs("claim-resource"),
            freshness_ref: local_ref("claim-freshness"),
            revocation_refs: refs("revocation-clean"),
            caveats: vec!["evidence-only".to_string()],
        }
    }

    fn admitted_capability(selector_ref: &str) -> crate::capability_tokens::CapabilityAdmissionReceipt {
        let token = crate::capability_tokens::CapabilityToken {
            token_kind: CLAIM_TOKEN_KIND.to_string(),
            issuer_ref: local_ref("issuer"),
            holder_ref: local_ref("holder"),
            session_ref: local_ref("session"),
            context_ref: local_ref("context"),
            resource_ref: selector_ref.to_string(),
            ability: CLAIM_ATTEST_ABILITY.to_string(),
            scope: "class-membership".to_string(),
            attenuation: "selector-exact".to_string(),
            caveats: Vec::new(),
            expires_at_tick: 10,
            revoked_refs: Vec::new(),
            policy_refs: refs("local-policy"),
            resource_refs: refs("local-resource"),
            delegation_refs: refs("delegation"),
            evidence_refs: refs("token-evidence"),
        };
        let proofset = crate::capability_tokens::CapabilityProofset {
            holder_ref: local_ref("holder"),
            session_ref: local_ref("session"),
            context_ref: local_ref("context"),
            tokens: vec![token],
            policy_refs: refs("local-policy"),
            resource_refs: refs("local-resource"),
            revocation_refs: Vec::new(),
            evidence_refs: refs("proofset-evidence"),
        };
        let request = claim_capability_request(RequestInput {
            holder_ref: &local_ref("holder"),
            session_ref: &local_ref("session"),
            context_ref: &local_ref("context"),
            selector_ref,
            claim_kind: "class-membership",
            at_tick: 1,
            policy_refs: &refs("local-policy"),
            resource_refs: &refs("local-resource"),
        })
        .expect("request");
        crate::capability_tokens::admit_capability(&proofset, &request).expect("capability admission")
    }

    fn admission_input() -> ClaimAdmissionInput {
        let selector = selector();
        let selector_ref =
            canonical_hash(&claim_subject_selector_value(&selector).expect("selector")).expect("selector ref");
        ClaimAdmissionInput {
            selector,
            claim: claim(selector_ref.clone()),
            at_tick: 1,
            capability_admission: Some(admitted_capability(&selector_ref)),
            ucan_verification_refs: refs("ucan"),
            basalt_enforcement_refs: refs("basalt"),
            local_policy_refs: refs("local-policy"),
            local_resource_refs: refs("local-resource"),
            freshness_refs: refs("freshness"),
            revocation_refs: refs("revocation-clean"),
            peer_context_refs: refs("peer-session"),
            transport_observation_refs: Vec::new(),
            registry_discovery_refs: Vec::new(),
            local_fixture_grant_refs: Vec::new(),
        }
    }

    // r[verify molten.claim_authority.subject_selectors]
    // r[verify molten.claim_authority.claim_records]
    // r[verify molten.claim_authority.capability_profile]
    // r[verify molten.claim_authority.positive_negative_tests]
    #[test]
    fn admitted_external_claim_binds_capability_ucan_and_basalt_path() {
        let admission = admit_authority_claim(&admission_input()).expect("claim admission");
        assert_eq!(admission.decision, DECISION_PASS);
        assert!(
            crate::preserves_rail::to_text(&admission.value)
                .expect("admission text")
                .contains("authority-claim-admission-v1")
        );
    }

    #[test]
    fn missing_proof_transport_registry_and_fixture_fallback_deny() {
        let mut input = admission_input();
        input.capability_admission = None;
        input.ucan_verification_refs.clear();
        input.basalt_enforcement_refs.clear();
        input.transport_observation_refs = refs("transport");
        input.registry_discovery_refs = refs("registry");
        input.local_fixture_grant_refs = refs("fixture-grant");
        let admission = admit_authority_claim(&input).expect("claim admission");
        assert_eq!(admission.decision, DECISION_DENY);
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic == "missing-claim-capability-admission"));
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "transport-evidence-is-not-claim-authority")
        );
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "registry-discovery-is-not-claim-authority")
        );
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "local-fixture-grant-cannot-satisfy-ucan-basalt-claim")
        );
    }

    #[test]
    fn broad_selector_without_attenuation_denies_visibly() {
        let mut input = admission_input();
        input.selector.selector_kind = SELECTOR_NAMESPACE.to_string();
        input.selector.selector_value = "cluster:friend".to_string();
        input.selector.caveats.clear();
        let selector_ref =
            canonical_hash(&claim_subject_selector_value(&input.selector).expect("selector")).expect("selector ref");
        input.claim.subject_selector_ref = selector_ref;
        let admission = admit_authority_claim(&input).expect("claim admission");
        assert_eq!(admission.decision, DECISION_DENY);
        assert!(
            admission
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "broad-selector-without-visible-attenuation")
        );
    }

    // r[verify molten.claim_authority.downstream_consumption]
    #[test]
    fn downstream_claim_use_is_exact_and_wrong_kind_denies() {
        let admission = admit_authority_claim(&admission_input()).expect("claim admission");
        let use_decision = decide_claim_use(&ClaimUseInput {
            admission: admission.clone(),
            required_selector_ref: admission.selector_ref.clone(),
            required_claim_kind: "class-membership".to_string(),
            subject_ref: local_ref("subject"),
            subsystem: "release-gate".to_string(),
            policy_refs: refs("subsystem-policy"),
            resource_refs: refs("subsystem-resource"),
            freshness_ref: local_ref("freshness"),
        })
        .expect("use decision");
        assert_eq!(use_decision.decision, DECISION_PASS);

        let required_selector_ref = admission.selector_ref.clone();
        let wrong_kind = decide_claim_use(&ClaimUseInput {
            admission,
            required_selector_ref,
            required_claim_kind: "release-channel-attestation".to_string(),
            subject_ref: local_ref("subject"),
            subsystem: "release-gate".to_string(),
            policy_refs: refs("subsystem-policy"),
            resource_refs: refs("subsystem-resource"),
            freshness_ref: local_ref("freshness"),
        })
        .expect("wrong kind");
        assert_eq!(wrong_kind.decision, DECISION_DENY);
        assert!(wrong_kind.diagnostics.iter().any(|diagnostic| diagnostic == "claim-kind-mismatch"));
    }

    // r[verify molten.claim_authority.registry_readback]
    // r[verify molten.claim_authority.registry_tests]
    #[test]
    fn registry_readback_classifies_without_authority() {
        let selector_value = claim_subject_selector_value(&selector()).expect("selector");
        assert_eq!(crate::ledger::artifact_kind(&selector_value), "claim-subject-selector");
        let summary = claim_readback_summary(&selector_value).expect("summary").expect("some summary");
        assert!(summary.contains("evidence candidate only"));
    }

    // r[verify molten.claim_authority.peer_diagnostics]
    // r[verify molten.claim_authority.peer_diagnostic_tests]
    #[test]
    fn peer_claim_diagnostics_separate_transport_from_claim_authority() {
        let admission = admit_authority_claim(&admission_input()).expect("claim admission");
        let pass = peer_claim_authority_diagnostic(&PeerClaimDiagnosticInput {
            peer_ref: local_ref("peer"),
            bootstrap_ref: Some(local_ref("bootstrap")),
            session_ref: Some(local_ref("session")),
            transport_refs: refs("transport"),
            claim_admission: Some(admission.clone()),
            claim_kind: "class-membership".to_string(),
            selector_ref: admission.selector_ref,
        })
        .expect("peer diagnostic");
        assert_eq!(pass.decision, DECISION_PASS);
        assert!(pass.diagnostics.iter().any(|diagnostic| diagnostic == "peer-transport-observed-context-only"));

        let deny = peer_claim_authority_diagnostic(&PeerClaimDiagnosticInput {
            peer_ref: local_ref("peer"),
            bootstrap_ref: Some(local_ref("bootstrap")),
            session_ref: Some(local_ref("session")),
            transport_refs: refs("transport"),
            claim_admission: None,
            claim_kind: "class-membership".to_string(),
            selector_ref: local_ref("selector"),
        })
        .expect("peer diagnostic deny");
        assert_eq!(deny.decision, DECISION_DENY);
        assert!(
            deny.diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "peer-claim-authority-missing-capability-ucan-basalt-proof")
        );
    }
}
