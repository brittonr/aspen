
pub fn boundary_negative_suite_value(input: &BoundaryNegativeSuiteInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("boundary negative suite name", input.suite_name)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("Preserves parser", input.preserves_parser_refs),
        ("receipt validator", input.receipt_validator_refs),
        ("source gate", input.source_gate_refs),
        ("repro bundle", input.repro_bundle_refs),
        ("node ingress", input.node_ingress_refs),
        ("provenance", input.provenance_refs),
        ("plugin hostcall", input.plugin_hostcall_refs),
        ("malformed denial", input.malformed_denial_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-security-boundary-negative-suite-v1", vec![
        string(PROD_SECURITY_BOUNDARY_NEGATIVE_SUITE_SCHEMA),
        decision_field(input.decision),
        record("suite", vec![string(input.suite_name)]),
        refs_field("preserves-parsers", input.preserves_parser_refs)?,
        refs_field("receipt-validators", input.receipt_validator_refs)?,
        refs_field("source-gates", input.source_gate_refs)?,
        refs_field("repro-bundles", input.repro_bundle_refs)?,
        refs_field("node-ingress", input.node_ingress_refs)?,
        refs_field("provenance", input.provenance_refs)?,
        refs_field("plugin-hostcalls", input.plugin_hostcall_refs)?,
        refs_field("malformed-denials", input.malformed_denial_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("parser-failures-structured", pass_check(input.preserves_parser_refs.is_empty())),
            check_value("receipt-validator-boundaries-covered", pass_check(input.receipt_validator_refs.is_empty())),
            check_value(
                "malformed-input-denies-not-missing-clean-evidence",
                pass_check(input.malformed_denial_refs.is_empty()),
            ),
        ]),
    ]))
}

pub fn incident_response_drill_value(input: &IncidentResponseDrillInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_allowed_text("incident kind", input.incident_kind, INCIDENT_KINDS)?;
    validate_text_field("incident response scenario", input.scenario)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("detection", input.detection_refs),
        ("containment", input.containment_refs),
        ("recovery", input.recovery_refs),
        ("next step", input.next_step_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    Ok(record("prod-security-incident-response-drill-v1", vec![
        string(PROD_SECURITY_DRILL_SCHEMA),
        decision_field(input.decision),
        record("incident-kind", vec![string(input.incident_kind)]),
        record("scenario", vec![string(input.scenario)]),
        refs_field("detection", input.detection_refs)?,
        refs_field("containment", input.containment_refs)?,
        refs_field("recovery", input.recovery_refs)?,
        refs_field("next-steps", input.next_step_refs)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("incident-detected", pass_check(input.detection_refs.is_empty())),
            check_value("containment-bound", pass_check(input.containment_refs.is_empty())),
            check_value(
                "recovery-next-steps-bound",
                pass_check(input.recovery_refs.is_empty() || input.next_step_refs.is_empty()),
            ),
        ]),
    ]))
}

pub fn security_readiness_report_value(input: &SecurityReadinessReportInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("security readiness report name", input.report_name)?;
    validate_text_field("pilot recommendation", input.pilot_recommendation)?;
    validate_diagnostics(input.diagnostics)?;
    for (label, refs) in [
        ("threat model", input.threat_model_refs),
        ("supply chain", input.supply_chain_refs),
        ("drill", input.drill_refs),
        ("redaction audit", input.redaction_audit_refs),
        ("boundary suite", input.boundary_suite_refs),
        ("incident response", input.incident_response_refs),
    ] {
        require_pass_refs(label, refs, input.decision)?;
    }
    if is_pass(input.decision)
        && !input.unresolved_risk_refs.is_empty()
        && input.pilot_recommendation == BROAD_PRODUCTION_SCOPE
    {
        return Err(MoltenError::invalid_harness(
            "security readiness with unresolved risks cannot recommend broad production",
        ));
    }
    Ok(record("prod-security-readiness-report-v1", vec![
        string(PROD_SECURITY_READINESS_REPORT_SCHEMA),
        decision_field(input.decision),
        record("report", vec![string(input.report_name)]),
        refs_field("threat-models", input.threat_model_refs)?,
        refs_field("supply-chain", input.supply_chain_refs)?,
        refs_field("drills", input.drill_refs)?,
        refs_field("redaction-audits", input.redaction_audit_refs)?,
        refs_field("boundary-suites", input.boundary_suite_refs)?,
        refs_field("incident-response", input.incident_response_refs)?,
        refs_field("unresolved-risks", input.unresolved_risk_refs)?,
        record("pilot-recommendation", vec![string(input.pilot_recommendation)]),
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("threat-model-bound", pass_check(input.threat_model_refs.is_empty())),
            check_value(
                "drills-and-negative-suites-bound",
                pass_check(input.drill_refs.is_empty() || input.boundary_suite_refs.is_empty()),
            ),
            check_value("pilot-scope-recommendation-explicit", "pass"),
        ]),
    ]))
}

