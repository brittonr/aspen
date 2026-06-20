    #[test]
    fn cli_artifact_registry_commands_work() {
        let dir = temp_dir("artifact-cli");
        let registry = dir.join("registry");
        let (base_ref, dep_ref) = install_artifact_pair(&dir, &registry);
        show_artifact_surfaces(&registry, &dep_ref);
        exercise_artifact_name_and_graph(&dir, registry, base_ref, dep_ref);
    }

    fn install_artifact_pair(dir: &Path, registry: &Path) -> (String, String) {
        let base_payload = dir.join("base.preserves");
        let dep_payload = dir.join("dependent.preserves");
        let base_out = dir.join("base-artifact.preserves");
        let dep_out = dir.join("dependent-artifact.preserves");
        write_file(&base_payload, "<schema \"base\">").expect("write base payload");
        write_file(&dep_payload, "<module \"dependent\">").expect("write dep payload");
        run_artifact_command(ArtifactCommand::Install {
            payload: base_payload,
            registry: registry.to_path_buf(),
            kind: "schema".to_string(),
            dependencies: Vec::new(),
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(base_out.clone()),
            receipt_out: Some(dir.join("base-install-receipt.preserves")),
        })
        .expect("install base artifact");
        let base_value = read_preserves_file(&base_out).expect("read base artifact");
        let base = artifacts::parse_artifact_value(&base_value).expect("parse base artifact");
        run_artifact_command(ArtifactCommand::Install {
            payload: dep_payload,
            registry: registry.to_path_buf(),
            kind: "steel".to_string(),
            dependencies: vec![base.artifact_ref.clone()],
            schema_refs: Vec::new(),
            effect_manifest_ref: None,
            artifact_out: Some(dep_out.clone()),
            receipt_out: Some(dir.join("dep-install-receipt.preserves")),
        })
        .expect("install dependent artifact");
        let dep_value = read_preserves_file(&dep_out).expect("read dependent artifact");
        let dep = artifacts::parse_artifact_value(&dep_value).expect("parse dependent artifact");
        (base.artifact_ref, dep.artifact_ref)
    }

    fn show_artifact_surfaces(registry: &Path, dep_ref: &str) {
        run_artifact_command(ArtifactCommand::List {
            registry: registry.to_path_buf(),
            kind: None,
        })
        .expect("artifact list");
        run_artifact_command(ArtifactCommand::View {
            artifact_ref: dep_ref.to_string(),
            registry: registry.to_path_buf(),
            payload: false,
        })
        .expect("artifact view envelope");
        run_artifact_command(ArtifactCommand::View {
            artifact_ref: dep_ref.to_string(),
            registry: registry.to_path_buf(),
            payload: true,
        })
        .expect("artifact view payload");
    }

    fn exercise_artifact_name_and_graph(dir: &Path, registry: PathBuf, base_ref: String, dep_ref: String) {
        run_artifact_command(ArtifactCommand::NameSet {
            registry: registry.clone(),
            kind: "name".to_string(),
            name: "app/main".to_string(),
            artifact_ref: dep_ref.clone(),
            receipt_out: Some(dir.join("name-set-receipt.preserves")),
        })
        .expect("artifact name set");
        run_artifact_command(ArtifactCommand::NameShow {
            registry: registry.clone(),
            kind: "name".to_string(),
            name: "app/main".to_string(),
        })
        .expect("artifact name show");
        run_artifact_command(ArtifactCommand::Deps {
            artifact_ref: dep_ref.clone(),
            registry: registry.clone(),
        })
        .expect("artifact deps");
        run_artifact_command(ArtifactCommand::Closure {
            artifact_ref: dep_ref,
            registry: registry.clone(),
            receipt_out: Some(dir.join("closure-receipt.preserves")),
        })
        .expect("artifact closure");
        run_artifact_command(ArtifactCommand::Impact {
            artifact_ref: base_ref,
            registry: registry.clone(),
            receipt_out: Some(dir.join("impact-receipt.preserves")),
        })
        .expect("artifact impact");
        run_artifact_command(ArtifactCommand::IndexRebuild {
            registry,
            receipt_out: Some(dir.join("rebuild-receipt.preserves")),
        })
        .expect("artifact index rebuild");
    }
