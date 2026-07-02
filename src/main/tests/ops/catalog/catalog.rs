#[test]
fn cli_catalog_commands_work() {
    let dir = temp_dir("catalog-cli");
    let fixture = catalog_install_fixture(&dir);
    catalog_query_fixture(&dir, &fixture);
    catalog_mcp_fixture(&dir, fixture);
}

struct CatalogCliFixture {
    registry: PathBuf,
    ledger_root: PathBuf,
    base_ref: String,
    dep_ref: String,
    list_receipt: PathBuf,
    view_receipt: PathBuf,
}

fn catalog_install_fixture(dir: &Path) -> CatalogCliFixture {
    let registry = dir.join("registry");
    let ledger_root = dir.join("ledger");
    write_file(&dir.join("catalog-base.preserves"), r#"<schema "catalog-base">"#)
        .expect("write catalog base payload");
    write_file(
        &dir.join("catalog-dependent.preserves"),
        r#"<doc "catalog-text" ["searchable"]>"#,
    )
    .expect("write catalog dep payload");
    let base = install_catalog_artifact(dir, &registry, "catalog-base", "schema", Vec::new());
    let dep = install_catalog_artifact(dir, &registry, "catalog-dependent", "doc", vec![base.artifact_ref.clone()]);
    molten::ledger::import_artifact(&ledger_root, &dep.value).expect("import dep artifact to ledger");
    CatalogCliFixture {
        registry,
        ledger_root,
        base_ref: base.artifact_ref,
        dep_ref: dep.artifact_ref,
        list_receipt: dir.join("catalog-list-receipt.preserves"),
        view_receipt: dir.join("catalog-view-receipt.preserves"),
    }
}

fn install_catalog_artifact(
    dir: &Path,
    registry: &Path,
    name: &str,
    kind: &str,
    dependencies: Vec<String>,
) -> molten::artifacts::ArtifactRecord {
    let artifact_out = dir.join(format!("{name}-artifact.preserves"));
    crate::cli_artifact::run(crate::cli_artifact::Command::Install {
        payload: dir.join(format!("{name}.preserves")),
        registry: registry.to_path_buf(),
        kind: kind.to_string(),
        dependencies,
        schema_refs: Vec::new(),
        effect_manifest_ref: None,
        artifact_out: Some(artifact_out.clone()),
        receipt_out: Some(dir.join(format!("{name}-install-receipt.preserves"))),
    })
    .expect("install catalog artifact");
    molten::artifacts::parse_artifact_value(&read_preserves_file(&artifact_out).expect("read catalog artifact"))
        .expect("parse catalog artifact")
}

fn catalog_query_fixture(dir: &Path, fixture: &CatalogCliFixture) {
    catalog_list_and_view(fixture);
    catalog_search_and_links(dir, fixture);
    catalog_short_id(dir, fixture);
}

fn catalog_list_and_view(fixture: &CatalogCliFixture) {
    crate::cli_catalog::run(crate::cli_catalog::Command::List {
        registry: fixture.registry.clone(),
        ledger: Some(fixture.ledger_root.clone()),
        kind: Some("doc".to_string()),
        hidden_refs: Vec::new(),
        receipt_out: Some(fixture.list_receipt.clone()),
    })
    .expect("catalog list");
    crate::cli_catalog::run(crate::cli_catalog::Command::View {
        reference: fixture.dep_ref.clone(),
        registry: fixture.registry.clone(),
        ledger: Some(fixture.ledger_root.clone()),
        payload_inclusion_enabled: true,
        redaction_enabled: true,
        hidden_refs: Vec::new(),
        receipt_out: Some(fixture.view_receipt.clone()),
    })
    .expect("catalog view");
}

fn catalog_search_and_links(dir: &Path, fixture: &CatalogCliFixture) {
    crate::cli_catalog::run(crate::cli_catalog::Command::Search {
        registry: fixture.registry.clone(),
        ledger: Some(fixture.ledger_root.clone()),
        artifact_kind: Some("doc".to_string()),
        ledger_kind: None,
        schema_ref: None,
        structural_fingerprint: None,
        effect_ref: None,
        policy_ref: None,
        capability_ref: None,
        evidence_ref: None,
        dependency_ref: Some(fixture.base_ref.clone()),
        dependent_ref: None,
        receipt_operation: None,
        receipt_decision: None,
        transcript_status: None,
        upgrade_status: None,
        text: Some("searchable".to_string()),
        root_refs: Vec::new(),
        dependency_inclusion_enabled: true,
        dependent_inclusion_enabled: true,
        hidden_refs: Vec::new(),
        receipt_out: Some(dir.join("catalog-search-receipt.preserves")),
    })
    .expect("catalog search");
    catalog_dependency_views(dir, fixture);
}

fn catalog_dependency_views(dir: &Path, fixture: &CatalogCliFixture) {
    crate::cli_catalog::run(crate::cli_catalog::Command::Deps {
        reference: fixture.dep_ref.clone(),
        registry: fixture.registry.clone(),
        ledger: Some(fixture.ledger_root.clone()),
        transitive: false,
        hidden_refs: Vec::new(),
        receipt_out: Some(dir.join("catalog-deps-receipt.preserves")),
    })
    .expect("catalog deps");
    crate::cli_catalog::run(crate::cli_catalog::Command::Dependents {
        reference: fixture.base_ref.clone(),
        registry: fixture.registry.clone(),
        ledger: Some(fixture.ledger_root.clone()),
        transitive: false,
        hidden_refs: Vec::new(),
        receipt_out: Some(dir.join("catalog-dependents-receipt.preserves")),
    })
    .expect("catalog dependents");
}

fn catalog_short_id(dir: &Path, fixture: &CatalogCliFixture) {
    crate::cli_catalog::run(crate::cli_catalog::Command::ShortId {
        prefix: fixture.dep_ref[7..19].to_string(),
        registry: fixture.registry.clone(),
        ledger: Some(fixture.ledger_root.clone()),
        min_length: 8,
        hidden_refs: Vec::new(),
        receipt_out: Some(dir.join("catalog-short-id-receipt.preserves")),
    })
    .expect("catalog short id");
}

fn catalog_mcp_fixture(dir: &Path, fixture: CatalogCliFixture) {
    let mcp_request = dir.join("catalog-mcp-request.preserves");
    let mcp_response = dir.join("catalog-mcp-response.preserves");
    let mcp_receipt = dir.join("catalog-mcp-receipt.preserves");
    write_catalog_mcp_request(&mcp_request, &fixture.base_ref);
    crate::cli_catalog::run(crate::cli_catalog::Command::McpCall {
        request: mcp_request,
        registry: fixture.registry,
        ledger: Some(fixture.ledger_root),
        chunks: None,
        out: Some(mcp_response.clone()),
        receipt_out: Some(mcp_receipt.clone()),
    })
    .expect("catalog mcp call");
    assert!(fs::read_to_string(&mcp_response)
        .expect("read mcp response")
        .contains(&fixture.dep_ref));
    crate::cli_catalog::run(crate::cli_catalog::Command::Show { artifact: mcp_receipt }).expect("catalog show MCP receipt");
    crate::cli_catalog::run(crate::cli_catalog::Command::Show {
        artifact: fixture.list_receipt,
    })
    .expect("catalog show receipt");
    crate::cli_catalog::run(crate::cli_catalog::Command::Show {
        artifact: fixture.view_receipt,
    })
    .expect("catalog show view receipt");
}

fn write_catalog_mcp_request(path: &Path, base_ref: &str) {
    write_file(
        path,
        &to_text(
            &molten::catalog_mcp::mcp_request_value(
                "catalog.search",
                vec![
                    record("kind", vec![string("doc")]),
                    record("dependency-ref", vec![string(base_ref)]),
                    record("text", vec![string("searchable")]),
                ],
            )
            .expect("mcp request"),
        )
        .expect("render mcp request"),
    )
    .expect("write mcp request");
}