pub fn pilot_decision_value(input: &PilotDecisionInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("pilot scope", input.scope)?;
    validate_text_slice("allowed workload", input.allowed_workloads)?;
    validate_text_slice("denied workload", input.denied_workloads)?;
    validate_text_slice("rollback trigger", input.rollback_triggers)?;
    validate_text_slice("stop condition", input.stop_conditions)?;
    validate_text_slice("pilot caveat", input.caveats)?;
    validate_diagnostics(input.diagnostics)?;
    require_pass_texts("allowed workload", input.allowed_workloads, input.decision)?;
    require_pass_texts("denied workload", input.denied_workloads, input.decision)?;
    require_pass_texts("rollback trigger", input.rollback_triggers, input.decision)?;
    require_pass_texts("stop condition", input.stop_conditions, input.decision)?;
    require_pass_refs("operator review", input.operator_review_refs, input.decision)?;
    if is_pass(input.decision) && input.scope == BROAD_PRODUCTION_SCOPE && !input.caveats.is_empty() {
        return Err(MoltenError::invalid_harness(
            "pilot decision with evidence-only caveats cannot claim broad production scope",
        ));
    }
    Ok(record("prod-release-pilot-decision-v1", vec![
        string(PROD_RELEASE_PILOT_DECISION_SCHEMA),
        decision_field(input.decision),
        record("scope", vec![string(input.scope)]),
        texts_field("allowed-workloads", input.allowed_workloads)?,
        texts_field("denied-workloads", input.denied_workloads)?,
        texts_field("rollback-triggers", input.rollback_triggers)?,
        texts_field("stop-conditions", input.stop_conditions)?,
        refs_field("operator-review", input.operator_review_refs)?,
        texts_field("caveats", input.caveats)?,
        diagnostics_field(input.diagnostics)?,
        checks_field(vec![
            check_value("allowed-workloads-explicit", pass_check(input.allowed_workloads.is_empty())),
            check_value("denied-workloads-explicit", pass_check(input.denied_workloads.is_empty())),
            check_value(
                "rollback-and-stop-conditions-explicit",
                pass_check(input.rollback_triggers.is_empty() || input.stop_conditions.is_empty()),
            ),
            check_value("operator-review-bound", pass_check(input.operator_review_refs.is_empty())),
        ]),
    ]))
}

struct ReleaseCandidateGate<'a> {
    input: &'a ReleaseCandidateGateInput<'a>,
}

impl<'a> ReleaseCandidateGate<'a> {
    fn new(input: &'a ReleaseCandidateGateInput<'a>) -> Self {
        Self { input }
    }

    fn validate(&self) -> Result<()> {
        validate_decision(self.input.decision)?;
        validate_text_field("candidate", self.input.candidate)?;
        validate_content_ref(self.input.source_ref)?;
        validate_source_gate_status(self.input.source_gate_status)?;
        validate_text_slice("source gate caveat", self.input.source_gate_caveats)?;
        validate_diagnostics(self.input.diagnostics)?;
        self.require_candidate_bound_evidence()?;
        self.require_source_gate_caveat()
    }

