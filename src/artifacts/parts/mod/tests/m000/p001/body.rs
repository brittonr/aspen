
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

        assert_name_use_requires_exact_ref(second.artifact_ref, scoped.resolution_ref);
    }

    /// Name-only use is denied; use bound to the exact ref, its resolution receipt, and policy evidence passes.
    fn assert_name_use_requires_exact_ref(exact_artifact_ref: String, resolution_ref: String) {
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
            exact_artifact_ref: Some(exact_artifact_ref),
            resolution_receipt_ref: Some(resolution_ref),
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
        let left_to_right = dependency_edge(DependencyEdgeInput {
            source_ref: &left,
            target_ref: &right,
            target_kind: "artifact",
            relation: "imports",
            required: true,
            scope: "cycle",
            evidence_refs: &evidence,
        })
            .expect("left edge");
        let right_to_left = dependency_edge(DependencyEdgeInput {
            source_ref: &right,
            target_ref: &left,
            target_kind: "artifact",
            relation: "imports",
            required: true,
            scope: "cycle",
            evidence_refs: &evidence,
        })
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

        assert_channel_admission_requires_evidence(&root, channel.pointer.pointer_ref, verified.receipt_ref);
    }
