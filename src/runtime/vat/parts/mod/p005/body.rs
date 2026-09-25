
enum DistCase {
    Live,
    Disconnected,
    Handoff,
    StaleUse,
    PendingOpen,
}

fn dist_refs() -> Result<DistRefs> {
    let far = VatObjectRef::new(REMOTE_VAT_ID, FAR_OBJECT_ID, VatReferenceKind::Far, Vec::new());
    let replacement = VatObjectRef::new(REMOTE_VAT_ID, "object:remote:handoff", VatReferenceKind::Far, Vec::new());
    Ok(DistRefs {
        far_ref: far.object_ref()?,
        replacement_ref: replacement.object_ref()?,
        session_ref: canonical_hash(&record("vat-session-descriptor-v1", vec![string("session:primary")]))?,
        pending_call_ref: canonical_hash(&record("vat-pending-call-v1", vec![string("call:primary")]))?,
        stale_call_ref: canonical_hash(&record("vat-pending-call-v1", vec![string("call:stale")]))?,
        far,
        replacement,
    })
}
