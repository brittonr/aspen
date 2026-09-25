
fn dependency_edge_value(input: DependencyEdgeInput<'_>) -> Result<IoValue> {
    let DependencyEdgeInput { source_ref, target_ref, target_kind, relation, required, scope, evidence_refs } = input;
    validate_ref(source_ref, "artifact dependency edge source ref")?;
    validate_ref(target_ref, "artifact dependency edge target ref")?;
    validate_dependency_label(target_kind, "artifact dependency edge target kind")?;
    validate_dependency_label(relation, "artifact dependency edge relation")?;
    validate_dependency_label(scope, "artifact dependency edge scope")?;
    validate_refs(evidence_refs, "artifact dependency edge evidence ref")?;
    Ok(record("artifact-dependency-edge-v1", vec![
        string(crate::preserves_rail::ARTIFACT_DEPENDENCY_EDGE_SCHEMA),
        record("source", vec![string(source_ref)]),
        record("target", vec![string(target_ref)]),
        record("target-kind", vec![string(target_kind)]),
        record("relation", vec![string(relation)]),
        record("required", vec![bool_value(required)]),
        record("scope", vec![string(scope)]),
        record("evidence", vec![refs_sequence(evidence_refs)]),
        checks_value(&["direct-edge", "content-ref-target", "planning-evidence-only"]),
    ]))
}

fn parse_dependency_edge_value(value: &IoValue) -> Result<ArtifactDependencyEdge> {
    let fields = value
        .collect_simple_record("artifact-dependency-edge-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-dependency-edge-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_DEPENDENCY_EDGE_SCHEMA, "artifact dependency edge")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "direct-edge", "artifact dependency edge")?;
    let required_value = value_to_iovalue(&fields[5]);
    let required_record = simple_record(&required_value, "required", 1)?;
    let is_required = required_record[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness("artifact dependency edge required must be bool"))?;
    Ok(ArtifactDependencyEdge {
        edge_ref: canonical_hash(value)?,
        source_ref: record_ref(&fields[1], "source")?,
        target_ref: record_ref(&fields[2], "target")?,
        target_kind: record_string(&fields[3], "target-kind")?,
        relation: record_string(&fields[4], "relation")?,
        required: is_required,
        scope: record_string(&fields[6], "scope")?,
        evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
        value: value.clone(),
    })
}

fn normalize_dependency_edges(edges: &[ArtifactDependencyEdge]) -> Result<NormalizedDependencyEdges> {
    let mut by_ref = std::collections::BTreeMap::new();
    let mut duplicates = Vec::new();
    for edge in edges {
        let parsed = parse_dependency_edge_value(&edge.value)?;
        if by_ref.insert(parsed.edge_ref.clone(), parsed).is_some() {
            push_bounded(
                &mut duplicates,
                edge.edge_ref.clone(),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact dependency duplicate refs",
            )?;
        }
    }
    Ok(NormalizedDependencyEdges {
        edges: by_ref.into_values().collect(),
        duplicate_refs: duplicates,
    })
}

fn validate_dependency_label(value: &str, field: &str) -> Result<()> {
    validate_non_empty(value, field)?;
    if value.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{field} {value} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_relation_filters(filters: &[String]) -> Result<()> {
    ensure_count_at_most(filters.len(), MAX_ARTIFACT_REF_LIST, "artifact impact query relation filters")?;
    for filter in filters {
        validate_dependency_label(filter, "artifact impact query relation filter")?;
    }
    Ok(())
}

fn relation_allowed(edge: &ArtifactDependencyEdge, filters: &[String]) -> bool {
    filters.is_empty() || filters.iter().any(|filter| filter == &edge.relation)
}

fn dependents_from_edges(
    edges: &[ArtifactDependencyEdge],
    subjects: &[String],
    filters: &[String],
) -> Result<Vec<String>> {
    let mut dependents = std::collections::BTreeSet::new();
    for edge in edges {
        if subjects.iter().any(|subject| subject == &edge.target_ref) && relation_allowed(edge, filters) {
            checked_count_sum(dependents.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact impact dependents")?;
            dependents.insert(edge.source_ref.clone());
        }
    }
    Ok(dependents.into_iter().collect())
}

fn transitive_dependents_from_edges(
    edges: &[ArtifactDependencyEdge],
    subject_ref: &str,
    filters: &[String],
) -> Result<Vec<String>> {
    let mut visited = std::collections::BTreeSet::new();
    let mut frontier = vec![subject_ref.to_string()];
    while let Some(current) = frontier.pop() {
        let direct = dependents_from_edges(edges, &[current], filters)?;
        for dependent in direct {
            if visited.insert(dependent.clone()) {
                push_bounded(&mut frontier, dependent, MAX_ARTIFACT_REF_LIST, "artifact impact traversal frontier")?;
            }
        }
    }
    Ok(visited.into_iter().collect())
}

fn redact_refs(refs: &[String], hidden: &std::collections::BTreeSet<String>) -> Result<Vec<String>> {
    let mut visible = Vec::new();
    for value_ref in refs {
        if !hidden.contains(value_ref) {
            push_bounded(&mut visible, value_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact impact visible refs")?;
        }
    }
    Ok(visible)
}

fn redacted_refs(refs: &[String], hidden: &std::collections::BTreeSet<String>) -> Result<Vec<String>> {
    let mut redacted = Vec::new();
    for value_ref in refs {
        if hidden.contains(value_ref) {
            push_bounded(&mut redacted, value_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact impact redacted refs")?;
        }
    }
    Ok(redacted)
}

fn impact_query_ref(input: &ArtifactImpactQueryInput, index_ref: &str) -> Result<String> {
    canonical_hash(&record("artifact-impact-query-v1", vec![
        record("subject", vec![string(&input.subject_ref)]),
        record("relations", vec![sequence(input.relation_filters.iter().map(string).collect())]),
        record("transitive", vec![bool_value(input.include_transitive)]),
        record("hidden", vec![refs_sequence(&input.hidden_refs)]),
        record("index", vec![string(index_ref)]),
    ]))
}
