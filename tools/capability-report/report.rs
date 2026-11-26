use std::collections::{BTreeMap, BTreeSet};

use super::model::*;

// r[impl molten.testing.capability_qualification.exact_evidence]
// r[impl molten.testing.capability_qualification.diagnostic_output]
pub fn build(matrix: &Matrix, mapping: &Mapping, input: &Input, observed: &BTreeMap<String, Vec<Observation>>) -> Result<Report, String> {
    if matrix.schema != "molten.testing.evidence-matrix.v1" || mapping.schema != "molten.capability-mapping.v1"
        || input.schema != "molten.capability-report.v2" { return Err("unsupported matrix, mapping or input schema".into()); }
    validate_subject(&input.subject)?;
    validate_matrix(matrix)?;
    unique(input.capabilities.iter().map(|entry| entry.id.as_str()), "input capability")?;
    unique(mapping.capabilities.iter().map(|entry| entry.id.as_str()), "mapping capability")?;
    let mut capabilities = Vec::new();
    for selected in &input.capabilities {
        let mut definition = mapping.capabilities.iter().find(|entry| entry.id == selected.id)
            .ok_or_else(|| format!("unknown capability {}", selected.id))?.clone();
        if definition.requirements.is_empty() || definition.families.is_empty() || definition.owners.is_empty() {
            return Err(format!("incomplete mapping {}", selected.id));
        }
        unique(definition.requirements.iter().map(String::as_str), "requirement")?;
        unique(definition.families.iter().map(|family| family.id.as_str()), "family")?;
        definition.requirements.sort(); definition.profiles.sort(); definition.owners.sort();
        definition.families.sort_by(|a, b| a.id.cmp(&b.id));
        for family in &mut definition.families { family.matrix_scopes.sort(); }
        let mut implementation: Vec<_> = matrix.entries.iter().filter(|entry| definition.requirements.contains(&entry.requirement_id)).cloned().collect();
        for entry in &mut implementation { entry.artifact_refs.sort(); entry.caveats.sort(); }
        implementation.sort();
        let mut reviewed_exemptions: Vec<_> = matrix.exemptions.iter().filter(|entry| definition.requirements.contains(&entry.requirement_id)).cloned().collect();
        reviewed_exemptions.sort();
        let mut evidence = observed.get(&selected.id).cloned().unwrap_or_default();
        for entry in &mut evidence {
            if !definition.requirements.contains(&entry.requirement_id) || !definition.families.iter().any(|family| family.id == entry.family)
                || !matches!(entry.coverage_kind.as_str(), "positive" | "negative") {
                return Err(format!("evidence outside reviewed requirement/family mapping for {}", selected.id));
            }
            entry.diagnostics.sort(); entry.diagnostics.dedup();
        }
        evidence.sort();
        let applicable: Vec<_> = definition.families.iter().filter(|family| family.environment == input.subject.environment).collect();
        let mut missing = Vec::new();
        let mut diagnostics = Vec::new();
        for requirement in &definition.requirements {
            for family in &applicable {
                let exempt = reviewed_exemptions.iter().any(|exemption| exemption.requirement_id == *requirement
                    && family.matrix_scopes.contains(&exemption.scope) && super::artifacts::reference(&exemption.evidence_ref)
                    && !exemption.review_note.trim().is_empty() && !exemption.reason_class.trim().is_empty());
                for kind in ["positive", "negative"] {
                    let key = format!("{requirement}/{}/{kind}", family.id);
                    let defined = implementation.iter().any(|entry| entry.requirement_id == *requirement
                        && entry.coverage_kind == kind && family.matrix_scopes.contains(&entry.evidence_scope));
                    let executed = evidence.iter().any(|entry| entry.requirement_id == *requirement && entry.family == family.id
                        && entry.coverage_kind == kind && entry.validated && entry.exact_subject_bound
                        && entry.native_decision == "pass" && !entry.scenario.is_empty());
                    if !defined && !exempt { diagnostics.push(format!("no existing matrix definition for {key}")); }
                    if exempt { diagnostics.push(format!("reviewed matrix exemption retained for {key}; not observed execution")); }
                    else if !executed { missing.push(key); }
                }
            }
        }
        let supported = definition.profiles.contains(&input.subject.execution_profile) && !applicable.is_empty();
        if !supported { diagnostics.push("selected profile/environment is outside the reviewed capability mapping".into()); }
        let failed = evidence.iter().any(|entry| entry.validated && entry.native_decision == "fail");
        let observed_positive = evidence.iter().any(|entry| entry.validated && entry.exact_subject_bound
            && entry.native_decision == "pass" && entry.coverage_kind == "positive");
        let clean = evidence.iter().all(|entry| entry.validated && entry.exact_subject_bound && entry.native_decision == "pass");
        let defined = !diagnostics.iter().any(|diagnostic| diagnostic.starts_with("no existing matrix"));
        let status = status(supported, failed, observed_positive, evidence.is_empty(), missing.is_empty(), clean && defined);
        if !evidence.is_empty() && !observed_positive { diagnostics.push("no exact-subject-bound positive runtime execution supplied".into()); }
        diagnostics.sort(); diagnostics.dedup(); missing.sort(); missing.dedup();
        capabilities.push(CapabilityReport { definition, implementation, reviewed_exemptions,
            execution: Execution { status, evidence, missing, diagnostics } });
    }
    capabilities.sort_by(|a, b| a.definition.id.cmp(&b.definition.id));
    let mut subject = input.subject.clone(); subject.feature_set.sort(); subject.adapter_refs.sort();
    Ok(Report { schema: "molten.capability-qualification.v2", subject, capabilities,
        non_claim: "diagnostic only; no operation authority, release admission, production readiness or live-scope promotion" })
}