    fn evidence_groups(&self) -> [(&'static str, &'a [CandidateEvidenceBinding<'a>]); 10] {
        [
            ("Rust validation", self.input.rust_validation_evidence),
            ("nextest", self.input.nextest_evidence),
            ("Nix check", self.input.nix_check_evidence),
            ("Cairn validation", self.input.cairn_validation_evidence),
            ("Octet", self.input.octet_evidence),
            ("dogfood", self.input.dogfood_evidence),
            ("release bundle verify", self.input.bundle_verify_evidence),
            ("promotion", self.input.promotion_evidence),
            ("export verify", self.input.export_verify_evidence),
            ("pilot decision", self.input.pilot_decision_evidence),
        ]
    }

    fn require_candidate_bound_evidence(&self) -> Result<()> {
        for (label, bindings) in self.evidence_groups() {
            validate_candidate_evidence_bindings(
                label,
                bindings,
                self.input.source_ref,
                self.input.decision,
            )?;
        }
        Ok(())
    }

    fn require_source_gate_caveat(&self) -> Result<()> {
        if is_pass(self.input.decision)
            && self.input.source_gate_status != SOURCE_REMEDIATED_ZERO_STATUS
            && self.input.source_gate_caveats.is_empty()
        {
            return Err(MoltenError::invalid_harness(
                "passing production candidate with non-zero source gate status requires source gate caveats",
            ));
        }
        Ok(())
    }

    fn value(&self) -> Result<IoValue> {
        Ok(record("prod-release-candidate-gate-v2", vec![
            string(PROD_RELEASE_CANDIDATE_GATE_SCHEMA),
            decision_field(self.input.decision),
            record("candidate", vec![string(self.input.candidate)]),
            record("source", vec![string(self.input.source_ref)]),
            candidate_evidence_field("rust-validation", self.input.rust_validation_evidence),
            candidate_evidence_field("nextest", self.input.nextest_evidence),
            candidate_evidence_field("nix-checks", self.input.nix_check_evidence),
            candidate_evidence_field("cairn-validation", self.input.cairn_validation_evidence),
            candidate_evidence_field("octet-source-gates", self.input.octet_evidence),
            candidate_evidence_field("dogfood", self.input.dogfood_evidence),
            candidate_evidence_field("release-bundle-verification", self.input.bundle_verify_evidence),
            candidate_evidence_field("promotion", self.input.promotion_evidence),
            candidate_evidence_field("export-verification", self.input.export_verify_evidence),
            record("source-gate-status", vec![string(self.input.source_gate_status)]),
            texts_field("source-gate-caveats", self.input.source_gate_caveats)?,
            candidate_evidence_field("pilot-decisions", self.input.pilot_decision_evidence),
            diagnostics_field(self.input.diagnostics)?,
            checks_field(self.checks()),
        ]))
    }

    fn checks(&self) -> Vec<IoValue> {
        vec![
            check_value("full-validation-matrix-bound", pass_check(self.has_validation_matrix_gap())),
            check_value("all-evidence-candidate-bound", "pass"),
            check_value("source-gate-current-or-limited", pass_check(self.has_source_gate_limiter())),
            check_value("bundle-promotion-export-bound", pass_check(self.has_release_bundle_gap())),
            check_value("pilot-decision-bound", pass_check(self.input.pilot_decision_evidence.is_empty())),
            check_value("declared-binding-does-not-prove-external-artifact-truth", "pass"),
            check_value("release-candidate-receipt-does-not-grant-authority", "pass"),
        ]
    }

    fn has_validation_matrix_gap(&self) -> bool {
        self.input.rust_validation_evidence.is_empty()
            || self.input.nextest_evidence.is_empty()
            || self.input.nix_check_evidence.is_empty()
            || self.input.cairn_validation_evidence.is_empty()
    }

    fn has_source_gate_limiter(&self) -> bool {
        self.input.source_gate_status != SOURCE_REMEDIATED_ZERO_STATUS && self.input.source_gate_caveats.is_empty()
    }

    fn has_release_bundle_gap(&self) -> bool {
        self.input.bundle_verify_evidence.is_empty()
            || self.input.promotion_evidence.is_empty()
            || self.input.export_verify_evidence.is_empty()
    }
}

fn validate_candidate_evidence_bindings(
    label: &'static str,
    bindings: &[CandidateEvidenceBinding<'_>],
    expected_source_ref: &str,
    decision: &str,
) -> Result<()> {
    if bindings.len() > MAX_PROD_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "production readiness {label} binding count {} exceeds bound {MAX_PROD_REFS}",
            bindings.len()
        )));
    }
    if is_pass(decision) && bindings.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "passing production readiness receipt requires at least one {label} candidate evidence binding"
        )));
    }
    for binding in bindings {
        validate_content_ref(binding.artifact_ref).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "invalid production readiness {label} artifact ref {}: {error}",
                binding.artifact_ref
            ))
        })?;
        validate_content_ref(binding.source_ref).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "invalid production readiness {label} candidate source ref {}: {error}",
                binding.source_ref
            ))
        })?;
        if binding.source_ref != expected_source_ref {
            return Err(MoltenError::invalid_harness(format!(
                "production readiness {label} candidate source mismatch: expected {expected_source_ref}, observed {}",
                binding.source_ref
            )));
        }
    }
    Ok(())
}

fn candidate_evidence_field(label: &'static str, bindings: &[CandidateEvidenceBinding<'_>]) -> IoValue {
    let values = bindings
        .iter()
        .map(|binding| {
            record(
                "candidate-evidence",
                vec![string(binding.artifact_ref), string(binding.source_ref)],
            )
        })
        .collect();
    record(label, vec![sequence(values)])
}

pub fn release_candidate_gate_value(input: &ReleaseCandidateGateInput<'_>) -> Result<IoValue> {
    let gate = ReleaseCandidateGate::new(input);
    gate.validate()?;
    gate.value()
}

fn decision_field(decision: &str) -> IoValue {
    record("decision", vec![string(decision)])
}

fn diagnostics_field(values: &[String]) -> Result<IoValue> {
    Ok(record("diagnostics", vec![sequence(string_values("diagnostic", values)?)]))
}

fn refs_field(label: &'static str, refs: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(ref_values(label, refs)?)]))
}

fn texts_field(label: &'static str, values: &[String]) -> Result<IoValue> {
    Ok(record(label, vec![sequence(string_values(label, values)?)]))
}

fn checks_field(checks: Vec<IoValue>) -> IoValue {
    record("checks", vec![sequence(checks)])
}

fn check_value(name: &'static str, status: &'static str) -> IoValue {
    record("check", vec![string(name), string(status)])
}

fn pass_check(is_failed: bool) -> &'static str {
    if is_failed { "deny" } else { "pass" }
}
