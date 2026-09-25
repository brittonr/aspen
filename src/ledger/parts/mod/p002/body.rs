
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
