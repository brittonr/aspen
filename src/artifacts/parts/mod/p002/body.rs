
pub fn reference_diagnostics(root: &Path, target_ref: &str) -> Result<Vec<String>> {
    validate_ref(target_ref, "artifact reference diagnostic ref")?;
    let mut diagnostics = Vec::new();
    if let Ok(impact) = impact_refs(root, &[target_ref.to_string()])
        && impact.iter().any(|reference| reference != target_ref)
    {
        push_bounded(
            &mut diagnostics,
            format!("registry reverse dependencies retain {target_ref}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact reference diagnostics",
        )?;
    }
    for pointer in all_name_pointers(root)? {
        if pointer.artifact_ref == target_ref || pointer.previous_ref.as_deref() == Some(target_ref) {
            push_bounded(
                &mut diagnostics,
                format!("registry pointer {}:{} retains {target_ref}", pointer.pointer_kind, pointer.name),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact reference diagnostics",
            )?;
        }
    }
    if registry_contains_structural_ref(root, target_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("registry receipts or metadata retain {target_ref}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact reference diagnostics",
        )?;
    }
    Ok(diagnostics)
}

pub fn rebuild_index(root: &Path) -> Result<ArtifactIndexRebuild> {
    ensure_dirs(root)?;
    let artifacts = list_artifacts(root, None)?;
    let names = all_name_pointers(root)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_derived_index_tables_in_tx(&write_txn)?;
    for artifact in &artifacts {
        store_derived_indexes_in_tx(&write_txn, artifact)?;
    }
    for pointer in &names {
        let mut table = write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        table
            .insert(
                name_key(&pointer.pointer_kind, &pointer.name)?.as_str(),
                canonical_bytes(&pointer.value)?.as_slice(),
            )
            .map_err(index_error)?;
    }
    let mut refs = Vec::new();
    for artifact in &artifacts {
        push_bounded(&mut refs, artifact.artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact index rebuild refs")?;
    }
    let rebuild_ref = local_ref("artifact-index-rebuild", &refs)?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        subject_ref: &rebuild_ref,
        name: None,
        refs: &refs,
        diagnostics: &[],
        checks: &[
            ("redb-index-artifacts", "pass"),
            ("redb-index-dependencies", "pass"),
            ("redb-index-reverse-dependencies", "pass"),
            ("redb-index-semantic", "pass"),
        ],
    })?;
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(ArtifactIndexRebuild {
        artifacts: artifacts.len(),
        names: names.len(),
        receipt_value,
    })
}

// r[impl molten.artifacts.dependency_edge_records]
pub fn dependency_edges_for_artifact(artifact: &ArtifactRecord) -> Result<Vec<ArtifactDependencyEdge>> {
    let mut edges = Vec::new();
    for dependency_ref in &artifact.dependency_refs {
        push_dependency_edge(
            &mut edges,
            artifact,
            dependency_ref,
            "artifact",
            "imports",
            true,
            artifact.evidence_refs.as_slice(),
        )?;
    }
    for schema_ref in &artifact.schema_refs {
        push_dependency_edge(
            &mut edges,
            artifact,
            schema_ref,
            "schema",
            "validates-with",
            true,
            artifact.evidence_refs.as_slice(),
        )?;
    }
    if let Some(effect_manifest_ref) = artifact.effect_manifest_ref.as_ref() {
        push_dependency_edge(
            &mut edges,
            artifact,
            effect_manifest_ref,
            "effect",
            "invokes",
            true,
            artifact.evidence_refs.as_slice(),
        )?;
    }
    for policy_ref in &artifact.policy_refs {
        push_dependency_edge(
            &mut edges,
            artifact,
            policy_ref,
            "policy",
            "validates-with",
            true,
            artifact.evidence_refs.as_slice(),
        )?;
    }
    for evidence_ref in &artifact.evidence_refs {
        push_dependency_edge(&mut edges, artifact, evidence_ref, "evidence", "documents", false, &[])?;
    }
    Ok(edges)
}

pub fn list_dependency_edges(root: &Path) -> Result<Vec<ArtifactDependencyEdge>> {
    let mut edges = Vec::new();
    for artifact in list_artifacts(root, None)? {
        extend_bounded(
            &mut edges,
            dependency_edges_for_artifact(&artifact)?,
            MAX_ARTIFACT_RECORDS,
            "artifact dependency edges",
        )?;
    }
    let normalized = normalize_dependency_edges(&edges)?.edges;
    Ok(normalized)
}

