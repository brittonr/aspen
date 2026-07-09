
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
        let identity_receipt = parse_artifact_identity_receipt(&first.identity_receipt_value).expect("identity receipt");
        assert_eq!(first.decision, "pass");
        assert_eq!(first.identity_receipt_ref, identity_receipt.receipt_ref);
        assert_eq!(identity_receipt.decision, "pass");
        assert_eq!(identity_receipt.artifact_ref.as_deref(), Some(first.artifact_ref.as_str()));
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
    fn artifact_identity_receipts_are_stable_and_domain_separated() {
        // r[verify molten.artifacts.canonical_identity_validation]
        let payload_ref = test_ref("canonical-payload");
        let schema_refs = vec![test_ref("schema")];
        let dependency_summary_refs = vec![test_ref("dependency-summary")];
        let policy_refs = vec![test_ref("policy")];
        let provenance_refs = vec![test_ref("provenance")];
        let schema_domain = domain_for_kind("schema");
        let schema_input = identity_input(
            "schema",
            &schema_domain,
            &payload_ref,
            &schema_refs,
            &dependency_summary_refs,
            &policy_refs,
            &provenance_refs,
        );
        let first = artifact_identity_receipt(&schema_input).expect("first identity receipt");
        let repeated = artifact_identity_receipt(&schema_input).expect("repeated identity receipt");
        let policy_domain = domain_for_kind("policy");
        let policy_input = ArtifactIdentityInput {
            kind: "policy",
            identity_domain: &policy_domain,
            ..schema_input
        };
        let different_domain = artifact_identity_receipt(&policy_input).expect("policy identity receipt");

        assert_eq!(first.decision, "pass");
        assert_eq!(first.artifact_ref, repeated.artifact_ref);
        assert_eq!(first.receipt_ref, repeated.receipt_ref);
        assert_ne!(first.artifact_ref, different_domain.artifact_ref);
        assert_eq!(parse_artifact_identity_receipt(&first.value).expect("parse").decision, "pass");
    }

    #[test]
    fn artifact_identity_receipts_deny_noncanonical_or_unsupported_identity() {
        // r[verify molten.artifacts.canonical_identity_validation]
        let payload_ref = test_ref("canonical-payload");
        let schema_refs = vec![test_ref("schema")];
        let dependency_summary_refs = Vec::new();
        let policy_refs = vec![test_ref("policy")];
        let provenance_refs = vec![test_ref("provenance")];
        let steel_domain = domain_for_kind("steel");
        let base = identity_input(
            "steel",
            &steel_domain,
            &payload_ref,
            &schema_refs,
            &dependency_summary_refs,
            &policy_refs,
            &provenance_refs,
        );
        let missing_payload = artifact_identity_receipt(&ArtifactIdentityInput {
            canonical_payload_ref: "",
            ..base
        })
        .expect("missing payload receipt");
        let wrong_domain = artifact_identity_receipt(&ArtifactIdentityInput {
            identity_domain: "molten.artifacts.domain.v1:policy",
            ..base
        })
        .expect("wrong domain receipt");
        let raw_source = artifact_identity_receipt(&ArtifactIdentityInput {
            canonicalizer: RAW_SOURCE_CANONICALIZER,
            ..base
        })
        .expect("raw source receipt");
        let unsupported_hash = artifact_identity_receipt(&ArtifactIdentityInput {
            hash_algorithm: "sha256",
            ..base
        })
        .expect("unsupported hash receipt");
        let unknown_domain = domain_for_kind("unknown-kind");
        let unsupported_kind = artifact_identity_receipt(&ArtifactIdentityInput {
            kind: "unknown-kind",
            identity_domain: &unknown_domain,
            ..base
        })
        .expect("unsupported kind receipt");

        assert_eq!(missing_payload.decision, "deny");
        assert_eq!(wrong_domain.decision, "deny");
        assert_eq!(raw_source.decision, "deny");
        assert_eq!(unsupported_hash.decision, "deny");
        assert_eq!(unsupported_kind.decision, "deny");
        assert!(missing_payload.artifact_ref.is_none());
        assert!(unsupported_hash
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("requires blake3")));
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
    fn name_views_resolve_exact_refs_and_do_not_grant_authority() {
        // r[verify molten.artifacts.name_view_validation]
        let root = temp_dir("artifact-name-views");
        let first = install_artifact(&root, &test_input("steel", "named-first", &[])).expect("first artifact");
        let second = install_artifact(&root, &test_input("steel", "named-second", &[])).expect("second artifact");
        let first_input = name_view_input("name", "policy/main", "project", &first.artifact_ref);
        let first_view = set_name_view(&root, &first_input).expect("first name view");
        let second_view = set_name_view(&root, &ArtifactNameViewInput {
            target_ref: second.artifact_ref.clone(),
            ..first_input.clone()
        })
        .expect("second name view");
        assert_eq!(read_artifact(&root, &first.artifact_ref).expect("first still addressable").artifact_ref, first.artifact_ref);
        assert_eq!(read_artifact(&root, &second.artifact_ref).expect("second addressable").artifact_ref, second.artifact_ref);
        assert_ne!(first_view.view.view_ref, second_view.view.view_ref);
        let unauthorized = set_name_view(&root, &ArtifactNameViewInput {
            capability_refs: Vec::new(),
            ..name_view_input("name", "policy/denied", "project", &first.artifact_ref)
        })
        .expect_err("unauthorized name view denies");
        assert!(unauthorized.to_string().contains("capability refs"));

        let peer_view = parse_name_view_value(
            &name_view_value(&name_view_input("name", "policy/main", "peer", &first.artifact_ref), None)
                .expect("peer view value"),
        )
        .expect("peer view");
        let project_view = second_view.view.clone();
        let scoped = resolve_name_view(&ArtifactNameResolutionInput {
            view_kind: "name".to_string(),
            name: "policy/main".to_string(),
            scope: Some("project".to_string()),
            candidate_views: vec![peer_view.clone(), project_view.clone()],
            stale_view_refs: Vec::new(),
            normative_use: true,
        })
        .expect("scoped resolution");
        assert_eq!(scoped.decision, "pass");
        assert_eq!(scoped.resolved_ref.as_deref(), Some(second.artifact_ref.as_str()));
        let ambiguous = resolve_name_view(&ArtifactNameResolutionInput {
            view_kind: "name".to_string(),
            name: "policy/main".to_string(),
            scope: None,
            candidate_views: vec![peer_view.clone(), project_view.clone()],
            stale_view_refs: Vec::new(),
            normative_use: true,
        })
        .expect("ambiguous resolution receipt");
        assert_eq!(ambiguous.decision, "deny");
        assert!(ambiguous.diagnostics.iter().any(|diagnostic| diagnostic.contains("ambiguous")));
        let stale = resolve_name_view(&ArtifactNameResolutionInput {
            view_kind: "name".to_string(),
            name: "policy/main".to_string(),
            scope: Some("peer".to_string()),
            candidate_views: vec![peer_view.clone()],
            stale_view_refs: vec![peer_view.view_ref],
            normative_use: true,
        })
        .expect("stale resolution receipt");
        assert_eq!(stale.decision, "deny");
        assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));

        let name_only = name_view_use_receipt(&ArtifactNameUseInput {
            operation: "remote-execution-admission".to_string(),
            name: Some("trusted/release".to_string()),
            exact_artifact_ref: None,
            resolution_receipt_ref: None,
            policy_refs: Vec::new(),
            provenance_refs: Vec::new(),
            capability_refs: Vec::new(),
        })
        .expect("name-only use receipt");
        assert_eq!(name_only.decision, "deny");
        assert!(name_only
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("name-only use denies")));
        let admitted = name_view_use_receipt(&ArtifactNameUseInput {
            operation: "remote-execution-admission".to_string(),
            name: Some("policy/main".to_string()),
            exact_artifact_ref: Some(second.artifact_ref),
            resolution_receipt_ref: Some(scoped.resolution_ref),
            policy_refs: vec![test_ref("name-use-policy")],
            provenance_refs: vec![test_ref("name-use-provenance")],
            capability_refs: vec![test_ref("name-use-capability")],
        })
        .expect("admitted exact-ref use receipt");
        assert_eq!(admitted.decision, "pass");
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
    fn dependency_edges_index_digest_and_impact_query_receipts_are_deterministic() {
        // r[verify molten.artifacts.dependency_index_validation]
        let root = temp_dir("artifact-dependency-edges");
        let base = install_artifact(&root, &test_input("schema", "edge-base", &[])).expect("base");
        let schema_ref = test_ref("edge-schema");
        let effect_ref = test_ref("edge-effect");
        let policy_ref = test_ref("edge-policy");
        let evidence_ref = test_ref("edge-evidence");
        let dependent = install_artifact(&root, &ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: record("payload", vec![string("edge-dependent")]),
            schema_refs: vec![schema_ref.clone()],
            dependency_refs: vec![base.artifact_ref.clone()],
            effect_manifest_ref: Some(effect_ref.clone()),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![evidence_ref.clone()],
            installer_ref: test_ref("edge-installer"),
            capability_refs: vec![test_ref("edge-capability")],
        })
        .expect("dependent");
        let edges = list_dependency_edges(&root).expect("edges");
        let digest = dependency_index_digest(&edges).expect("digest");
        let repeated_digest = dependency_index_digest(&list_dependency_edges(&root).expect("repeated edges"))
            .expect("repeated digest");
        assert_eq!(digest, repeated_digest);
        assert!(edges.iter().any(|edge| edge.source_ref == dependent.artifact_ref
            && edge.target_ref == base.artifact_ref
            && edge.target_kind == "artifact"));
        assert!(edges.iter().any(|edge| edge.target_ref == schema_ref && edge.target_kind == "schema"));
        assert!(edges.iter().any(|edge| edge.target_ref == effect_ref && edge.target_kind == "effect"));
        assert!(edges.iter().any(|edge| edge.target_ref == policy_ref && edge.target_kind == "policy"));
        assert!(edges.iter().any(|edge| edge.target_ref == evidence_ref && edge.target_kind == "evidence"));

        let query = impact_query(&root, &ArtifactImpactQueryInput {
            subject_ref: base.artifact_ref.clone(),
            relation_filters: vec!["imports".to_string()],
            include_transitive: true,
            hidden_refs: Vec::new(),
        })
        .expect("impact query");
        assert_eq!(query.decision, "pass");
        assert!(query.direct_dependents.contains(&dependent.artifact_ref));
        assert!(query.transitive_dependents.contains(&dependent.artifact_ref));
        assert!(query.receipt_value.collect_simple_record("artifact-receipt-v1", Some(8)).is_some());

        let redacted = impact_query(&root, &ArtifactImpactQueryInput {
            subject_ref: base.artifact_ref,
            relation_filters: vec!["imports".to_string()],
            include_transitive: false,
            hidden_refs: vec![dependent.artifact_ref.clone()],
        })
        .expect("redacted impact query");
        assert!(!redacted.direct_dependents.contains(&dependent.artifact_ref));
        assert_eq!(redacted.redacted_refs, vec![dependent.artifact_ref]);
        assert!(redacted.diagnostics.iter().any(|diagnostic| diagnostic.contains("redacted")));
    }

    #[test]
    fn dependency_edge_normalization_deduplicates_and_cycle_traversal_terminates() {
        // r[verify molten.artifacts.dependency_index_validation]
        let left = test_ref("cycle-left");
        let right = test_ref("cycle-right");
        let evidence = vec![test_ref("cycle-evidence")];
        let left_to_right = dependency_edge(&left, &right, "artifact", "imports", true, "cycle", &evidence)
            .expect("left edge");
        let right_to_left = dependency_edge(&right, &left, "artifact", "imports", true, "cycle", &evidence)
            .expect("right edge");
        let normalized = normalize_dependency_edges(&[
            left_to_right.clone(),
            left_to_right.clone(),
            right_to_left.clone(),
        ])
        .expect("normalize");
        assert_eq!(normalized.edges.len(), 2);
        assert_eq!(normalized.duplicate_refs, vec![left_to_right.edge_ref.clone()]);
        let dependents = transitive_dependents_from_edges(&normalized.edges, &left, &["imports".to_string()])
            .expect("cycle traversal");
        assert!(dependents.contains(&left));
        assert!(dependents.contains(&right));
        let digest = dependency_index_digest(&normalized.edges).expect("digest");
        let duplicate_digest = dependency_index_digest(&[
            right_to_left,
            left_to_right.clone(),
            left_to_right,
        ])
        .expect("duplicate digest");
        assert_eq!(digest, duplicate_digest);
    }

    #[test]
    fn release_snapshots_verify_channels_are_non_authority_and_catalog_surfaces_caveats() {
        // r[verify molten.release_snapshots.namespace_snapshot_artifacts]
        // r[verify molten.release_snapshots.closure_integrity]
        // r[verify molten.release_snapshots.channel_view_non_authority]
        // r[verify molten.release_snapshots.evidence_caveats]
        let root = temp_dir("release-snapshot-pass");
        let base = install_artifact(&root, &test_input("schema", "release-base", &[])).expect("base artifact");
        let app = install_artifact(
            &root,
            &test_input("steel", "release-app", std::slice::from_ref(&base.artifact_ref)),
        )
        .expect("app artifact");
        let draft = release_snapshot_draft("internal/pilot", "snapshot-2026-07-09", &[base.artifact_ref, app.artifact_ref]);
        let installed = install_release_snapshot(&root, &ReleaseSnapshotInstallInput {
            snapshot: draft.clone(),
            installer_ref: test_ref("release-installer"),
            capability_refs: vec![test_ref("release-install-capability")],
        })
        .expect("install release snapshot");
        let verified = verify_release_snapshot(&root, &ReleaseSnapshotVerifyInput {
            snapshot_ref: installed.artifact_ref.clone(),
            required_caveats: vec!["pilot-scope".to_string()],
        })
        .expect("verify release snapshot");
        assert_eq!(verified.decision, "pass");
        let hidden_caveat = verify_release_snapshot(&root, &ReleaseSnapshotVerifyInput {
            snapshot_ref: installed.artifact_ref.clone(),
            required_caveats: vec!["missing-promotion-caveat".to_string()],
        })
        .expect("verify hidden caveat denial");
        assert_eq!(hidden_caveat.decision, "deny");
        assert!(hidden_caveat
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("required caveat not rendered")));

        let channel = set_release_channel(&root, &ReleaseChannelUpdateInput {
            channel: "release/stable".to_string(),
            snapshot_ref: installed.artifact_ref.clone(),
            policy_refs: vec![test_ref("release-channel-policy")],
            capability_refs: vec![test_ref("release-channel-capability")],
            evidence_refs: vec![verified.receipt_ref.clone()],
        })
        .expect("channel update");
        let unauthorized = set_release_channel(&root, &ReleaseChannelUpdateInput {
            channel: "release/stable".to_string(),
            snapshot_ref: installed.artifact_ref.clone(),
            policy_refs: vec![test_ref("release-channel-policy")],
            capability_refs: Vec::new(),
            evidence_refs: vec![verified.receipt_ref.clone()],
        })
        .expect_err("channel update without capability denies");
        assert!(unauthorized.to_string().contains("capability refs"));

        let channel_only = release_channel_admission_receipt(&ReleaseChannelAdmissionInput {
            channel_pointer_ref: channel.pointer.pointer_ref.clone(),
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
            channel_pointer_ref: channel.pointer.pointer_ref,
            release_evidence_refs: vec![verified.receipt_ref],
            policy_refs: vec![test_ref("admission-policy")],
            provenance_refs: vec![test_ref("admission-provenance")],
            source_gate_refs: vec![test_ref("admission-source-gate")],
            authority_refs: vec![test_ref("admission-authority")],
            resource_refs: vec![test_ref("admission-resource")],
        })
        .expect("fully evidenced admission receipt");
        assert_eq!(admitted.decision, "pass");

        let catalog = crate::catalog::search(&root, None, &crate::catalog::SearchInput {
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
        let db = ensure_index_tables(&tampered_root).expect("artifact db");
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
