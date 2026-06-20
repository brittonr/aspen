    #[test]
    fn cli_rewrite_commands_work() {
        let dir = temp_dir("rewrite-cli");
        let registry = dir.join("registry");
        install_rewrite_doc(&dir, &registry);
        let plan_out = preview_rewrite(&dir, &registry);
        apply_rewrite(&dir, &registry, plan_out);
        assert_rewritten_doc(registry);
    }

    fn install_rewrite_doc(dir: &Path, registry: &Path) {
        let payload = dir.join("doc.preserves");
        let artifact_out = dir.join("doc-artifact.preserves");
        write_file(&payload, r#"<doc "old" ["old" "keep"]>"#).expect("write rewrite payload");
        run_artifact_command(ArtifactCommand::Install {
            payload,
            registry: registry.to_path_buf(),
            kind: "doc".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(artifact_out),
            receipt_out: Some(dir.join("doc-install-receipt.preserves")),
        })
        .expect("install rewrite artifact");
    }

    fn preview_rewrite(dir: &Path, registry: &Path) -> PathBuf {
        let matches_out = dir.join("rewrite-matches.preserves");
        let plan_out = dir.join("rewrite-plan.preserves");
        run_rewrite_command(RewriteCommand::Find {
            registry: registry.to_path_buf(),
            pattern_kind: "string-equals".to_string(),
            pattern: "old".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            matches_out: Some(matches_out.clone()),
            receipt_out: Some(dir.join("rewrite-find-receipt.preserves")),
        })
        .expect("rewrite find");
        assert!(fs::read_to_string(&matches_out).expect("read matches").contains("rewrite-match-v1"));
        run_rewrite_command(RewriteCommand::Preview {
            registry: registry.to_path_buf(),
            from: "old".to_string(),
            to: "new".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            plan_out: Some(plan_out.clone()),
            receipt_out: Some(dir.join("rewrite-preview-receipt.preserves")),
        })
        .expect("rewrite preview");
        run_rewrite_command(RewriteCommand::Show {
            artifact: plan_out.clone(),
        })
        .expect("rewrite show plan");
        plan_out
    }

    fn apply_rewrite(dir: &Path, registry: &Path, _plan_out: PathBuf) {
        let apply_receipt = dir.join("rewrite-apply-receipt.preserves");
        let upgrade_plan = dir.join("rewrite-upgrade-plan.preserves");
        run_rewrite_command(RewriteCommand::Apply {
            registry: registry.to_path_buf(),
            from: "old".to_string(),
            to: "new".to_string(),
            artifact_kinds: vec!["doc".to_string()],
            root_refs: Vec::new(),
            dependency_inclusion_enabled: true,
            hidden_refs: Vec::new(),
            plan_out: None,
            receipt_out: Some(apply_receipt.clone()),
            upgrade_plan_out: Some(upgrade_plan.clone()),
            session_id: "rewrite-cli-session".to_string(),
        })
        .expect("rewrite apply");
        run_rewrite_command(RewriteCommand::Show {
            artifact: apply_receipt,
        })
        .expect("rewrite show receipt");
        assert!(fs::read_to_string(upgrade_plan).expect("read upgrade plan").contains("upgrade-plan-v1"));
    }

    fn assert_rewritten_doc(registry: PathBuf) {
        let docs = artifacts::list_artifacts(&registry, Some("doc")).expect("list rewritten docs");
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|artifact| {
            artifacts::read_payload(&registry, &artifact.artifact_ref)
                .and_then(|value| to_text(&value))
                .is_ok_and(|text| text.contains("new"))
        }));
    }
