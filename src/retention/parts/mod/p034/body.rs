
fn check_scope<Root: ?Sized>(input: &RemoteClearanceRefsInput<'_, Root>, clearance: &RemoteGcClearance) -> Check {
    let mut scope_mismatches = 0usize;
    if input.scope.requester_ref != Some(clearance.requester_ref.as_str()) {
        scope_mismatches += 1;
    }
    if clearance.object_ref != input.scope.object_ref || clearance.object_kind != input.scope.object_kind {
        scope_mismatches += 1;
    }
    if clearance.retention_class != input.scope.retention_class {
        scope_mismatches += 1;
    }
    if clearance.action != input.scope.action {
        scope_mismatches += 1;
    }
    Check {
        is_admitted: scope_mismatches == 0,
        scope_mismatches,
    }
}