pub fn status(supported: bool, failed: bool, observed: bool, empty: bool, complete: bool, clean: bool) -> &'static str {
    if failed { "failed" } else if !supported { "unsupported" }
    else if observed && complete && clean { "bounded-observed" }
    else if empty { "not-recorded" } else { "inconclusive" }
}

pub fn validate_subject(subject: &Subject) -> Result<(), String> {
    if !super::artifacts::reference(&subject.source_ref) || !super::artifacts::reference(&subject.build_ref)
        || !super::artifacts::reference(&subject.dependency_closure_ref) || subject.adapter_refs.is_empty()
        || subject.adapter_refs.iter().any(|value| !super::artifacts::reference(value))
        || subject.execution_profile.trim().is_empty() || subject.platform.trim().is_empty() {
        return Err("exact source/build/closure/adapters/profile/platform must be non-placeholder".into());
    }
    unique(subject.feature_set.iter().map(String::as_str), "feature")?;
    unique(subject.adapter_refs.iter().map(String::as_str), "adapter")?; Ok(())
}

// Placeholder digests and one command aliased across rows are not exact
// requirement-bound evidence; the whole matrix is denied, not silently filtered.
// r[impl molten.testing.evidence_matrix_native_producers]
pub fn validate_matrix(matrix: &Matrix) -> Result<(), String> {
    unique(matrix.entries.iter().map(|entry| entry.command.as_str()), "matrix command (aliased evidence)")?;
    for entry in &matrix.entries {
        for reference in entry.artifact_refs.iter().chain(std::iter::once(&entry.receipt_ref)) {
            if !super::artifacts::reference(reference) || molten::hardening_core::is_placeholder_ref(reference) {
                return Err(format!("placeholder or malformed matrix ref for {}: {reference}", entry.requirement_id));
            }
        }
    }
    for exemption in &matrix.exemptions {
        if !super::artifacts::reference(&exemption.evidence_ref) || molten::hardening_core::is_placeholder_ref(&exemption.evidence_ref) {
            return Err(format!("placeholder or malformed exemption evidence for {}", exemption.requirement_id));
        }
    }
    Ok(())
}

fn unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values { if value.trim().is_empty() || !seen.insert(value) { return Err(format!("empty or duplicate {label}: {value}")); } }
    Ok(())
}

pub fn markdown(report: &Report, report_ref: &str) -> String {
    let mut text = format!("# Capability qualification report\n\nReport: `{report_ref}`\n\nSource: `{}`; build: `{}`; profile: `{}`; platform: `{}`; environment: `{:?}`\n\n| Capability | Implementation | Execution | Missing families |\n|---|---|---|---|\n",
        report.subject.source_ref, report.subject.build_ref, report.subject.execution_profile, report.subject.platform, report.subject.environment);
    for row in &report.capabilities {
        text.push_str(&format!("| {} | {} matrix entries / {} requirements | **{}** | {} |\n", escape(&row.definition.id), row.implementation.len(), row.definition.requirements.len(), row.execution.status, row.execution.missing.len()));
    }
    for row in &report.capabilities {
        for evidence in &row.execution.evidence {
            text.push_str(&format!("\n- {} / {} / `{}`: **{}** (native {}), validated={}, exact-subject={}; scenario: {}\n", escape(&row.definition.id), escape(&evidence.family), evidence.artifact_ref, evidence.native_decision, evidence.canonical_decision, evidence.validated, evidence.exact_subject_bound, escape(&evidence.scenario)));
            for diagnostic in &evidence.diagnostics { text.push_str(&format!("  - {}\n", escape(diagnostic))); }
        }
        for diagnostic in &row.execution.diagnostics { text.push_str(&format!("\n- {}: {}\n", escape(&row.definition.id), escape(diagnostic))); }
        for missing in &row.execution.missing { text.push_str(&format!("\n- Missing: `{}`\n", escape(missing))); }
    }
    text.push_str(&format!("\n_{}_\n", report.non_claim)); text
}

fn escape(text: &str) -> String { text.replace('|', "&#124;").replace(['\n', '\r', '`'], " ") }
