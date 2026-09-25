// Root-only ledger GC that enforces retention authorization before removal.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gc {
    pub dry_run: bool,
    pub decision: String,
    pub removed_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, Copy)]
pub struct GcInput<'a> {
    pub dry_run: bool,
    pub retention_evidence: &'a crate::retention::DestructiveEvidence,
    pub apply_refs: &'a [String],
}

pub fn gc(root: &std::path::Path, input: GcInput<'_>) -> crate::error::Result<Gc> {
    ensure_dirs(root)?;
    let pins = pinned_refs(root)?;
    let candidates = scan_unpinned(root, &pins)?;
    let action = action_for(input.dry_run);
    let requester_ref =
        crate::retention::destructive_requester_ref(input.retention_evidence, "ledger-gc-missing-requester")?;
    let evidence_summary = crate::retention::destructive_evidence_value(input.retention_evidence)?;
    let review = review_entries(
        ReviewInput {
            root,
            source: input,
            action,
            requester_ref: &requester_ref,
        },
        &candidates,
    )?;
    let decision = decision_for(&review.denied_refs);
    let removed_refs = remove_entries(root, &candidates, input.dry_run, decision)?;
    let receipt_value = outcome_value(OutcomeInput {
        is_dry_run: input.dry_run,
        decision,
        removed_refs: &removed_refs,
        evidence_summary,
        review: &review,
    });
    Ok(Gc {
        dry_run: input.dry_run,
        decision: decision.to_string(),
        removed_refs,
        retention_receipt_refs: review.retention_receipt_refs,
        execution_gate_refs: review.execution_gate_refs,
        receipt_value,
    })
}

fn scan_unpinned(root: &std::path::Path, pins: &[String]) -> crate::error::Result<Vec<Entry>> {
    let mut candidates = Vec::new();
    for entry in list_artifacts(root)? {
        if pins.iter().any(|pin| pin == &entry.artifact_ref) {
            continue;
        }
        push_bounded(&mut candidates, entry, MAX_SCAN_ENTRIES, "ledger gc candidates")?;
    }
    Ok(candidates)
}

fn action_for(is_dry_run: bool) -> &'static str {
    if is_dry_run {
        crate::retention::ACTION_ELIGIBILITY
    } else {
        crate::retention::ACTION_DELETE
    }
}

#[derive(Clone, Copy)]
struct ReviewInput<'a> {
    root: &'a std::path::Path,
    source: GcInput<'a>,
    action: &'a str,
    requester_ref: &'a str,
}

#[derive(Default)]
struct Review {
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    retention_receipt_refs: Vec<String>,
    execution_gate_refs: Vec<String>,
    denied_refs: Vec<String>,
}

fn review_entries(input: ReviewInput<'_>, candidates: &[Entry]) -> crate::error::Result<Review> {
    let mut review = Review::default();
    for entry in candidates {
        let retention_class = retention_class(&entry.artifact_kind);
        let admission = crate::retention::admit_destructive_evidence(crate::retention::DestructiveAdmissionInput {
            root: input.root,
            evidence: input.source.retention_evidence,
            object_ref: &entry.artifact_ref,
            object_kind: &entry.artifact_kind,
            retention_class,
            action: input.action,
        })?;
        extend_refs(
            &mut review.admission_diagnostics,
            &admission.diagnostics,
            "ledger retention admission diagnostics",
        )?;
        extend_refs(&mut review.admission_refs, &admission.admitted_refs, "ledger retention admission refs")?;
        let evaluation = crate::retention::evaluate(crate::retention::EvaluationInput {
            root: input.root,
            object_ref: &entry.artifact_ref,
            object_kind: &entry.artifact_kind,
            retention_class,
            action: input.action,
            requester_ref: input.requester_ref,
            is_reference_index_complete: input.source.retention_evidence.is_reference_index_complete,
            retained_refs: &input.source.retention_evidence.retained_refs,
            remote_refs: &input.source.retention_evidence.remote_refs,
            policy_refs: &input.source.retention_evidence.policy_refs,
            evidence_refs: &input.source.retention_evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })?;
        push_bounded(
            &mut review.retention_receipt_refs,
            evaluation.receipt.receipt_ref.clone(),
            MAX_SCAN_ENTRIES,
            "ledger retention receipt refs",
        )?;
        let is_execution_denied = record_execution(input, entry, retention_class, &mut review)?;
        if admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_execution_denied {
            push_bounded(
                &mut review.denied_refs,
                entry.artifact_ref.clone(),
                MAX_SCAN_ENTRIES,
                "ledger retention denials",
            )?;
        }
    }
    Ok(review)
}

