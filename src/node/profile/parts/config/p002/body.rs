
#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn required_adapters() -> Vec<NodeAdapterBinding> {
        crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS
            .iter()
            .map(|adapter| {
                crate::node_runtime::node_adapter_binding(adapter, &local_ref(&format!("adapter-{adapter}"))).unwrap()
            })
            .collect()
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn profile() -> CheckedNodeProfile {
        CheckedNodeProfile {
            profile_ref: local_ref("node-profile"),
            actual_profile_ref: Some(local_ref("node-profile")),
            source_kind: SOURCE_KIND_CHECKED_EXPORT.to_string(),
            tier: TIER_PILOT.to_string(),
            schema_id: "molten.prod-ops.deployment-profile.v1".to_string(),
            schema_version: "1".to_string(),
            source_language: "nickel".to_string(),
            profile_identity: "pilot-node".to_string(),
            state_root_ref: local_ref("state-root"),
            adapters: required_adapters(),
            policy_refs: refs("policy"),
            capability_refs: refs("capability"),
            resource_refs: refs("resource"),
            effect_profile_refs: refs("effects"),
            overrideable_fields: vec![OVERRIDE_STATE_ROOT_REF.to_string()],
        }
    }

    fn input() -> ProfileBackedConfigInput {
        ProfileBackedConfigInput {
            identity_ref: local_ref("identity"),
            profile: profile(),
            overrides: NodeProfileOverrides::default(),
        }
    }

    // r[verify molten.node_runtime.profile_backed_config]
    // r[verify molten.node_runtime.profile_startup_receipt_binding]
    #[test]
    fn profile_backed_config_builds_canonical_node_config_and_metadata_refs() {
        let resolved = resolve_profile_backed_config(&input()).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_PASS);
        assert_eq!(resolved.diagnostics, Vec::<String>::new());
        assert_eq!(resolved.profile_metadata_refs.len(), 2);
        let config = crate::node_runtime::parse_node_config(&resolved.config_value).expect("config parse");
        assert_eq!(config.config_ref, resolved.config_ref);
        assert_eq!(config.policy_refs, refs("policy"));
        assert!(
            crate::preserves_rail::to_text(&resolved.resolution_value)
                .expect("resolution text")
                .contains("node-profile-config-resolution-v1")
        );
    }

    // r[verify molten.node_runtime.profile_override_policy]
    #[test]
    fn development_profile_records_allowed_override() {
        let mut with_override = input();
        with_override.profile.tier = TIER_DEVELOPMENT.to_string();
        let override_ref = local_ref("override-state-root");
        with_override.overrides.state_root_ref = Some(override_ref.clone());
        let resolved = resolve_profile_backed_config(&with_override).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_PASS);
        assert!(
            resolved
                .accepted_overrides
                .iter()
                .any(|item| item == &format!("accepted-override:{OVERRIDE_STATE_ROOT_REF}={override_ref}"))
        );
        let config = crate::node_runtime::parse_node_config(&resolved.config_value).expect("config parse");
        assert_eq!(config.state_root_ref, override_ref);
    }

    #[test]
    fn release_profile_denies_invariant_weakening_override() {
        let mut with_override = input();
        with_override.profile.tier = TIER_RELEASE.to_string();
        with_override.overrides.state_root_ref = Some(local_ref("override-state-root"));
        let resolved = resolve_profile_backed_config(&with_override).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_DENY);
        assert!(
            resolved
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == &format!("denied-profile-override:{OVERRIDE_STATE_ROOT_REF}"))
        );
    }

    #[test]
    fn profile_resolution_denies_tampered_ref_runtime_nickel_and_unsupported_adapter() {
        let mut bad = input();
        bad.profile.actual_profile_ref = Some(local_ref("tampered-node-profile"));
        bad.profile.source_kind = SOURCE_KIND_NICKEL_SOURCE.to_string();
        bad.profile.adapters.push(
            crate::node_runtime::node_adapter_binding("unsupported-adapter", &local_ref("unsupported-adapter"))
                .expect("unsupported shape ok"),
        );
        let resolved = resolve_profile_backed_config(&bad).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_DENY);
        assert!(resolved.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("profile-ref-mismatch")));
        assert!(
            resolved
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "runtime-nickel-evaluation-denied:startup-consumes-checked-exports")
        );
        assert!(
            resolved
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "unsupported-node-adapter-profile:unsupported-adapter")
        );
    }

    // r[verify molten.node_runtime.local_default_config_caveat]
    #[test]
    fn local_default_config_is_fixture_scoped_and_not_release_evidence() {
        let local = resolve_local_default_config(&LocalDefaultConfigInput {
            identity_ref: local_ref("identity"),
            state_root_ref: local_ref("state-root"),
            adapters: required_adapters(),
            policy_refs: refs("policy"),
            capability_refs: refs("capability"),
            resource_refs: refs("resource"),
            effect_profile_refs: refs("effects"),
        })
        .expect("local resolution");
        assert_eq!(local.decision, DECISION_PASS);
        assert!(local.diagnostics.iter().any(|diagnostic| diagnostic == LOCAL_FIXTURE_CAVEAT));
        assert!(
            crate::preserves_rail::to_text(&local.resolution_value)
                .expect("local resolution text")
                .contains(LOCAL_FIXTURE_CAVEAT)
        );
    }
}