// r[impl molten.artifacts.reverse_dependency_index]
// r[impl molten.artifacts.index_rebuild_determinism]
pub fn dependency_index_digest(edges: &[ArtifactDependencyEdge]) -> Result<String> {
    let normalized = normalize_dependency_edges(edges)?;
    let edge_refs = normalized.edges.iter().map(|edge| edge.edge_ref.clone()).collect::<Vec<_>>();
    local_ref("artifact-dependency-index", &edge_refs)
}

// r[impl molten.artifacts.impact_query_receipts]
pub fn impact_query(root: &Path, input: &ArtifactImpactQueryInput) -> Result<ArtifactImpactQueryReceipt> {
    validate_ref(&input.subject_ref, "artifact impact query subject ref")?;
    validate_relation_filters(&input.relation_filters)?;
    validate_refs(&input.hidden_refs, "artifact impact query hidden ref")?;
    let edges = list_dependency_edges(root)?;
    let index_ref = dependency_index_digest(&edges)?;
    let hidden = input.hidden_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let direct_all = dependents_from_edges(&edges, &[input.subject_ref.clone()], &input.relation_filters)?;
    let direct = redact_refs(&direct_all, &hidden)?;
    let redacted_direct = redacted_refs(&direct_all, &hidden)?;
    let transitive_all = if input.include_transitive {
        transitive_dependents_from_edges(&edges, &input.subject_ref, &input.relation_filters)?
    } else {
        Vec::new()
    };
    let transitive = redact_refs(&transitive_all, &hidden)?;
    let redacted_transitive = redacted_refs(&transitive_all, &hidden)?;
    let redacted = sorted_unique(&[redacted_direct, redacted_transitive].concat());
    let mut diagnostics = Vec::new();
    if !redacted.is_empty() {
        push_bounded(
            &mut diagnostics,
            "impact query redacted hidden dependency refs".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact impact query diagnostics",
        )?;
    }
    let query_ref = impact_query_ref(input, &index_ref)?;
    let mut refs = vec![input.subject_ref.clone(), index_ref.clone(), query_ref.clone()];
    extend_cloned_bounded(&mut refs, &direct, MAX_ARTIFACT_REF_LIST, "artifact impact query refs")?;
    extend_cloned_bounded(&mut refs, &transitive, MAX_ARTIFACT_REF_LIST, "artifact impact query refs")?;
    extend_cloned_bounded(&mut refs, &redacted, MAX_ARTIFACT_REF_LIST, "artifact impact query refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "impact-query",
        decision: "pass",
        subject_ref: &query_ref,
        name: None,
        refs: &sorted_unique(&refs),
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-dependency-edges", "pass"),
            ("reverse-index-digest", "pass"),
            ("redaction-bound", "pass"),
            ("planning-evidence-only", "pass"),
        ],
    })?;
    Ok(ArtifactImpactQueryReceipt {
        query_ref,
        decision: "pass".to_string(),
        direct_dependents: direct,
        transitive_dependents: transitive,
        redacted_refs: redacted,
        diagnostics,
        receipt_value,
    })
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<ArtifactReceipt> {
    validate_ref(receipt_ref, "artifact receipt ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = receipts.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("artifact receipt {receipt_ref} not found")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_artifact_receipt(&value)
}

pub fn list_receipts(root: &Path) -> Result<Vec<ArtifactReceipt>> {
    let mut receipts = Vec::new();
    for value in receipt_values(root)? {
        push_bounded(
            &mut receipts,
            parse_artifact_receipt(&value)?,
            MAX_ARTIFACT_RECEIPTS,
            "artifact registry receipts",
        )?;
    }
    receipts.sort_by(|left, right| left.receipt_ref.cmp(&right.receipt_ref));
    Ok(receipts)
}

pub fn parse_artifact_receipt(value: &IoValue) -> Result<ArtifactReceipt> {
    let fields = value
        .collect_simple_record("artifact-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_RECEIPT_SCHEMA, "artifact receipt")?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("artifact receipt missing checks"));
    }
    Ok(ArtifactReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        subject_ref: record_ref(&fields[3], "subject")?,
        name: record_optional_string(&fields[4], "name")?,
        value: value.clone(),
    })
}

