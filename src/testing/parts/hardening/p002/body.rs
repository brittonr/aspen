
/// The deterministic fast-core, harness, CLI, and distributed-simulation profile rows.
fn subsystem_profile_rows() -> Vec<SemanticProfileInput> {
    vec![
        semantic_profile_row(ProfileRowInput {
            profile_id: "fast-core",
            evidence_scope: "unit",
            filter_expression: "package(molten) & test(/hardening|bounded|preserves|profile|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/fast-core/junit.xml",
            cost_class: "fast",
            caveats: &["deterministic profile excludes live, VM, exploratory, retry-only, and diagnostic-only tests"],
            excluded_partitions: required_deterministic_exclusions(),
            platform_required: false,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "harness",
            evidence_scope: "integration",
            filter_expression: "package(molten) & test(/harness|replay|repro|gate|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/harness/junit.xml",
            cost_class: "moderate",
            caveats: &[
                "harness receipts are evidence-only and exclude live/VM diagnostics from deterministic pass evidence",
            ],
            excluded_partitions: required_deterministic_exclusions(),
            platform_required: false,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "cli",
            evidence_scope: "cli",
            filter_expression: "package(molten) & test(/cli|cliharness|command|receipt/) & not test(/live|vm|dogfood|soak|exploratory/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/cli/junit.xml",
            cost_class: "moderate",
            caveats: &["CLI rendered output remains diagnostic unless bound to canonical artifacts"],
            excluded_partitions: required_deterministic_exclusions(),
            platform_required: false,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "distributed-simulation",
            evidence_scope: "integration",
            filter_expression: "package(molten) & test(/distributed|simulation|fault|two_peer|remote/) & not test(/live|vm|dogfood|soak|exploratory/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/distributed-simulation/junit.xml",
            cost_class: "moderate",
            caveats: &["simulation evidence covers deterministic model faults, not live transport behavior"],
            excluded_partitions: required_deterministic_exclusions(),
            platform_required: false,
            platform_available: true,
        }),
    ]
}

/// The VM, dogfood soak, CI, deterministic aggregate, and exploratory profile rows.
fn platform_and_aggregate_profile_rows() -> Vec<SemanticProfileInput> {
    vec![
        semantic_profile_row(ProfileRowInput {
            profile_id: "vm-platform",
            evidence_scope: "vm",
            filter_expression: "package(molten) & test(/vm|nixos|platform/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/vm-platform/junit.xml",
            cost_class: "platform",
            caveats: &["VM evidence is platform integration evidence and availability is host-dependent"],
            excluded_partitions: &[],
            platform_required: true,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "dogfood-soak",
            evidence_scope: "dogfood",
            filter_expression: "package(molten) & test(/dogfood|soak|release/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/dogfood-soak/junit.xml",
            cost_class: "soak",
            caveats: &["dogfood soak evidence is operator-readiness evidence only"],
            excluded_partitions: &[],
            platform_required: false,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "ci",
            evidence_scope: "integration",
            filter_expression: "package(molten)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/ci/junit.xml",
            cost_class: "moderate",
            caveats: &["CI profile aggregates evidence and does not replace subsystem receipts"],
            excluded_partitions: &[],
            platform_required: false,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "deterministic",
            evidence_scope: "integration",
            filter_expression: "package(molten) & not test(/live|vm|dogfood|soak|exploratory/)",
            retry_policy: "zero-retry",
            expected_junit_path: "target/nextest/deterministic/junit.xml",
            cost_class: "moderate",
            caveats: &["deterministic aggregate excludes non-replayable evidence partitions"],
            excluded_partitions: required_deterministic_exclusions(),
            platform_required: false,
            platform_available: true,
        }),
        semantic_profile_row(ProfileRowInput {
            profile_id: "exploratory",
            evidence_scope: "exemption",
            filter_expression: "package(molten)",
            retry_policy: "retry-pass",
            expected_junit_path: "target/nextest/exploratory/junit.xml",
            cost_class: "moderate",
            caveats: &["exploratory retry success is diagnostic-only and cannot satisfy deterministic pass evidence"],
            excluded_partitions: &[],
            platform_required: false,
            platform_available: true,
        }),
    ]
}

pub fn build_nextest_profile_matrix(input: &NextestProfileMatrixInput) -> Result<NextestProfileMatrix> {
    ensure_bound(input.profiles.len(), "nextest profiles")?;
    let mut diagnostics = Vec::with_capacity(input.profiles.len());
    let mut seen = OrderedSet::new();
    for profile in &input.profiles {
        validate_profile(profile, &mut diagnostics)?;
        if !seen.insert(profile.profile_id.clone()) {
            diagnostics.push(format!("duplicate-profile:{}", profile.profile_id));
        }
    }
    diagnostics.extend(
        required_semantic_profiles()
            .iter()
            .filter(|required| !seen.contains(**required))
            .map(|required| format!("missing-profile:{required}")),
    );
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = nextest_profile_matrix_value(input, decision, &diagnostics)?;
    let matrix_ref = hash(&value)?;
    Ok(NextestProfileMatrix {
        decision: decision.to_string(),
        diagnostics,
        matrix_ref,
        value,
    })
}

