
fn aggregate_proof_diagnostics(input: &AggregateProofInput) -> Result<Vec<String>> {
    ensure_count_at_most(input.obligations.len(), MAX_PROOF_OBLIGATIONS, "proof obligations")?;
    ensure_count_at_most(input.required_obligation_ids.len(), MAX_PROOF_OBLIGATIONS, "required proof obligations")?;
    let max_diagnostics = aggregate_proof_diagnostic_bound(input)?;
    let mut diagnostics = Vec::with_capacity(input.obligations.len().saturating_add(1));
    if input.obligations.is_empty() {
        diagnostics.push_limited("missing-obligations".to_string(), max_diagnostics, "aggregate proof diagnostics")?;
    }
    let mut ids = OrderedSet::new();
    for obligation in &input.obligations {
        validate_obligation(obligation)?;
        if !ids.insert(obligation.id.clone()) {
            diagnostics.push_limited(
                format!("duplicate-obligation:{}", obligation.id),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
        if obligation.subject_ref != input.subject_ref {
            diagnostics.push_limited(
                format!("wrong-subject:{}", obligation.id),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
        if obligation.decision != obligation_expected_decision(&obligation.class)? {
            diagnostics.push_limited(
                format!("wrong-expected-decision:{}", obligation.id),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
    }
    let obligation_map = input
        .obligations
        .iter()
        .map(|obligation| (obligation.id.clone(), obligation))
        .collect::<OrderedMap<_, _>>();
    for required in &input.required_obligation_ids {
        validate_text("required obligation id", required)?;
        if !obligation_map.contains_key(required) {
            diagnostics.push_limited(
                format!("missing-child:{required}"),
                max_diagnostics,
                "aggregate proof diagnostics",
            )?;
        }
    }
    Ok(diagnostics)
}
