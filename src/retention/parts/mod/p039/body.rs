
fn candidate_bundle_expected_refs(bundle: &CandidateBundle) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_ref_slice(&mut refs, &bundle.gc_plan_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_apply_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_execution_refs)?;
    push_ref_slice(&mut refs, &bundle.gc_audit_refs)?;
    push_ref_slice(&mut refs, &bundle.retention_receipt_refs)?;
    push_ref_slice(&mut refs, &bundle.tombstone_refs)?;
    Ok(refs)
}

fn push_ref_slice(values: &mut impl VecSink<String>, refs: &[String]) -> Result<()> {
    for reference in refs {
        push_bounded(values, reference.clone(), MAX_RETENTION_REFS, "retention bundle expected refs")?;
    }
    Ok(())
}