pub fn build_cli_receipt_first_gate(input: &CliReceiptFirstInput) -> Result<CliReceiptFirstGate> {
    validate_text("cli command", &input.command)?;
    ensure_bound(input.canonical_artifact_refs.len(), "cli artifact refs")?;
    ensure_bound(input.rendered_output_kinds.len(), "cli rendered output kinds")?;
    let mut diagnostics = input.diagnostics.clone();
    for reference in &input.canonical_artifact_refs {
        validate_ref(reference, "cli canonical artifact")?;
    }
    for kind in &input.rendered_output_kinds {
        validate_rendered_output_kind(kind)?;
    }
    if input.evidence_bearing && input.canonical_artifact_refs.is_empty() {
        diagnostics.push("missing-canonical-artifact".to_string());
    }
    if input.negative_case {
        match input.failure_artifact_ref.as_ref() {
            Some(reference) => validate_ref(reference, "cli failure artifact")?,
            None => diagnostics.push("missing-negative-failure-artifact".to_string()),
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_for(&diagnostics);
    let value = cli_receipt_first_value(input, decision, &diagnostics)?;
    let gate_ref = hash(&value)?;
    Ok(CliReceiptFirstGate {
        decision: decision.to_string(),
        diagnostics,
        gate_ref,
        value,
    })
}

fn validate_boundary_observation(
    item: &BoundaryObservationInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_boundary_class(&item.class, diagnostics);
    validate_boundary_polarity(&item.polarity)?;
    validate_text("boundary requirement id", &item.requirement_id)?;
    if let Err(error) = validate_ref(&item.evidence_ref, "boundary evidence") {
        diagnostics.push_item(format!("stale-evidence-ref:{}:{error}", item.class));
    }
    Ok(())
}

fn validate_boundary_requirement(item: &BoundaryRequirementInput) -> Result<()> {
    let mut diagnostics = Vec::new();
    validate_boundary_class(&item.class, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness(diagnostics.join(",")));
    }
    validate_boundary_polarity(&item.polarity)?;
    validate_text("boundary requirement id", &item.requirement_id)
}

fn validate_boundary_exemption(
    item: &BoundaryCoverageExemptionInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_boundary_class(&item.class, diagnostics);
    validate_text("boundary exemption reason", &item.reason)?;
    validate_text("boundary exemption scope", &item.scope)?;
    validate_text("boundary exemption caveat", &item.caveat)?;
    if item.caveat != "diagnostic-only" {
        diagnostics.push_item(format!("exemption-caveat-not-diagnostic-only:{}", item.class));
    }
    if let Err(error) = validate_ref(&item.evidence_ref, "boundary exemption evidence") {
        diagnostics.push_item(format!("exemption-without-evidence:{}:{error}", item.class));
    }
    Ok(())
}

fn validate_boundary_class(class: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if !allowed_boundary_classes().contains(&class) {
        diagnostics.push_item(format!("unsupported-boundary-class:{class}"));
    }
}

fn validate_boundary_polarity(polarity: &str) -> Result<()> {
    match polarity {
        "positive" | "negative" | "diagnostic" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported boundary polarity {other}"))),
    }
}

fn boundary_key(class: &str, polarity: &str) -> String {
    format!("{polarity}:{class}")
}

fn validate_matrix_entry(
    entry: &EvidenceMatrixEntryInput,
    requirements: &OrderedMap<String, crate::requirement_traceability::RequirementInput>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_text("matrix requirement id", &entry.requirement_id)?;
    if !requirements.contains_key(&entry.requirement_id) {
        diagnostics.push_item(format!("stale-requirement-id:{}", entry.requirement_id));
    }
    validate_matrix_coverage_kind(&entry.coverage_kind, diagnostics)?;
    validate_evidence_scope(&entry.evidence_scope, diagnostics)?;
    validate_text("matrix target", &entry.target)?;
    validate_text("matrix command", &entry.command)?;
    if entry.artifact_refs.is_empty() {
        diagnostics.push_item(format!("missing-artifact-ref:{}", entry.requirement_id));
    }
    validate_ref_list_with_diagnostics("matrix artifact", &entry.artifact_refs, diagnostics)?;
    if let Some(reference) = entry.receipt_ref.as_ref() {
        validate_ref_with_diagnostics(reference, "matrix receipt", diagnostics);
    }
    for caveat in &entry.caveats {
        validate_text("matrix caveat", caveat)?;
    }
    Ok(())
}

fn validate_matrix_exemption(
    exemption: &EvidenceMatrixExemptionInput,
    requirements: &OrderedMap<String, crate::requirement_traceability::RequirementInput>,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_text("matrix exemption requirement", &exemption.requirement_id)?;
    if !requirements.contains_key(&exemption.requirement_id) {
        diagnostics.push_item(format!("stale-exemption-requirement:{}", exemption.requirement_id));
    }
    validate_text("matrix exemption reason", &exemption.reason)?;
    validate_ref_with_diagnostics(&exemption.evidence_ref, "matrix exemption evidence", diagnostics);
    validate_text("matrix exemption scope", &exemption.scope)?;
    validate_text("matrix exemption review note", &exemption.review_note)
}
