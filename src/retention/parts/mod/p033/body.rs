
pub fn store_remote_gc_clearance_live_workflow_with_root(
    root: &CapabilityRetentionRoot,
    value: &IoValue,
) -> Result<RemoteGcClearanceLiveWorkflow> {
    ensure_store_with_root(root)?;
    let workflow = parse_remote_gc_clearance_live_workflow(value)?;
    write_store_value_with_root(
        root,
        &capability_ref_path(REMOTE_CLEARANCE_LIVE_WORKFLOW_DIR, &workflow.workflow_ref)?,
        &workflow.value,
    )?;
    Ok(workflow)
}

struct AdmissionScope<'a> {
    requester_ref: Option<&'a str>,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
    action: &'a str,
}

struct AdmissionRefsInput<'a, Root: ?Sized = Path> {
    root: &'a Root,
    refs: &'a [String],
    expected_kind: &'a str,
    scope: &'a AdmissionScope<'a>,
    required_remote_refs: &'a [String],
}