fn extend_refs(
    target: &mut impl crate::bounded::VecSink<String>,
    values: &[String],
    label: &str,
) -> crate::error::Result<()> {
    for value in values {
        push_bounded(target, value.clone(), MAX_SCAN_ENTRIES, label)?;
    }
    Ok(())
}

fn record_execution(
    input: ReviewInput<'_>,
    entry: &Entry,
    retention_class: &str,
    review: &mut Review,
) -> crate::error::Result<bool> {
    if input.source.dry_run {
        return Ok(false);
    }
    let apply_ref = matching_apply_ref(ApplyRefMatchInput {
        root: input.root,
        apply_refs: input.source.apply_refs,
        subsystem: "ledger-gc",
        action: input.action,
        object_ref: &entry.artifact_ref,
        object_kind: &entry.artifact_kind,
        retention_class,
    });
    let execution_gate = crate::retention::store_gc_execution_gate(crate::retention::GcExecutionGateInput {
        root: input.root,
        subsystem: "ledger-gc",
        action: input.action,
        object_ref: &entry.artifact_ref,
        object_kind: &entry.artifact_kind,
        retention_class,
        apply_ref,
    })?;
    push_bounded(
        &mut review.execution_gate_refs,
        execution_gate.execution_ref.clone(),
        MAX_SCAN_ENTRIES,
        "ledger retention execution gate refs",
    )?;
    if execution_gate.decision == "pass" {
        return Ok(false);
    }
    extend_refs(
        &mut review.execution_diagnostics,
        &execution_gate.diagnostics,
        "ledger retention execution diagnostics",
    )?;
    Ok(true)
}

fn decision_for(denied_refs: &[String]) -> &'static str {
    if denied_refs.is_empty() { "pass" } else { "deny" }
}

fn remove_entries(
    root: &std::path::Path,
    candidates: &[Entry],
    is_dry_run: bool,
    decision: &str,
) -> crate::error::Result<Vec<String>> {
    let mut removed_refs = Vec::new();
    if decision == "pass" {
        for entry in candidates {
            push_bounded(&mut removed_refs, entry.artifact_ref.clone(), MAX_SCAN_ENTRIES, "ledger removed refs")?;
            if !is_dry_run {
                std::fs::remove_file(content_path(root, &entry.artifact_ref)?).map_err(crate::error::Failure::from)?;
            }
        }
    }
    Ok(removed_refs)
}

struct OutcomeInput<'a> {
    is_dry_run: bool,
    decision: &'a str,
    removed_refs: &'a [String],
    evidence_summary: preserves::IOValue,
    review: &'a Review,
}

