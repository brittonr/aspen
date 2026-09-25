
    /// A channel pointer alone is not admission authority; release evidence, policy, provenance, source gate,
    /// authority, and resource refs admit it, and the catalog surfaces the snapshot caveat.
    fn assert_channel_admission_requires_evidence(
        root: &std::path::Path,
        channel_pointer_ref: String,
        verified_receipt_ref: String,
    ) {
        let channel_only = release_channel_admission_receipt(&ReleaseChannelAdmissionInput {
            channel_pointer_ref: channel_pointer_ref.clone(),
            release_evidence_refs: Vec::new(),
            policy_refs: Vec::new(),
            provenance_refs: Vec::new(),
            source_gate_refs: Vec::new(),
            authority_refs: Vec::new(),
            resource_refs: Vec::new(),
        })
        .expect("channel-only admission receipt");
        assert_eq!(channel_only.decision, "deny");
        assert!(channel_only
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("non-authority")));
        let admitted = release_channel_admission_receipt(&ReleaseChannelAdmissionInput {
            channel_pointer_ref,
            release_evidence_refs: vec![verified_receipt_ref],
            policy_refs: vec![test_ref("admission-policy")],
            provenance_refs: vec![test_ref("admission-provenance")],
            source_gate_refs: vec![test_ref("admission-source-gate")],
            authority_refs: vec![test_ref("admission-authority")],
            resource_refs: vec![test_ref("admission-resource")],
        })
        .expect("fully evidenced admission receipt");
        assert_eq!(admitted.decision, "pass");

        let catalog = crate::catalog::search(root, None, &crate::catalog::SearchInput {
            root_refs: Vec::new(),
            include_dependencies: true,
            include_dependents: true,
            filters: vec![crate::catalog::Filter::Text("release-snapshot-caveat:pilot-scope".to_string())],
            visibility: crate::catalog::VisibilityInput::default(),
        })
        .expect("catalog release snapshot caveat search");
        assert_eq!(catalog.items.len(), 1);
    }

    #[test]
    fn release_snapshot_verification_denies_missing_tampered_and_stale_evidence() {
        // r[verify molten.release_snapshots.validation]
        let root = temp_dir("release-snapshot-deny");
        let base = install_artifact(&root, &test_input("schema", "deny-base", &[])).expect("base artifact");
        let app = install_artifact(
            &root,
            &test_input("steel", "deny-app", std::slice::from_ref(&base.artifact_ref)),
        )
        .expect("app artifact");
        let draft = release_snapshot_draft("internal/pilot", "snapshot-deny", &[base.artifact_ref.clone(), app.artifact_ref.clone()]);
        let mut bad_input = release_snapshot_value_input(&root, &draft).expect("valid snapshot input");
        bad_input.artifact_refs = vec![app.artifact_ref.clone()];
        bad_input.stale_evidence_refs = vec![test_ref("stale-evidence")];
        let bad_payload = release_snapshot_value(&bad_input).expect("bad snapshot payload");
        let bad_install = install_artifact(&root, &ArtifactInstallInput {
            kind: RELEASE_SNAPSHOT_ARTIFACT_KIND.to_string(),
            payload: bad_payload,
            schema_refs: Vec::new(),
            dependency_refs: bad_input.artifact_refs.clone(),
            effect_manifest_ref: None,
            policy_refs: bad_input.policy_refs.clone(),
            evidence_refs: release_snapshot_install_evidence_refs(&bad_input).expect("bad evidence refs"),
            installer_ref: test_ref("bad-release-installer"),
            capability_refs: vec![test_ref("bad-release-capability")],
        })
        .expect("install malformed snapshot");
        let denied = verify_release_snapshot(&root, &ReleaseSnapshotVerifyInput {
            snapshot_ref: bad_install.artifact_ref,
            required_caveats: vec!["pilot-scope".to_string()],
        })
        .expect("verify malformed snapshot");
        assert_eq!(denied.decision, "deny");
        assert!(denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("snapshot omitted closure member")));
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale evidence")));

        assert_tampered_member_denied();
    }

    /// A snapshot whose member bytes were overwritten in the artifact index is denied as tampered.
    fn assert_tampered_member_denied() {
        let tampered_root = temp_dir("release-snapshot-tampered");
        let tampered_base = install_artifact(&tampered_root, &test_input("schema", "tamper-base", &[]))
            .expect("tampered base artifact");
        let tampered_app = install_artifact(
            &tampered_root,
            &test_input("steel", "tamper-app", std::slice::from_ref(&tampered_base.artifact_ref)),
        )
        .expect("tampered app artifact");
        let tampered_draft = release_snapshot_draft(
            "internal/pilot",
            "snapshot-tampered",
            &[tampered_base.artifact_ref.clone(), tampered_app.artifact_ref.clone()],
        );
        let tampered_snapshot = install_release_snapshot(&tampered_root, &ReleaseSnapshotInstallInput {
            snapshot: tampered_draft,
            installer_ref: test_ref("tamper-installer"),
            capability_refs: vec![test_ref("tamper-capability")],
        })
        .expect("install tampered fixture snapshot");
        let capability_root =
            CapabilityArtifactRoot::open(&tampered_root).expect("open tampered artifact capability root");
        let db = ensure_index_tables(&capability_root).expect("artifact db");
        let write_txn = db.begin_write().expect("write txn");
        {
            let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).expect("artifacts table");
            let app_bytes = canonical_bytes(&tampered_app.artifact.value).expect("app bytes");
            artifacts
                .insert(tampered_base.artifact_ref.as_str(), app_bytes.as_slice())
                .expect("tamper member bytes");
        }
        write_txn.commit().expect("commit tampered member");
        drop(db);
        let tampered = verify_release_snapshot(&tampered_root, &ReleaseSnapshotVerifyInput {
            snapshot_ref: tampered_snapshot.artifact_ref,
            required_caveats: vec!["pilot-scope".to_string()],
        })
        .expect("tampered member denial receipt");
        assert_eq!(tampered.decision, "deny");
        assert!(tampered.diagnostics.iter().any(|diagnostic| diagnostic.contains("tampered")));
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

    fn name_view_input(view_kind: &str, name: &str, scope: &str, target_ref: &str) -> ArtifactNameViewInput {
        ArtifactNameViewInput {
            view_kind: view_kind.to_string(),
            name: name.to_string(),
            scope: scope.to_string(),
            target_kind: "artifact-ref".to_string(),
            target_ref: target_ref.to_string(),
            issuer_ref: test_ref(&format!("issuer-{scope}-{name}")),
            policy_refs: vec![test_ref(&format!("policy-{scope}-{name}"))],
            evidence_refs: vec![test_ref(&format!("evidence-{scope}-{name}"))],
            capability_refs: vec![test_ref(&format!("capability-{scope}-{name}"))],
            tombstone_ref: None,
        }
    }

    fn release_snapshot_draft(namespace_scope: &str, snapshot_id: &str, artifact_refs: &[String]) -> ReleaseSnapshotDraftInput {
        ReleaseSnapshotDraftInput {
            namespace_scope: namespace_scope.to_string(),
            snapshot_id: snapshot_id.to_string(),
            artifact_refs: artifact_refs.to_vec(),
            artifact_set_ref: Some(test_ref(&format!("artifact-set-{snapshot_id}"))),
            doc_refs: vec![test_ref(&format!("doc-{snapshot_id}"))],
            transcript_refs: vec![test_ref(&format!("transcript-{snapshot_id}"))],
            expected_receipt_refs: vec![test_ref(&format!("receipt-{snapshot_id}"))],
            policy_refs: vec![test_ref(&format!("policy-{snapshot_id}"))],
            provenance_refs: vec![test_ref(&format!("provenance-{snapshot_id}"))],
            source_gate_refs: vec![test_ref(&format!("source-gate-{snapshot_id}"))],
            resource_refs: vec![test_ref(&format!("resource-{snapshot_id}"))],
            compatibility_refs: vec![test_ref(&format!("compatibility-{snapshot_id}"))],
            migration_refs: vec![test_ref(&format!("migration-{snapshot_id}"))],
            upgrade_session_refs: vec![test_ref(&format!("upgrade-{snapshot_id}"))],
            rollback_refs: vec![test_ref(&format!("rollback-{snapshot_id}"))],
            cutover_refs: vec![test_ref(&format!("cutover-{snapshot_id}"))],
            caveats: vec!["pilot-scope".to_string(), "redaction: internal-only".to_string()],
            non_claims: vec!["channel names do not grant authority, deployment, or execution".to_string()],
            redaction_profile_ref: Some(test_ref(&format!("redaction-{snapshot_id}"))),
            signature_refs: vec![test_ref(&format!("signature-{snapshot_id}"))],
            stale_evidence_refs: Vec::new(),
        }
    }

    fn identity_input<'a>(
        kind: &'a str,
        identity_domain: &'a str,
        payload_ref: &'a str,
        schema_refs: &'a [String],
        dependency_summary_refs: &'a [String],
        policy_refs: &'a [String],
        provenance_refs: &'a [String],
    ) -> ArtifactIdentityInput<'a> {
        ArtifactIdentityInput {
            kind,
            identity_domain,
            canonical_payload_ref: payload_ref,
            canonicalizer: PRESERVES_VALUE_CANONICALIZER,
            artifact_ref: None,
            schema_refs,
            dependency_summary_refs,
            effect_manifest_ref: None,
            policy_refs,
            provenance_refs,
            hash_algorithm: ARTIFACT_IDENTITY_HASH_ALGORITHM,
        }
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("artifact-test-ref", vec![string(label)])).expect("test ref")
    }
