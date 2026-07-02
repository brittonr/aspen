
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_identity_is_stable_across_names_and_changes_with_payload_kind_or_deps() {
        let root = temp_dir("artifact-identity");
        let payload = record("module", vec![string("v1")]);
        let input = ArtifactInstallInput {
            kind: "steel".to_string(),
            payload: payload.clone(),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        };
        let first = install_artifact(&root, &input).expect("install first");
        let duplicate = install_artifact(&root, &input).expect("install duplicate");
        assert_eq!(first.decision, "pass");
        assert_eq!(first.artifact_ref, duplicate.artifact_ref);
        let pointer = set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/main",
            artifact_ref: &first.artifact_ref,
            policy_refs: &input.policy_refs,
            evidence_refs: &input.evidence_refs,
        })
        .expect("set name");
        assert_eq!(pointer.artifact_ref, first.artifact_ref);
        assert_eq!(read_payload(&root, &first.artifact_ref).expect("payload"), payload);

        let changed_payload = install_artifact(&root, &ArtifactInstallInput {
            payload: record("module", vec![string("v2")]),
            ..input.clone()
        })
        .expect("changed payload");
        assert_ne!(first.artifact_ref, changed_payload.artifact_ref);
        let changed_kind = install_artifact(&root, &ArtifactInstallInput {
            kind: "wasm".to_string(),
            ..input.clone()
        })
        .expect("changed kind");
        assert_ne!(first.artifact_ref, changed_kind.artifact_ref);
        let changed_deps = install_artifact(&root, &ArtifactInstallInput {
            dependency_refs: vec![first.artifact_ref.clone()],
            ..input
        })
        .expect("changed deps");
        assert_ne!(first.artifact_ref, changed_deps.artifact_ref);
    }

    #[test]
    fn artifact_registry_rejects_malformed_refs_and_missing_materialization() {
        let root = temp_dir("artifact-ref-shape");
        let mut input = test_input("steel", "bad-ref", &[]);
        input.schema_refs = vec!["blake3:fixture".to_string()];
        let error = install_artifact(&root, &input).expect_err("short schema ref denied");
        assert!(error.to_string().contains("canonical blake3 content ref"));

        let content_payload = ArtifactPayloadRef::ContentRef {
            manifest_ref: "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            length: 128,
        };
        let artifact_error = artifact_value(ArtifactValueInput {
            kind: "doc",
            payload: &content_payload,
            schema_refs: &[test_ref("schema")],
            dependency_refs: &[],
            effect_manifest_ref: None,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect_err("uppercase content manifest ref denied");
        assert!(artifact_error.to_string().contains("canonical blake3 content ref"));

        let missing = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let missing_error = read_artifact(&root, missing).expect_err("valid-shaped missing artifact denied");
        assert!(missing_error.to_string().contains("not found"));
    }

    #[test]
    fn artifact_registry_detects_tampered_materialized_artifact_bytes() {
        let root = temp_dir("artifact-tampered-bytes");
        let first = install_artifact(&root, &test_input("steel", "first", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "second", &[])).expect("second artifact");
        assert_ne!(first.artifact_ref, second.artifact_ref);
        let db = ensure_index_tables(&root).expect("artifact db");
        let write_txn = db.begin_write().expect("write txn");
        {
            let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).expect("artifacts table");
            let second_bytes = canonical_bytes(&second.artifact.value).expect("second bytes");
            artifacts
                .insert(first.artifact_ref.as_str(), second_bytes.as_slice())
                .expect("tamper artifact bytes");
        }
        write_txn.commit().expect("commit tamper");
        drop(db);
        let error = read_artifact(&root, &first.artifact_ref).expect_err("tampered artifact bytes denied");
        assert!(error.to_string().contains("artifact registry content hash mismatch"), "unexpected error: {error}");
    }

    #[test]
    fn artifact_names_do_not_substitute_for_content_identity() {
        let root = temp_dir("artifact-name-not-identity");
        let first = install_artifact(&root, &test_input("steel", "first-name", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "second-name", &[])).expect("second artifact");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/current",
            artifact_ref: &first.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("first name pointer");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: "app/current",
            artifact_ref: &second.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("second name pointer");
        assert_eq!(
            read_payload(&root, &first.artifact_ref).expect("first payload"),
            record("payload", vec![string("first-name")])
        );
        assert_eq!(
            read_payload(&root, &second.artifact_ref).expect("second payload"),
            record("payload", vec![string("second-name")])
        );
        assert_ne!(first.artifact_ref, second.artifact_ref);
    }

    #[test]
    fn dependency_closure_impact_missing_dependencies_and_rebuild_work() {
        let root = temp_dir("artifact-deps");
        let base = install_artifact(&root, &test_input("schema", "base", &[])).expect("base");
        let dependent =
            install_artifact(&root, &test_input("steel", "dependent", std::slice::from_ref(&base.artifact_ref)))
                .expect("dependent");
        let closure = dependency_closure(&root, std::slice::from_ref(&dependent.artifact_ref)).expect("closure");
        assert_eq!(closure.missing_refs, Vec::<String>::new());
        assert!(closure.closure_refs.contains(&base.artifact_ref));
        assert!(closure.closure_refs.contains(&dependent.artifact_ref));
        let impact = impact(&root, std::slice::from_ref(&base.artifact_ref)).expect("impact");
        assert!(impact.impacted_refs.contains(&base.artifact_ref));
        assert!(impact.impacted_refs.contains(&dependent.artifact_ref));
        let missing = test_ref("missing-dep");
        let denied =
            install_artifact(&root, &test_input("steel", "bad", std::slice::from_ref(&missing))).expect("denied");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.missing_dependencies, vec![missing]);
        let rebuild = rebuild_index(&root).expect("rebuild");
        assert!(rebuild.artifacts >= 2);
    }

    #[test]
    fn large_payloads_use_chunk_refs_and_cleanup_diagnostics_see_pointers() {
        let root = temp_dir("artifact-large");
        let large = IoValue::new("x".repeat(INLINE_PAYLOAD_LIMIT + 512));
        let installed = install_artifact(&root, &ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: large.clone(),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install large");
        assert!(matches!(installed.artifact.payload, ArtifactPayloadRef::ContentRef { .. }));
        assert_eq!(read_payload(&root, &installed.artifact_ref).expect("read payload"), large);
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "alias",
            name: "docs/current",
            artifact_ref: &installed.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("alias");
        let diagnostics = reference_diagnostics(&root, &installed.artifact_ref).expect("diagnostics");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.contains("pointer")));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_artifact_closure_reverse_edges_and_no_name_identity(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let root = temp_dir("artifact-hegel");
        let base = install_artifact(&root, &test_input("schema", &format!("base-{salt}"), &[])).expect("base");
        let middle = install_artifact(
            &root,
            &test_input("steel", &format!("middle-{salt}"), std::slice::from_ref(&base.artifact_ref)),
        )
        .expect("middle");
        let leaf = install_artifact(
            &root,
            &test_input("transcript", &format!("leaf-{salt}"), std::slice::from_ref(&middle.artifact_ref)),
        )
        .expect("leaf");
        let closure_one = dependency_closure(&root, std::slice::from_ref(&leaf.artifact_ref)).expect("closure one");
        let closure_two = dependency_closure(&root, std::slice::from_ref(&leaf.artifact_ref)).expect("closure two");
        assert_eq!(closure_one.closure_hash, closure_two.closure_hash);
        assert!(closure_one.closure_refs.contains(&base.artifact_ref));
        let impact_base = impact_refs(&root, std::slice::from_ref(&base.artifact_ref)).expect("impact base");
        assert!(impact_base.contains(&middle.artifact_ref));
        assert!(impact_base.contains(&leaf.artifact_ref));
        let before_name = leaf.artifact_ref.clone();
        let pointer_name = format!("app/{salt}");
        set_name_pointer(&root, &SetNamePointerInput {
            pointer_kind: "name",
            name: &pointer_name,
            artifact_ref: &leaf.artifact_ref,
            policy_refs: &[test_ref("policy")],
            evidence_refs: &[test_ref("evidence")],
        })
        .expect("set name");
        let after_name = read_artifact(&root, &leaf.artifact_ref).expect("read after name").artifact_ref;
        assert_eq!(before_name, after_name);
    }

    fn test_input(kind: &str, label: &str, dependency_refs: &[String]) -> ArtifactInstallInput {
        ArtifactInstallInput {
            kind: kind.to_string(),
            payload: record("payload", vec![string(label)]),
            schema_refs: vec![test_ref(&format!("schema-{label}"))],
            dependency_refs: dependency_refs.to_vec(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref(&format!("policy-{label}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{label}"))],
            installer_ref: test_ref(&format!("installer-{label}")),
            capability_refs: vec![test_ref(&format!("capability-{label}"))],
        }
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("artifact-test-ref", vec![string(label)])).expect("test ref")
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
