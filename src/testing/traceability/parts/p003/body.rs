fn requirement_map(requirements: &[RequirementInput]) -> Result<OrderedMap<String, RequirementInput>> {
    if requirements.is_empty() {
        return Err(MoltenError::invalid_harness("traceability manifest requires requirements"));
    }
    if requirements.len() > MAX_REQUIREMENTS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability requirement count {} exceeds bound {MAX_REQUIREMENTS}",
            requirements.len()
        )));
    }
    let mut map = OrderedMap::new();
    for requirement in requirements {
        validate_requirement(requirement)?;
        if map.insert(requirement.id.clone(), requirement.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!("duplicate traceability requirement {}", requirement.id)));
        }
    }
    Ok(map)
}

fn coverage_map(coverage: &[CoverageInput]) -> Result<OrderedMap<String, CoverageInput>> {
    if coverage.len() > MAX_COVERAGE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability coverage count {} exceeds bound {MAX_COVERAGE_ITEMS}",
            coverage.len()
        )));
    }
    let mut map = OrderedMap::new();
    for entry in coverage {
        validate_text("coverage requirement", &entry.requirement_id)?;
        if map.insert(entry.requirement_id.clone(), entry.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate traceability coverage entry {}",
                entry.requirement_id
            )));
        }
    }
    Ok(map)
}

fn entry_for_requirement(
    requirement: &RequirementInput,
    coverage: Option<&CoverageInput>,
    require_receipt_backed: bool,
) -> Result<TraceabilityEntry> {
    let mut diagnostics = Vec::new();
    let positive = coverage.map(|entry| entry.positive.clone()).unwrap_or_default();
    let negative = coverage.map(|entry| entry.negative.clone()).unwrap_or_default();
    let exemption = coverage.and_then(|entry| entry.exemption.clone());
    let is_coverage_required = requires_coverage(requirement);
    let has_coverage = !positive.is_empty() || !negative.is_empty();

    validate_evidence_list("positive", &positive, require_receipt_backed, &mut diagnostics)?;
    validate_evidence_list("negative", &negative, require_receipt_backed, &mut diagnostics)?;
    if let Some(exemption) = exemption.as_ref() {
        validate_exemption(exemption, &mut diagnostics)?;
    }

    let status_input = EntryStatusInput {
        diagnostics: &diagnostics,
        exemption: exemption.as_ref(),
        is_coverage_required,
        has_coverage,
        positive_count: positive.len(),
        negative_count: negative.len(),
    };
    let status = match entry_status_case(&status_input) {
        EntryStatusCase::Stale => "stale-reference",
        EntryStatusCase::Exempt => "exempt",
        EntryStatusCase::MissingPositive => {
            diagnostics.push_limited(
                "missing-positive-coverage".to_string(),
                MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
                "traceability entry diagnostics",
            )?;
            "missing-positive"
        }
        EntryStatusCase::MissingNegative => {
            diagnostics.push_limited(
                "missing-negative-coverage".to_string(),
                MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
                "traceability entry diagnostics",
            )?;
            "missing-negative"
        }
        EntryStatusCase::Covered => "covered",
        EntryStatusCase::Unsupported => "unsupported",
    };

    Ok(TraceabilityEntry {
        requirement_id: requirement.id.clone(),
        source: requirement.source.clone(),
        kind: requirement.kind.clone(),
        changed: requirement.changed,
        status: status.to_string(),
        diagnostics,
        positive,
        negative,
        exemption,
    })
}

fn stale_coverage_entry(coverage: &CoverageInput) -> Result<TraceabilityEntry> {
    let mut diagnostics = Vec::with_capacity(MAX_STATUS_DIAGNOSTICS);
    diagnostics.push_limited(
        "stale-requirement-id".to_string(),
        MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
        "traceability entry diagnostics",
    )?;
    validate_evidence_list("positive", &coverage.positive, false, &mut diagnostics)?;
    validate_evidence_list("negative", &coverage.negative, false, &mut diagnostics)?;
    if let Some(exemption) = coverage.exemption.as_ref() {
        validate_exemption(exemption, &mut diagnostics)?;
    }
    Ok(TraceabilityEntry {
        requirement_id: coverage.requirement_id.clone(),
        source: "<stale-coverage>".to_string(),
        kind: "evidence".to_string(),
        changed: false,
        status: "stale-reference".to_string(),
        diagnostics,
        positive: coverage.positive.clone(),
        negative: coverage.negative.clone(),
        exemption: coverage.exemption.clone(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryStatusCase {
    Stale,
    Exempt,
    MissingPositive,
    MissingNegative,
    Covered,
    Unsupported,
}

struct EntryStatusInput<'a> {
    diagnostics: &'a [String],
    exemption: Option<&'a CoverageExemption>,
    is_coverage_required: bool,
    has_coverage: bool,
    positive_count: usize,
    negative_count: usize,
}

fn entry_status_case(input: &EntryStatusInput<'_>) -> EntryStatusCase {
    match (
        input.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("stale-")),
        input.exemption.is_some(),
        input.is_coverage_required && input.positive_count == 0,
        input.is_coverage_required && input.negative_count == 0,
        input.is_coverage_required || input.has_coverage,
    ) {
        (true, _, _, _, _) => EntryStatusCase::Stale,
        (_, true, _, _, _) => EntryStatusCase::Exempt,
        (_, _, true, _, _) => EntryStatusCase::MissingPositive,
        (_, _, _, true, _) => EntryStatusCase::MissingNegative,
        (_, _, _, _, true) => EntryStatusCase::Covered,
        _ => EntryStatusCase::Unsupported,
    }
}

fn requires_coverage(requirement: &RequirementInput) -> bool {
    requirement.changed || requirement.kind == "evidence"
}
