
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_loopback_handshake_admits_join_with_capability() {
        let policy = NegotiationPolicy::default();
        let local = sample_handshake("local", vec![sample_offer("join:gossip", "topic:updates")], Vec::new());
        let remote = sample_handshake("remote", Vec::new(), vec![JoinRequest {
            kind: "gossip-topic".to_string(),
            target: "topic:updates".to_string(),
            required_capability: "join:gossip".to_string(),
        }]);
        let agreement = negotiate_peers(&local, &remote, &policy).expect("negotiate");
        assert_eq!(agreement.decision, "pass");
        assert_eq!(agreement.admitted_joins.len(), 1);
        assert!(agreement.denied_joins.is_empty());
        assert_eq!(agreement.selected_features.runtime_versions, vec![policy.mandatory_runtime]);
    }

    #[test]
    fn unsafe_downgrade_and_missing_capability_are_denied() {
        let policy = NegotiationPolicy::default();
        let local = sample_handshake("local", Vec::new(), Vec::new());
        let mut remote_features = sample_features();
        remote_features.preserves_boundaries = vec!["legacy-preserves".to_string()];
        let remote = handshake_value(&HandshakeValueInput {
            node_id: "remote",
            identity_ref: &ref_for("remote-identity"),
            endpoint_id: "iroh:remote",
            molten_version: "0.1.0",
            features: &remote_features,
            requested_joins: &[JoinRequest {
                kind: "docs-namespace".to_string(),
                target: "docs:private".to_string(),
                required_capability: "join:docs".to_string(),
            }],
            capability_offers: &[],
            resource_limits: &sample_limits(),
            policy_refs: &[],
            receipt_refs: &[],
        })
        .expect("remote handshake");
        let agreement = negotiate_peers(&local, &remote, &policy).expect("negotiate denial");
        assert_eq!(agreement.decision, "fail");
        assert_eq!(agreement.denied_joins.len(), 1);
        assert!(
            crate::preserves_rail::to_text(&agreement.receipt_value)
                .expect("receipt text")
                .contains("unsafe-downgrade")
        );
    }

    #[test]
    fn capability_offers_do_not_grant_authority_until_join_is_admitted() {
        let local = sample_handshake("local", vec![sample_offer("join:jobs", "*")], Vec::new());
        let remote = sample_handshake("remote", Vec::new(), Vec::new());
        let agreement = negotiate_peers(&local, &remote, &NegotiationPolicy::default()).expect("negotiate");
        assert_eq!(agreement.decision, "pass");
        assert!(agreement.admitted_joins.is_empty());
        assert!(
            crate::preserves_rail::to_text(&agreement.receipt_value)
                .expect("receipt text")
                .contains("capability-offers-not-authority")
        );
    }

    #[test]
    fn resource_limits_are_bound_by_policy_and_peers() {
        let policy = NegotiationPolicy {
            max_inflight: 8,
            max_bytes: 1024,
            max_topics: 2,
            max_jobs: 1,
            ..NegotiationPolicy::default()
        };
        let local = sample_handshake("local", Vec::new(), Vec::new());
        let remote = sample_handshake("remote", Vec::new(), Vec::new());
        let agreement = negotiate_peers(&local, &remote, &policy).expect("negotiate");
        assert_eq!(agreement.resource_limits.max_inflight, 8);
        assert_eq!(agreement.resource_limits.max_bytes, 1024);
        assert_eq!(agreement.resource_limits.max_topics, 2);
        assert_eq!(agreement.resource_limits.max_jobs, 1);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_negotiation_is_deterministic_and_denied_join_is_safe(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let should_offer_capability = tc.draw(hegel::generators::booleans());
        let offers = if should_offer_capability {
            vec![sample_offer("join:gossip", &format!("topic:{salt}"))]
        } else {
            Vec::new()
        };
        let local = sample_handshake(&format!("local-{salt}"), offers, Vec::new());
        let remote = sample_handshake(&format!("remote-{salt}"), Vec::new(), vec![JoinRequest {
            kind: "gossip-topic".to_string(),
            target: format!("topic:{salt}"),
            required_capability: "join:gossip".to_string(),
        }]);
        let first = negotiate_peers(&local, &remote, &NegotiationPolicy::default()).expect("first negotiation");
        let second = negotiate_peers(&local, &remote, &NegotiationPolicy::default()).expect("second negotiation");
        assert_eq!(first.value, second.value);
        assert_eq!(first.receipt_value, second.receipt_value);
        if should_offer_capability {
            assert_eq!(first.decision, "pass");
            assert_eq!(first.admitted_joins.len(), 1);
        } else {
            assert_eq!(first.decision, "fail");
            assert_eq!(first.denied_joins.len(), 1);
            assert!(first.admitted_joins.is_empty());
        }
    }

    fn sample_handshake(name: &str, offers: Vec<CapabilityOffer>, joins: Vec<JoinRequest>) -> IoValue {
        handshake_value(&HandshakeValueInput {
            node_id: name,
            identity_ref: &ref_for(&format!("identity-{name}")),
            endpoint_id: &format!("iroh:{name}"),
            molten_version: "0.1.0",
            features: &sample_features(),
            requested_joins: &joins,
            capability_offers: &offers,
            resource_limits: &sample_limits(),
            policy_refs: &[],
            receipt_refs: &[],
        })
        .expect("sample handshake")
    }

    fn sample_features() -> FeatureVector {
        FeatureVector {
            runtime_versions: vec!["molten-runtime-v1".to_string()],
            registry_protocols: vec!["registry-v1".to_string(), "registry-v2".to_string()],
            schema_identities: vec!["schema-identity-v1".to_string()],
            preserves_boundaries: vec!["preserves-boundary-v1".to_string()],
            handler_profiles: vec!["native".to_string(), "wasm".to_string()],
            transports: vec!["iroh-gossip".to_string(), "iroh-blobs".to_string()],
            replay: true,
        }
    }

    fn sample_offer(capability: &str, scope: &str) -> CapabilityOffer {
        CapabilityOffer {
            capability: capability.to_string(),
            scope: scope.to_string(),
            attenuation: "scoped".to_string(),
            expires_at: Some(100),
            policy_refs: Vec::new(),
        }
    }

    fn sample_limits() -> ResourceLimits {
        ResourceLimits {
            max_inflight: 64,
            max_bytes: 1_048_576,
            max_topics: 8,
            max_jobs: 4,
        }
    }

    fn ref_for(label: &str) -> String {
        canonical_hash(&record("peer-bootstrap-test-ref", vec![string(label)])).expect("test ref")
    }
}
