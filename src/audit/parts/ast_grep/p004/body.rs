// Receipt checks over one supplied structural audit scan and the bytes it measured.

/// The profile, tool, identity, finding, and non-claim checks of one structural audit scan, in
/// receipt order. Every check passes exactly when it produced no diagnostics.
fn audit_checks(input: &ScanInput, measurement: &ScanMeasurement<'_>) -> Vec<ReceiptCheck> {
    // r[impl molten.evidence_gates.static_audit_content_bound_hashes]
    let mut checks = identity_checks(input, measurement);
    checks.extend(finding_checks(input, measurement));
    let mut missing_non_claims = Vec::with_capacity(REQUIRED_NON_CLAIMS.len());
    require_items(
        "non-claim",
        REQUIRED_NON_CLAIMS,
        input.profile.non_claims.iter().map(String::as_str),
        &mut missing_non_claims,
    );
    checks.push(check(
        "non-claims-bound",
        missing_non_claims,
        "receipt binds authority, replay, sealed-repro, UCAN, distributed-safety, and release non-claims",
    ));
    checks
}

/// Profile validity, the tool identity, and the rule-bundle and scan-scope identities recomputed from the
/// measured rule and source bytes, plus the run identity.
fn identity_checks(input: &ScanInput, measurement: &ScanMeasurement<'_>) -> Vec<ReceiptCheck> {
    vec![
        check(
            "ast-grep-profile-valid",
            validate_profile(&input.profile).diagnostics,
            "profile satisfies runtime-authority audit contract",
        ),
        check(
            "ast-grep-tool-bound",
            violation(
                !input.ast_grep_version.starts_with(AST_GREP_TOOL_PREFIX),
                format!("tool version {:?} must start with {AST_GREP_TOOL_PREFIX:?}", input.ast_grep_version),
            ),
            "ast-grep tool version is bound",
        ),
        check(
            "rule-bundle-blake3-bound",
            measured_identity_diagnostics(
                "rule bundle identity",
                &input.rule_bundle_hash,
                rule_bundle_hash(&input.profile, measurement.rule_sources),
            ),
            "rule bundle identity equals the BLAKE3 identity of the measured rule YAML bytes",
        ),
        check(
            "scan-scope-blake3-bound",
            measured_identity_diagnostics(
                "scan scope identity",
                &input.scan_scope_hash,
                scan_scope_hash(measurement.scanned_sources),
            ),
            "scan scope identity equals the BLAKE3 identity of the measured scanned sources",
        ),
        check(
            "evidence-gate-run-bound",
            violation(
                !is_content_ref(&input.evidence_gate_run_ref),
                format!("evidence-gate run identity {:?} is not a content ref", input.evidence_gate_run_ref),
            ),
            "runtime or evidence-gate run identity is a content ref",
        ),
    ]
}

fn measured_identity_diagnostics(label: &str, supplied: &str, measured: crate::error::Result<String>) -> Vec<String> {
    match measured {
        Err(error) => vec![format!("{label} {supplied:?} cannot be measured: {error}")],
        Ok(measured) if measured != supplied => {
            vec![format!("{label} {supplied:?} does not match measured {measured}")]
        }
        Ok(_) => Vec::new(),
    }
}

/// Known-rule, blocking-rule, and finding-shape checks over the supplied findings.
fn finding_checks(input: &ScanInput, measurement: &ScanMeasurement<'_>) -> Vec<ReceiptCheck> {
    let rules_by_id =
        input.profile.rules.iter().map(|rule| (rule.id.as_str(), rule)).collect::<std::collections::BTreeMap<_, _>>();
    let surface_ids =
        input.profile.surfaces.iter().map(|surface| surface.id.as_str()).collect::<std::collections::BTreeSet<_>>();
    let scanned_paths =
        measurement.scanned_sources.iter().map(|source| source.path).collect::<std::collections::BTreeSet<_>>();
    let unknown_rule_diagnostics = input
        .findings
        .iter()
        .filter(|finding| !rules_by_id.contains_key(finding.rule_id.as_str()))
        .map(|finding| format!("finding at {} references undeclared rule {}", finding.path, finding.rule_id))
        .collect();
    let blocking_diagnostics = input
        .findings
        .iter()
        .filter(|finding| {
            rules_by_id.get(finding.rule_id.as_str()).is_some_and(|rule| rule.posture == RulePosture::Blocking)
        })
        .map(|finding| format!("blocking rule {} matched {}", finding.rule_id, finding.path))
        .collect();
    let candidate_diagnostics = input
        .findings
        .iter()
        .flat_map(|finding| finding_shape_diagnostics(finding, &surface_ids, &scanned_paths))
        .collect();
    vec![
        check(
            "findings-reference-known-rules",
            unknown_rule_diagnostics,
            "all structural findings reference declared rules",
        ),
        check(
            "blocking-findings-absent",
            blocking_diagnostics,
            "blocking structural rules have no findings in converted scopes",
        ),
        check(
            "findings-are-structural-candidates",
            candidate_diagnostics,
            "findings name declared surfaces and scanned repository-relative paths; they are candidate structural \
             evidence only",
        ),
    ]
}

fn finding_shape_diagnostics(
    finding: &Finding,
    surface_ids: &std::collections::BTreeSet<&str>,
    scanned_paths: &std::collections::BTreeSet<&str>,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(FINDING_SHAPE_DIAGNOSTICS_MAX);
    if !surface_ids.contains(finding.surface.as_str()) {
        diagnostics.push(format!(
            "finding {} at {} names undeclared surface {}",
            finding.rule_id, finding.path, finding.surface
        ));
    }
    if !is_repository_relative_path(&finding.path) {
        diagnostics.push(format!("finding {} path {:?} is not repository-relative", finding.rule_id, finding.path));
    } else if !scanned_paths.contains(finding.path.as_str()) {
        diagnostics.push(format!("finding {} path {} is outside the measured scan", finding.rule_id, finding.path));
    }
    if finding.message.trim().is_empty() {
        diagnostics.push(format!("finding {} at {} has no message", finding.rule_id, finding.path));
    }
    diagnostics
}

fn violation(is_violated: bool, diagnostic: String) -> Vec<String> {
    if is_violated { vec![diagnostic] } else { Vec::new() }
}
