fn validate_obligation(obligation: &ProofObligationInput) -> Result<()> {
    validate_text("obligation id", &obligation.id)?;
    validate_obligation_class(&obligation.class)?;
    validate_ref(&obligation.subject_ref, "obligation subject")?;
    validate_ref_list("obligation prerequisite refs", &obligation.prerequisite_refs, MAX_RECEIPT_REFS)?;
    validate_ref_list("obligation receipt refs", &obligation.receipt_refs, MAX_RECEIPT_REFS)?;
    validate_decision(&obligation.decision)?;
    for requirement_id in &obligation.requirement_ids {
        validate_requirement_id(requirement_id)?;
    }
    if let Some(kind) = obligation.coverage_kind.as_ref() {
        validate_coverage_kind(kind)?;
    }
    validate_string_list("obligation caveats", &obligation.caveats, MAX_RECEIPT_REFS)
}

fn obligation_expected_decision(class: &str) -> Result<&'static str> {
    match class {
        "fail-closed-negative" => Ok("deny"),
        "input-validation" | "canonicalization" | "admission" | "mutation-boundary" | "replay-determinism" => {
            Ok("pass")
        }
        other => Err(MoltenError::invalid_harness(format!("unsupported proof obligation class {other}"))),
    }
}

fn aggregate_proof_value(
    input: &AggregateProofInput,
    obligations: &[ProofObligationInput],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("aggregate-proof-manifest-v1", vec![
        string(AGGREGATE_PROOF_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest-id", vec![string(&input.manifest_id)]),
        record("subject", vec![string(&input.subject_ref)]),
        record("required", vec![sequence(string_values(&input.required_obligation_ids)?)]),
        record("obligations", vec![sequence(obligation_values(obligations)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
        record("caveats", vec![sequence(vec![
            string("aggregate proof manifests are evidence only"),
            string("subsystem gates still control authority and side effects"),
        ])]),
    ]))
}

fn obligation_values(obligations: &[ProofObligationInput]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(obligations.len());
    for obligation in obligations {
        values.push(record("obligation", vec![
            record("id", vec![string(&obligation.id)]),
            record("class", vec![string(&obligation.class)]),
            record("subject", vec![string(&obligation.subject_ref)]),
            record("prerequisites", vec![sequence(string_values(&obligation.prerequisite_refs)?)]),
            record("receipts", vec![sequence(string_values(&obligation.receipt_refs)?)]),
            record("decision", vec![string(&obligation.decision)]),
            record("requirements", vec![sequence(string_values(&obligation.requirement_ids)?)]),
            record("coverage-kind", vec![optional_string_value(obligation.coverage_kind.as_deref())]),
            record("caveats", vec![sequence(string_values(&obligation.caveats)?)]),
        ]));
    }
    Ok(values)
}

fn layered_proof_diagnostics(input: &LayeredProofInput) -> Result<Vec<String>> {
    ensure_count_at_most(input.layers.len(), MAX_PROOF_LAYERS, "proof layers")?;
    let max_diagnostics = layered_proof_diagnostic_bound(&input.layers)?;
    let mut diagnostics = Vec::with_capacity(input.layers.len().saturating_add(1));
    if input.layers.is_empty() {
        diagnostics.push_limited("missing-layers".to_string(), max_diagnostics, "layered proof diagnostics")?;
    }
    let mut ids = OrderedSet::new();
    for layer in &input.layers {
        validate_layer(layer)?;
        if !ids.insert(layer.id.clone()) {
            diagnostics.push_limited(
                format!("duplicate-layer:{}", layer.id),
                max_diagnostics,
                "layered proof diagnostics",
            )?;
        }
        if layer.subject_ref != input.subject_ref {
            diagnostics.push_limited(
                format!("wrong-subject:{}", layer.id),
                max_diagnostics,
                "layered proof diagnostics",
            )?;
        }
        if layer.role == "operator-readback" && layer.decision == "pass" {
            diagnostics.push_limited(
                format!("diagnostic-readback-used-as-pass:{}", layer.id),
                max_diagnostics,
                "layered proof diagnostics",
            )?;
        }
    }
    let by_id = input.layers.iter().map(|layer| (layer.id.clone(), layer)).collect::<OrderedMap<_, _>>();
    ensure_count_at_most(by_id.len(), MAX_PROOF_LAYERS, "proof layer ids")?;
    for layer in &input.layers {
        for child in &layer.child_ids {
            let Some(child_layer) = by_id.get(child) else {
                diagnostics.push_limited(
                    format!("stale-child:{}:{child}", layer.id),
                    max_diagnostics,
                    "layered proof diagnostics",
                )?;
                continue;
            };
            if child_layer.subject_ref != layer.subject_ref {
                diagnostics.push_limited(
                    format!("wrong-child-subject:{}:{child}", layer.id),
                    max_diagnostics,
                    "layered proof diagnostics",
                )?;
            }
            if !role_can_bind(&layer.role, &child_layer.role) {
                diagnostics.push_limited(
                    format!("unsupported-layer-link:{}:{}", layer.role, child_layer.role),
                    max_diagnostics,
                    "layered proof diagnostics",
                )?;
            }
        }
    }
    for cycle in layer_cycles(&by_id)? {
        diagnostics.push_limited(format!("cycle:{cycle}"), max_diagnostics, "layered proof diagnostics")?;
    }
    Ok(diagnostics)
}

fn validate_layer(layer: &ProofLayerInput) -> Result<()> {
    validate_text("proof layer id", &layer.id)?;
    validate_layer_role(&layer.role)?;
    validate_ref(&layer.subject_ref, "proof layer subject")?;
    validate_decision(&layer.decision)?;
    validate_string_list("proof layer child ids", &layer.child_ids, MAX_PROOF_LAYERS)?;
    validate_ref_list("proof layer evidence refs", &layer.evidence_refs, MAX_RECEIPT_REFS)?;
    validate_string_list("proof layer caveats", &layer.caveats, MAX_RECEIPT_REFS)
}

fn layer_cycles(by_id: &OrderedMap<String, &ProofLayerInput>) -> Result<Vec<String>> {
    let mut cycles = Vec::with_capacity(by_id.len());
    for id in by_id.keys() {
        let mut seen = OrderedSet::new();
        let mut stack = Vec::with_capacity(by_id.len());
        stack.push_limited(id.clone(), MAX_LAYER_STACK_ITEMS, "proof layer traversal stack")?;
        while let Some(current) = stack.pop() {
            if !seen.insert(current.clone()) {
                cycles.push_limited(id.clone(), MAX_PROOF_LAYERS, "proof layer cycles")?;
                break;
            }
            if let Some(layer) = by_id.get(&current) {
                for child in &layer.child_ids {
                    stack.push_limited(child.clone(), MAX_LAYER_STACK_ITEMS, "proof layer traversal stack")?;
                }
            }
        }
    }
    cycles.sort();
    cycles.dedup();
    Ok(cycles)
}

fn role_can_bind(parent: &str, child: &str) -> bool {
    match parent {
        "pure-core" => false,
        "gate" => child == "pure-core",
        "replay" => child == "pure-core" || child == "gate",
        "release" => child == "pure-core" || child == "gate" || child == "replay",
        "operator-readback" => true,
        _ => false,
    }
}

fn layered_proof_value(
    subject_ref: &str,
    layers: &[ProofLayerInput],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("layered-proof-manifest-v1", vec![
        string(LAYERED_PROOF_MANIFEST_SCHEMA),
        record("decision", vec![string(decision)]),
        record("subject", vec![string(subject_ref)]),
        record("layers", vec![sequence(layer_values(layers)?)]),
        record("diagnostics", vec![sequence(string_values(diagnostics)?)]),
        record("caveats", vec![sequence(vec![
            string("layered proof evidence does not promote trust automatically"),
            string("operator readbacks are non-normative summaries"),
        ])]),
    ]))
}

fn layer_values(layers: &[ProofLayerInput]) -> Result<Vec<IoValue>> {
    let mut values = Vec::with_capacity(layers.len());
    for layer in layers {
        values.push(record("layer", vec![
            record("id", vec![string(&layer.id)]),
            record("role", vec![string(&layer.role)]),
            record("subject", vec![string(&layer.subject_ref)]),
            record("decision", vec![string(&layer.decision)]),
            record("children", vec![sequence(string_values(&layer.child_ids)?)]),
            record("evidence", vec![sequence(string_values(&layer.evidence_refs)?)]),
            record("caveats", vec![sequence(string_values(&layer.caveats)?)]),
        ]));
    }
    Ok(values)
}