fn store_artifact_in_tx(
    write_txn: &redb::WriteTransaction,
    artifact: &ArtifactRecord,
    payload_bytes: &[u8],
) -> Result<()> {
    {
        let artifact_bytes = canonical_bytes(&artifact.value)?;
        let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
        artifacts.insert(artifact.artifact_ref.as_str(), artifact_bytes.as_slice()).map_err(index_error)?;
    }
    if matches!(artifact.payload, ArtifactPayloadRef::Inline { .. }) {
        let mut payloads = write_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
        payloads.insert(artifact.artifact_ref.as_str(), payload_bytes).map_err(index_error)?;
    }
    store_derived_indexes_in_tx(write_txn, artifact)
}

fn store_derived_indexes_in_tx(write_txn: &redb::WriteTransaction, artifact: &ArtifactRecord) -> Result<()> {
    {
        let deps_value = refs_value(&artifact.dependency_refs);
        let mut deps = write_txn.open_table(INDEX_DEPS).map_err(index_error)?;
        deps.insert(artifact.artifact_ref.as_str(), canonical_bytes(&deps_value)?.as_slice())
            .map_err(index_error)?;
    }
    for dependency_ref in &artifact.dependency_refs {
        let mut existing = {
            let reverse = write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
            if let Some(bytes) = reverse.get(dependency_ref.as_str()).map_err(index_error)? {
                parse_refs_value(&parse_canonical_bytes(bytes.value())?, "reverse")?
            } else {
                Vec::new()
            }
        };
        if !existing.iter().any(|value| value == &artifact.artifact_ref) {
            push_bounded(
                &mut existing,
                artifact.artifact_ref.clone(),
                MAX_ARTIFACT_REF_LIST,
                "artifact reverse dependency refs",
            )?;
            existing.sort();
        }
        let mut reverse = write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
        reverse
            .insert(dependency_ref.as_str(), canonical_bytes(&refs_value(&existing))?.as_slice())
            .map_err(index_error)?;
    }
    insert_str_index(write_txn, INDEX_KIND, "kind", &artifact.kind, &artifact.artifact_ref)?;
    for schema_ref in &artifact.schema_refs {
        insert_str_index(write_txn, INDEX_SCHEMA, "schema", schema_ref, &artifact.artifact_ref)?;
    }
    if let Some(effect_manifest_ref) = artifact.effect_manifest_ref.as_ref() {
        insert_str_index(write_txn, INDEX_EFFECT, "effect", effect_manifest_ref, &artifact.artifact_ref)?;
    }
    for policy_ref in &artifact.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", policy_ref, &artifact.artifact_ref)?;
    }
    for evidence_ref in &artifact.evidence_refs {
        insert_str_index(write_txn, INDEX_EVIDENCE, "evidence", evidence_ref, &artifact.artifact_ref)?;
    }
    Ok(())
}

fn insert_str_index(
    write_txn: &redb::WriteTransaction,
    table_definition: TableDef<&str, &str>,
    index_name: &str,
    indexed_ref: &str,
    artifact_ref: &str,
) -> Result<()> {
    let key = canonical_hash(&record("artifact-semantic-index-key", vec![
        string(index_name),
        string(indexed_ref),
        string(artifact_ref),
    ]))?;
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    table.insert(key.as_str(), artifact_ref).map_err(index_error)?;
    Ok(())
}

fn missing_dependencies(root: &Path, dependency_refs: &[String]) -> Result<Vec<String>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let artifacts = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let mut missing = Vec::new();
    for dependency_ref in dependency_refs {
        if artifacts.get(dependency_ref.as_str()).map_err(index_error)?.is_none() {
            push_bounded(&mut missing, dependency_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact missing dependencies")?;
        }
    }
    Ok(missing)
}

