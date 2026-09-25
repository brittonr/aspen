
// r[impl molten.catalog.share_like_linked_views]
// r[impl molten.catalog.redaction_authorization]
pub fn impact(registry_root: &Path, ledger_root: Option<&Path>, input: &GraphInput) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = build_query_value(&QueryValueInput {
        operation: "impact",
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: false,
        include_dependents: input.transitive,
        filters: &[Filter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: "impact-query",
        include_payload: false,
    })?;
    let impact = crate::artifacts::impact_query(registry_root, &crate::artifacts::ArtifactImpactQueryInput {
        subject_ref: full_ref,
        relation_filters: Vec::new(),
        include_transitive: input.transitive,
        hidden_refs: input.visibility.hidden_refs.clone(),
    })?;
    let mut items = vec![impact.receipt_value];
    let summaries = collect_summaries(registry_root, ledger_root, &input.visibility)?;
    for reference in impact.direct_dependents.iter().chain(impact.transitive_dependents.iter()) {
        if let Some(summary) = summaries.iter().find(|summary| &summary.artifact_ref == reference) {
            push_bounded(&mut items, summary.value.clone(), MAX_CATALOG_ITEMS, "catalog impact items")?;
        }
    }
    finish_query("impact", query_value, items, impact.diagnostics)
}