fn outcome_value(input: OutcomeInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("ledger-gc-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::EVIDENCE_LEDGER_GC_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(mode_for(input.is_dry_run))]),
        crate::preserves_rail::record("removed", vec![crate::preserves_rail::sequence(
            input.removed_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention", vec![crate::preserves_rail::sequence(
            input.review.retention_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-execution", vec![crate::preserves_rail::sequence(
            input.review.execution_gate_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("denied", vec![crate::preserves_rail::sequence(
            input.review.denied_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-evidence", vec![input.evidence_summary]),
        crate::preserves_rail::record("retention-admission", vec![crate::preserves_rail::sequence(
            input.review.admission_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-diagnostics", vec![crate::preserves_rail::sequence(
            input.review.admission_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("retention-execution-diagnostics", vec![crate::preserves_rail::sequence(
            input.review.execution_diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![outcome_checks(input.is_dry_run, input.decision, input.review)]),
    ])
}

fn mode_for(is_dry_run: bool) -> &'static str {
    if is_dry_run { "dry-run" } else { "apply" }
}

fn outcome_checks(is_dry_run: bool, decision: &str, review: &Review) -> preserves::IOValue {
    crate::preserves_rail::sequence(vec![
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("pin-preservation"),
            crate::preserves_rail::string("pass"),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("derived-index-scan"),
            crate::preserves_rail::string("pass"),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("retention-receipt-bound"),
            crate::preserves_rail::string("pass"),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("retention-execution-gate"),
            crate::preserves_rail::string(pass_or_fail(is_dry_run || review.execution_diagnostics.is_empty())),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("retention-authority-evidence"),
            crate::preserves_rail::string(pass_or_fail(review.admission_diagnostics.is_empty())),
        ]),
        crate::preserves_rail::record("check", vec![
            crate::preserves_rail::string("deny-before-removal"),
            crate::preserves_rail::string(if decision == "pass" { "pass" } else { "fail" }),
        ]),
    ])
}

fn pass_or_fail(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

struct ApplyRefMatchInput<'a> {
    root: &'a std::path::Path,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

fn matching_apply_ref<'a>(input: ApplyRefMatchInput<'a>) -> Option<&'a str> {
    let mut fallback_ref = None;
    for apply_ref in input.apply_refs {
        let Ok(apply) = crate::retention::read_gc_apply(input.root, apply_ref) else {
            if fallback_ref.is_none() {
                fallback_ref = Some(apply_ref.as_str());
            }
            continue;
        };
        if apply.decision == "pass"
            && apply.subsystem == input.subsystem
            && apply.action == input.action
            && apply.object_ref == input.object_ref
            && apply.object_kind == input.object_kind
            && apply.retention_class == input.retention_class
        {
            return Some(apply_ref.as_str());
        }
        if fallback_ref.is_none() {
            fallback_ref = Some(apply_ref.as_str());
        }
    }
    fallback_ref
}

fn retention_class(artifact_kind: &str) -> &'static str {
    if artifact_kind.contains("secret") || artifact_kind.contains("encrypted") || artifact_kind.contains("redaction") {
        crate::retention::CLASS_PRIVATE_SECRET_REF
    } else if artifact_kind.contains("cache") {
        crate::retention::CLASS_EPHEMERAL_CACHE
    } else if artifact_kind.contains("artifact") || artifact_kind.contains("manifest") {
        crate::retention::CLASS_PUBLIC_ARTIFACT
    } else {
        crate::retention::CLASS_AUDIT_RECEIPT
    }
}

fn pinned_refs(root: &std::path::Path) -> crate::error::Result<Vec<String>> {
    let pins = root.join("pins");
    if !pins.exists() {
        return Ok(Vec::new());
    }
    let mut refs = Vec::new();
    for entry in std::fs::read_dir(pins).map_err(crate::error::Failure::from)? {
        let entry = entry.map_err(crate::error::Failure::from)?;
        if entry.file_type().map_err(crate::error::Failure::from)?.is_file() {
            let reference = std::fs::read_to_string(entry.path()).map_err(crate::error::Failure::from)?;
            crate::preserves_rail::validate_content_ref(&reference).map_err(|error| {
                crate::error::Failure::invalid_harness(format!(
                    "ledger pin file contains invalid content ref {reference}: {error}"
                ))
            })?;
            push_bounded(&mut refs, reference, MAX_SCAN_ENTRIES, "ledger pinned refs")?;
        }
    }
    Ok(refs)
}
fn content_path(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<std::path::PathBuf> {
    Ok(root.join("content").join(filename_for_ref(artifact_ref)?))
}
