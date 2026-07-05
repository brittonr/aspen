fn validate_evidence_list(
    label: &str,
    evidence: &[VerificationEvidence],
    require_receipt_backed: bool,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    if evidence.len() > MAX_COVERAGE_ITEMS {
        return Err(MoltenError::invalid_harness(format!(
            "traceability {label} evidence count {} exceeds bound {MAX_COVERAGE_ITEMS}",
            evidence.len()
        )));
    }
    let mut receipt_refs = OrderedSet::new();
    for item in evidence {
        let input = EvidenceItemInput {
            label,
            item,
            require_receipt_backed,
        };
        validate_evidence_item(&input, &mut receipt_refs, diagnostics)?;
    }
    Ok(())
}

struct EvidenceItemInput<'a> {
    label: &'a str,
    item: &'a VerificationEvidence,
    require_receipt_backed: bool,
}

fn validate_evidence_item(
    input: &EvidenceItemInput<'_>,
    receipt_refs: &mut OrderedSet<String>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    validate_text("evidence target", &input.item.target)?;
    validate_text("evidence source", &input.item.source)?;
    collect_evidence_presence_diagnostics(input, diagnostics)?;
    collect_evidence_artifact_diagnostics(input, diagnostics)?;
    collect_evidence_receipt_diagnostics(input, receipt_refs, diagnostics)?;
    collect_evidence_decision_diagnostics(input, diagnostics)
}

fn collect_evidence_presence_diagnostics(
    input: &EvidenceItemInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    let label = input.label;
    let item = input.item;
    if !item.target_exists {
        diagnostics.push_limited(
            format!("stale-{label}-target:{}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    if item.command.trim().is_empty() {
        diagnostics.push_limited(
            format!("stale-{label}-command:{}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    if item.source == "compatibility" && input.require_receipt_backed {
        diagnostics.push_limited(
            format!("stale-{label}-compatibility-only:{}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    Ok(())
}

fn collect_evidence_artifact_diagnostics(
    input: &EvidenceItemInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    let label = input.label;
    let item = input.item;
    if !item.artifact_present {
        diagnostics.push_limited(
            format!("stale-{label}-artifact:{}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    if item.artifact_ref.trim().is_empty() {
        diagnostics.push_limited(
            format!("stale-{label}-artifact-ref:{}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    } else if let Err(error) = crate::preserves_rail::validate_content_ref(&item.artifact_ref) {
        diagnostics.push_limited(
            format!("stale-{label}-artifact-ref:{}:{error}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    ensure_count_at_most(item.artifact_refs.len(), MAX_RECEIPT_REFS, "evidence artifact refs")?;
    for artifact_ref in &item.artifact_refs {
        if let Err(error) = crate::preserves_rail::validate_content_ref(artifact_ref) {
            diagnostics.push_limited(
                format!("stale-{label}-artifact-ref:{}:{error}", item.target),
                MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
                "traceability entry diagnostics",
            )?;
        }
    }
    Ok(())
}

fn collect_evidence_receipt_diagnostics(
    input: &EvidenceItemInput<'_>,
    receipt_refs: &mut OrderedSet<String>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    let label = input.label;
    let item = input.item;
    if let Some(receipt_ref) = &item.receipt_ref {
        if !receipt_refs.insert(receipt_ref.clone()) {
            diagnostics.push_limited(
                format!("stale-{label}-duplicate-receipt:{receipt_ref}"),
                MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
                "traceability entry diagnostics",
            )?;
        }
        if let Err(error) = crate::preserves_rail::validate_content_ref(receipt_ref) {
            diagnostics.push_limited(
                format!("stale-{label}-receipt-ref:{}:{error}", item.target),
                MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
                "traceability entry diagnostics",
            )?;
        }
    } else if item.source != "compatibility" {
        diagnostics.push_limited(
            format!("stale-{label}-missing-receipt-ref:{}", item.target),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    Ok(())
}

fn collect_evidence_decision_diagnostics(
    input: &EvidenceItemInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    let label = input.label;
    let item = input.item;
    let expected = expected_decision(label)?;
    if item.source != "compatibility" && item.expected_decision != expected {
        diagnostics.push_limited(
            format!("stale-{label}-expected-decision:{}:{}", item.target, item.expected_decision),
            MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
            "traceability entry diagnostics",
        )?;
    }
    Ok(())
}

fn validate_exemption(exemption: &CoverageExemption, diagnostics: &mut impl PushLimited<String>) -> Result<()> {
    validate_text("exemption class", &exemption.class)?;
    validate_text("exemption evidence", &exemption.evidence)?;
    match exemption.class.as_str() {
        "documentation-only" | "operator-guidance" | "non-executable" => Ok(()),
        other => {
            diagnostics.push_limited(
                format!("stale-exemption-class:{other}"),
                MAX_TRACEABILITY_ENTRY_DIAGNOSTICS,
                "traceability entry diagnostics",
            )?;
            Ok(())
        }
    }
}
