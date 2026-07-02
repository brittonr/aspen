const MAX_SCAN_ENTRIES: usize = 100_000;
const _: () = assert!(MAX_SCAN_ENTRIES > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub artifact_ref: String,
    pub artifact_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub artifact_ref: String,
    pub artifact_kind: String,
    pub receipt_value: preserves::IOValue,
}

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

pub fn import_artifact(root: &std::path::Path, artifact: &preserves::IOValue) -> crate::error::Result<Import> {
    ensure_dirs(root)?;
    let artifact_ref = crate::preserves_rail::canonical_hash(artifact)?;
    let artifact_kind = artifact_kind(artifact).to_string();
    let bytes = crate::preserves_rail::canonical_bytes(artifact)?;
    let path = content_path(root, &artifact_ref)?;
    if path.exists() {
        let existing = std::fs::read(&path).map_err(crate::error::MoltenError::from)?;
        let existing_value = crate::preserves_rail::parse_canonical_bytes(&existing)?;
        let existing_ref = crate::preserves_rail::canonical_hash(&existing_value)?;
        if existing_ref != artifact_ref {
            return Err(crate::error::MoltenError::invalid_harness(format!(
                "ledger content path for {artifact_ref} contains corrupted bytes hashing to {existing_ref}"
            )));
        }
    } else {
        std::fs::write(&path, bytes).map_err(crate::error::MoltenError::from)?;
    }
    let receipt_value = import_receipt_value(&artifact_ref, &artifact_kind);
    Ok(Import {
        artifact_ref,
        artifact_kind,
        receipt_value,
    })
}

pub fn export_artifact(
    root: &std::path::Path,
    artifact_ref: &str,
    out: &std::path::Path,
) -> crate::error::Result<Export> {
    let artifact = read_artifact(root, artifact_ref)?;
    let artifact_kind = artifact_kind(&artifact).to_string();
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(crate::error::MoltenError::from)?;
    }
    std::fs::write(out, crate::preserves_rail::to_text(&artifact)?).map_err(crate::error::MoltenError::from)?;
    let receipt_value = crate::preserves_rail::record("ledger-export-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::EVIDENCE_LEDGER_EXPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("artifact-kind", vec![crate::preserves_rail::string(&artifact_kind)]),
        crate::preserves_rail::record("artifact", vec![crate::preserves_rail::string(artifact_ref)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("content-ref-found"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-export"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]);
    Ok(Export {
        artifact_ref: artifact_ref.to_string(),
        artifact_kind,
        receipt_value,
    })
}

pub fn read_artifact(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<preserves::IOValue> {
    let path = content_path(root, artifact_ref)?;
    let bytes = std::fs::read(&path).map_err(crate::error::MoltenError::from)?;
    let value = crate::preserves_rail::parse_canonical_bytes(&bytes)?;
    let actual_ref = crate::preserves_rail::canonical_hash(&value)?;
    if actual_ref != artifact_ref {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "ledger content hash mismatch: got {actual_ref}, expected {artifact_ref}"
        )));
    }
    Ok(value)
}

pub fn list_artifacts(root: &std::path::Path) -> crate::error::Result<Vec<Entry>> {
    let content = root.join("content");
    if !content.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(content).map_err(crate::error::MoltenError::from)? {
        let entry = entry.map_err(crate::error::MoltenError::from)?;
        if !entry.file_type().map_err(crate::error::MoltenError::from)?.is_file() {
            continue;
        }
        let Some(artifact_ref) = ref_from_filename(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        let value = read_artifact(root, &artifact_ref)?;
        push_bounded(
            &mut entries,
            Entry {
                artifact_ref,
                artifact_kind: artifact_kind(&value).to_string(),
            },
            MAX_SCAN_ENTRIES,
            "ledger artifact entries",
        )?;
    }
    entries.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(entries)
}

pub fn pin_artifact(root: &std::path::Path, artifact_ref: &str) -> crate::error::Result<()> {
    ensure_dirs(root)?;
    read_artifact(root, artifact_ref)?;
    std::fs::write(pin_path(root, artifact_ref)?, artifact_ref).map_err(crate::error::MoltenError::from)
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
