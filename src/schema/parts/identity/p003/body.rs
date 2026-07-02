
#[cfg(test)]
mod tests {
    use super::*;

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    #[test]
    fn structural_fingerprint_ignores_record_field_order() {
        let first = parse_text(
            r#"<shape "record" "profile" [<shape "field" "name" <shape "string">> <shape "field" "age" <shape "u64">>]>"#,
        )
        .expect("first shape");
        let second = parse_text(
            r#"<shape "record" "profile" [<shape "field" "age" <shape "u64">> <shape "field" "name" <shape "string">>]>"#,
        )
        .expect("second shape");
        let (_, _, first_fp) = structural_fingerprint(&first).expect("first fp");
        let (_, _, second_fp) = structural_fingerprint(&second).expect("second fp");
        assert_eq!(first_fp, second_fp);
    }

    #[test]
    fn unique_schemas_need_exact_ref_alias_or_migration_even_with_same_shape() {
        let shape = parse_text(r#"<shape "record" "id" [<shape "field" "value" <shape "string">>]>"#).expect("shape");
        let expected = identity(MODE_UNIQUE, "user-id", &shape, None);
        let actual = identity(MODE_UNIQUE, "order-id", &shape, None);
        let mismatch = compatibility_decision_value(&CompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("mismatch");
        assert_eq!(
            parse_compatibility(&mismatch).expect("parse mismatch").decision,
            DECISION_MISMATCH_REQUIRES_MIGRATION
        );
        let alias = parse_alias(
            &alias_value(&AliasInput {
                from_schema_ref: actual.schema_ref.clone(),
                to_schema_ref: expected.schema_ref.clone(),
                scope: "storage".to_string(),
                policy_refs: vec![test_ref("alias-policy")],
                evidence_refs: vec![test_ref("alias-evidence")],
            })
            .expect("alias value"),
        )
        .expect("alias");
        let admitted = compatibility_decision_value(&CompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: Some(alias.clone()),
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("alias admitted");
        assert_eq!(parse_compatibility(&admitted).expect("parse admitted").decision, DECISION_ADMITTED_ALIAS);
        let migration = compatibility_decision_value(&CompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: None,
            migration_ref: Some(test_ref("migration-recipe")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("migration available");
        assert_eq!(parse_compatibility(&migration).expect("parse migration").decision, DECISION_MIGRATION_AVAILABLE);
        let reverse = compatibility_decision_value(&CompatibilityInput {
            expected: actual,
            actual: expected,
            alias: Some(alias),
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("alias reverse");
        assert_eq!(
            parse_compatibility(&reverse).expect("parse reverse").decision,
            DECISION_MISMATCH_REQUIRES_MIGRATION
        );
    }

    #[test]
    fn structural_and_branded_compatibility_are_explicit() {
        let shape = parse_text(r#"<shape "sequence" <shape "string">>"#).expect("shape");
        let left = identity(MODE_STRUCTURAL, "left", &shape, None);
        let right = identity(MODE_STRUCTURAL, "right", &shape, None);
        let structural = compatibility_decision_value(&CompatibilityInput {
            expected: left,
            actual: right,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("structural compat");
        assert_eq!(parse_compatibility(&structural).expect("parse structural").decision, DECISION_STRUCTURAL_MATCH);

        let brand = test_ref("brand");
        let branded_left = identity(MODE_BRANDED_STRUCTURAL, "brand-left", &shape, Some(brand.clone()));
        let branded_right = identity(MODE_BRANDED_STRUCTURAL, "brand-right", &shape, Some(brand));
        let branded = compatibility_decision_value(&CompatibilityInput {
            expected: branded_left,
            actual: branded_right,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            deny_by_policy: false,
        })
        .expect("brand compat");
        assert_eq!(parse_compatibility(&branded).expect("parse brand").decision, DECISION_BRAND_MATCH);
    }

    #[test]
    fn compatibility_helpers_cover_storage_protocol_effect_and_policy_contracts() {
        let shape =
            parse_text(r#"<shape "record" "event" [<shape "field" "payload" <shape "bytes">>]>"#).expect("shape");
        let expected = identity(MODE_UNIQUE, "expected-event", &shape, None);
        let actual = identity(MODE_UNIQUE, "actual-event", &shape, None);
        for (scope, admits) in [
            ("storage", compatibility_admits_storage as fn(&IoValue, &str, &str) -> Result<bool>),
            ("protocol", compatibility_admits_protocol_payload),
            ("effect", compatibility_admits_effect_schema),
            ("policy", compatibility_admits_policy_contract_schema),
        ] {
            let alias = parse_alias(
                &alias_value(&AliasInput {
                    from_schema_ref: actual.schema_ref.clone(),
                    to_schema_ref: expected.schema_ref.clone(),
                    scope: scope.to_string(),
                    policy_refs: vec![test_ref(&format!("{scope}-policy"))],
                    evidence_refs: vec![test_ref(&format!("{scope}-evidence"))],
                })
                .expect("alias value"),
            )
            .expect("alias");
            let compatibility = compatibility_decision_value(&CompatibilityInput {
                expected: expected.clone(),
                actual: actual.clone(),
                alias: Some(alias),
                migration_ref: None,
                policy_refs: vec![test_ref("policy")],
                evidence_refs: vec![test_ref("evidence")],
                deny_by_policy: false,
            })
            .expect("compatibility");
            assert_eq!(
                parse_compatibility(&compatibility).expect("parse compatibility").decision,
                DECISION_ADMITTED_ALIAS
            );
            assert!(admits(&compatibility, &expected.schema_ref, &actual.schema_ref).expect("admitted"));
            let receipt = compatibility_receipt_value(scope, &compatibility).expect("receipt");
            assert_eq!(parse_compatibility_receipt(&receipt).expect("parse receipt").decision, "pass");
        }
    }

    #[test]
    fn registry_search_finds_matching_fingerprints() {
        let root = temp_dir("schema-registry");
        let shape = parse_text(r#"<shape "string">"#).expect("shape");
        let identity = identity(MODE_STRUCTURAL, "search", &shape, None);
        let installed = crate::artifacts::install_artifact(&root, &crate::artifacts::ArtifactInstallInput {
            kind: "schema-identity".to_string(),
            payload: identity.value.clone(),
            schema_refs: vec![identity.schema_ref.clone()],
            dependency_refs: vec![identity.schema_ref.clone()],
            effect_manifest_ref: None,
            policy_refs: identity.policy_refs.clone(),
            evidence_refs: identity.evidence_refs.clone(),
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install identity");
        assert_eq!(installed.decision, "deny", "identity depends on schema ref not installed yet");
        let schema_artifact = crate::artifacts::install_artifact(&root, &crate::artifacts::ArtifactInstallInput {
            kind: "schema".to_string(),
            payload: record("schema-source", vec![string("search")]),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install schema");
        let identity = identity_with_schema(MODE_STRUCTURAL, &schema_artifact.artifact_ref, &shape, None);
        let installed = crate::artifacts::install_artifact(&root, &crate::artifacts::ArtifactInstallInput {
            kind: "schema-identity".to_string(),
            payload: identity.value.clone(),
            schema_refs: vec![identity.schema_ref.clone()],
            dependency_refs: vec![identity.schema_ref.clone()],
            effect_manifest_ref: None,
            policy_refs: identity.policy_refs.clone(),
            evidence_refs: identity.evidence_refs.clone(),
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install identity");
        assert_eq!(installed.decision, "pass");
        let matches = search_registry_by_fingerprint(&root, &identity.structural_fingerprint).expect("search");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].identity_ref, identity.identity_ref);
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_fingerprint_and_compatibility_invariants(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let shape = record("shape", vec![
            string("record"),
            string(format!("profile-{salt}")),
            sequence(vec![
                record("shape", vec![string("field"), string("name"), record("shape", vec![string("string")])]),
                record("shape", vec![string("field"), string("age"), record("shape", vec![string("u64")])]),
            ]),
        ]);
        let (_, _, first_fp) = structural_fingerprint(&shape).expect("first fp");
        let (_, _, second_fp) = structural_fingerprint(&shape).expect("second fp");
        assert_eq!(first_fp, second_fp);
        let expected = identity(MODE_STRUCTURAL, &format!("expected-{salt}"), &shape, None);
        let actual = identity(MODE_STRUCTURAL, &format!("actual-{salt}"), &shape, None);
        let compatibility = compatibility_decision_value(&CompatibilityInput {
            expected: expected.clone(),
            actual: actual.clone(),
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            deny_by_policy: false,
        })
        .expect("compatibility");
        let parsed = parse_compatibility(&compatibility).expect("parse compatibility");
        assert_eq!(parsed.decision, DECISION_STRUCTURAL_MATCH);
        assert!(
            compatibility_admits_storage(&compatibility, &expected.schema_ref, &actual.schema_ref).expect("admits")
        );
        let denied = compatibility_decision_value(&CompatibilityInput {
            expected,
            actual,
            alias: None,
            migration_ref: None,
            policy_refs: vec![test_ref(&format!("policy-{salt}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{salt}"))],
            deny_by_policy: true,
        })
        .expect("denied compatibility");
        assert_eq!(parse_compatibility(&denied).expect("parse denied").decision, DECISION_DENIED_BY_POLICY);
    }

    fn identity(mode: &str, label: &str, shape: &IoValue, brand_ref: Option<String>) -> Identity {
        identity_with_schema(mode, &test_ref(&format!("schema-{label}")), shape, brand_ref)
    }

    fn identity_with_schema(mode: &str, schema_ref: &str, shape: &IoValue, brand_ref: Option<String>) -> Identity {
        let value = identity_value(&IdentityInput {
            mode: mode.to_string(),
            schema_ref: schema_ref.to_string(),
            shape: shape.clone(),
            brand_ref,
            metadata_refs: vec![test_ref("metadata")],
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
        })
        .expect("identity value");
        parse_identity(&value).expect("identity")
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("schema-identity-test-ref", vec![string(label)])).expect("test ref")
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
