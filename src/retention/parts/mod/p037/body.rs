
fn gc_audit_scope<'a>(
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
) -> GcAuditScope<'a> {
    GcAuditScope {
        subsystem,
        retention: audit_scope(action, object_ref, object_kind, retention_class),
    }
}

fn audit_scope<'a>(
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
) -> AuditScope<'a> {
    AuditScope {
        action,
        object_ref,
        object_kind,
        retention_class,
    }
}

fn same_gc_scope(left: &GcAuditScope<'_>, right: &GcAuditScope<'_>) -> bool {
    left.subsystem == right.subsystem && same_audit_scope(&left.retention, &right.retention)
}

fn same_audit_scope(left: &AuditScope<'_>, right: &AuditScope<'_>) -> bool {
    left.action == right.action
        && left.object_ref == right.object_ref
        && left.object_kind == right.object_kind
        && left.retention_class == right.retention_class
}