fn compute_closure_refs(root: &Path, roots: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    validate_refs(roots, "artifact closure root ref")?;
    let db = ensure_index_tables(root)?;
    let mut closure = std::collections::BTreeSet::new();
    let mut missing = std::collections::BTreeSet::new();
    ensure_count_at_most(roots.len(), MAX_ARTIFACT_REF_LIST, "artifact closure roots")?;
    let mut stack = roots.to_vec();
    while let Some(current) = stack.pop() {
        if closure.contains(&current) || missing.contains(&current) {
            continue;
        }
        let deps = {
            let read_txn = db.begin_read().map_err(index_error)?;
            let artifacts = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
            if artifacts.get(current.as_str()).map_err(index_error)?.is_none() {
                checked_count_sum(missing.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact closure missing refs")?;
                missing.insert(current.clone());
                Vec::new()
            } else {
                let deps = read_txn.open_table(INDEX_DEPS).map_err(index_error)?;
                if let Some(bytes) = deps.get(current.as_str()).map_err(index_error)? {
                    parse_refs_value(&parse_canonical_bytes(bytes.value())?, "dependencies")?
                } else {
                    Vec::new()
                }
            }
        };
        if !missing.contains(&current) {
            checked_count_sum(closure.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact closure refs")?;
            closure.insert(current);
        }
        for dependency in deps {
            push_bounded(&mut stack, dependency, MAX_ARTIFACT_REF_LIST, "artifact closure traversal stack")?;
        }
    }
    Ok((closure.into_iter().collect(), missing.into_iter().collect()))
}

struct ArtifactReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    subject_ref: &'a str,
    name: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct NamePointerValueInput<'a> {
    pointer_kind: &'a str,
    name: &'a str,
    artifact_ref: &'a str,
    previous_ref: Option<&'a str>,
    policy_refs: &'a [String],
    receipt_ref: &'a str,
}

fn artifact_receipt_value(input: &ArtifactReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "artifact receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported artifact receipt decision {}", input.decision)));
    }
    validate_ref(input.subject_ref, "artifact receipt subject ref")?;
    validate_refs(input.refs, "artifact receipt ref")?;
    Ok(record("artifact-receipt-v1", vec![
        string(crate::preserves_rail::ARTIFACT_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("subject", vec![string(input.subject_ref)]),
        record("name", vec![optional_string_value(input.name)]),
        record("refs", vec![refs_sequence(input.refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

struct NormalizedDependencyEdges {
    edges: Vec<ArtifactDependencyEdge>,
    duplicate_refs: Vec<String>,
}

fn push_dependency_edge(
    edges: &mut Vec<ArtifactDependencyEdge>,
    artifact: &ArtifactRecord,
    target_ref: &str,
    target_kind: &str,
    relation: &str,
    required: bool,
    evidence_refs: &[String],
) -> Result<()> {
    let edge = dependency_edge(
        &artifact.artifact_ref,
        target_ref,
        target_kind,
        relation,
        required,
        &artifact.kind,
        evidence_refs,
    )?;
    push_bounded(edges, edge, MAX_ARTIFACT_RECORDS, "artifact dependency edges")
}

fn dependency_edge(
    source_ref: &str,
    target_ref: &str,
    target_kind: &str,
    relation: &str,
    required: bool,
    scope: &str,
    evidence_refs: &[String],
) -> Result<ArtifactDependencyEdge> {
    let value = dependency_edge_value(source_ref, target_ref, target_kind, relation, required, scope, evidence_refs)?;
    let edge_ref = canonical_hash(&value)?;
    Ok(ArtifactDependencyEdge {
        edge_ref,
        source_ref: source_ref.to_string(),
        target_ref: target_ref.to_string(),
        target_kind: target_kind.to_string(),
        relation: relation.to_string(),
        required,
        scope: scope.to_string(),
        evidence_refs: evidence_refs.to_vec(),
        value,
    })
}

fn dependency_edge_value(
    source_ref: &str,
    target_ref: &str,
    target_kind: &str,
    relation: &str,
    required: bool,
    scope: &str,
    evidence_refs: &[String],
) -> Result<IoValue> {
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
    let required = required_record[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness("artifact dependency edge required must be bool"))?;
    Ok(ArtifactDependencyEdge {
        edge_ref: canonical_hash(value)?,
        source_ref: record_ref(&fields[1], "source")?,
        target_ref: record_ref(&fields[2], "target")?,
        target_kind: record_string(&fields[3], "target-kind")?,
        relation: record_string(&fields[4], "relation")?,
        required,
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
