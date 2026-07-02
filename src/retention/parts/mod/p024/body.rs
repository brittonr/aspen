
pub fn summary(value: &IoValue) -> Result<String> {
    if let Some(text) = base(value) {
        return Ok(text);
    }
    if let Some(text) = admission(value) {
        return Ok(text);
    }
    if let Some(text) = peer(value) {
        return Ok(text);
    }
    if let Some(text) = live(value) {
        return Ok(text);
    }
    if let Some(text) = gate(value) {
        return Ok(text);
    }
    if let Some(text) = audit(value) {
        return Ok(text);
    }
    if let Some(text) = review(value) {
        return Ok(text);
    }
    if let Some(text) = profile(value) {
        return Ok(text);
    }
    if let Some(text) = stored(value) {
        return Ok(text);
    }
    Err(MoltenError::invalid_harness("unsupported retention artifact"))
}

fn base(value: &IoValue) -> Option<String> {
    if let Ok(profile) = parse_class_profile(value) {
        return Some(format!(
            "retention class ref={} class={} min={} max={} policies={} diagnostics={}",
            profile.profile_ref,
            profile.class_name,
            profile.minimum_age_seconds,
            profile.maximum_age_seconds.map_or_else(|| "none".to_string(), |value| value.to_string()),
            profile.policy_refs.len(),
            profile.diagnostics.join(",")
        ));
    }
    if let Ok(pin) = parse_pin(value) {
        return Some(format!(
            "retention pin ref={} object={} kind={} class={} source={} owner={}",
            pin.pin_ref, pin.object_ref, pin.object_kind, pin.retention_class, pin.source, pin.owner_ref
        ));
    }
    if let Ok(index) = parse_reference_index(value) {
        return Some(format!(
            "retention index ref={} object={} kind={} pins={} retained={} remote={} complete={}",
            index.index_ref,
            index.object_ref,
            index.object_kind,
            index.pin_refs.len(),
            index.retained_refs.len(),
            index.remote_refs.len(),
            index.is_complete
        ));
    }
    None
}

fn admission(value: &IoValue) -> Option<String> {
    if let Ok(admission) = parse_evidence_admission(value) {
        return Some(format!(
            "retention admission ref={} kind={} decision={} object={} class={} action={} current={} revoked={} diagnostics={}",
            admission.admission_ref,
            admission.kind,
            admission.decision,
            admission.object_ref,
            admission.retention_class,
            admission.action,
            admission.is_current,
            admission.revoked_refs.len(),
            admission.diagnostics.join(",")
        ));
    }
    None
}

fn peer(value: &IoValue) -> Option<String> {
    if let Ok(request) = parse_remote_gc_clearance_request(value) {
        return Some(format!(
            "retention remote clearance request ref={} requester={} peer={} remote={} object={} class={} action={} evidence={}",
            request.request_ref,
            request.requester_ref,
            request.peer_ref,
            request.remote_ref,
            request.object_ref,
            request.retention_class,
            request.action,
            request.evidence_refs.len()
        ));
    }
    if let Ok(response) = parse_remote_gc_clearance_response(value) {
        return Some(format!(
            "retention remote clearance response ref={} decision={} request={} clearance={} peer={} remote={} diagnostics={}",
            response.response_ref,
            response.decision,
            response.request_ref,
            response.clearance_ref,
            response.request.peer_ref,
            response.request.remote_ref,
            response.diagnostics.join(",")
        ));
    }
    if let Ok(import) = parse_remote_gc_clearance_import(value) {
        return Some(format!(
            "retention remote clearance import ref={} decision={} request={} response={} clearance={} peer={} remote={} diagnostics={}",
            import.import_ref,
            import.decision,
            import.request_ref,
            import.response_ref,
            import.clearance_ref.as_deref().unwrap_or("none"),
            import.peer_ref,
            import.remote_ref,
            import.diagnostics.join(",")
        ));
    }
    None
}

fn live(value: &IoValue) -> Option<String> {
    if let Ok(workflow) = parse_remote_gc_clearance_live_workflow(value) {
        return Some(format!(
            "retention remote clearance live workflow ref={} decision={} request={} response={} import={} clearance={} peer={} remote={} diagnostics={}",
            workflow.workflow_ref,
            workflow.decision,
            workflow.request_ref,
            workflow.response_ref,
            workflow.import_ref,
            workflow.clearance_ref.as_deref().unwrap_or("none"),
            workflow.peer_ref,
            workflow.remote_ref,
            workflow.diagnostics.join(",")
        ));
    }
    if let Ok(clearance) = parse_remote_gc_clearance(value) {
        return Some(format!(
            "retention remote clearance ref={} decision={} peer={} remote={} object={} class={} action={} current={} retained={} revoked={} diagnostics={}",
            clearance.clearance_ref,
            clearance.decision,
            clearance.peer_ref,
            clearance.remote_ref,
            clearance.object_ref,
            clearance.retention_class,
            clearance.action,
            clearance.is_current,
            clearance.retained_refs.len(),
            clearance.revoked_refs.len(),
            clearance.diagnostics.join(",")
        ));
    }
    None
}

fn gate(value: &IoValue) -> Option<String> {
    if let Ok(plan) = parse_gc_plan(value) {
        return Some(format!(
            "retention gc plan ref={} decision={} subsystem={} action={} object={} class={} requester={} index={} gates={} diagnostics={}",
            plan.plan_ref,
            plan.decision,
            plan.subsystem,
            plan.action,
            plan.object_ref,
            plan.retention_class,
            plan.requester_ref.as_deref().unwrap_or("none"),
            plan.index_ref,
            plan.gates.len(),
            plan.diagnostics.join(",")
        ));
    }
    if let Ok(apply) = parse_gc_apply(value) {
        return Some(format!(
            "retention gc apply ref={} decision={} subsystem={} action={} object={} class={} plan={} recomputed={} receipt={} tombstone={} diagnostics={}",
            apply.apply_ref,
            apply.decision,
            apply.subsystem,
            apply.action,
            apply.object_ref,
            apply.retention_class,
            apply.plan_ref,
            apply.recomputed_plan_ref,
            apply.retention_receipt_ref.as_deref().unwrap_or("none"),
            apply.tombstone_ref.as_deref().unwrap_or("none"),
            apply.diagnostics.join(",")
        ));
    }
    None
}

fn audit(value: &IoValue) -> Option<String> {
    if let Ok(execute) = parse_gc_execution_gate(value) {
        return Some(format!(
            "retention gc execute ref={} decision={} subsystem={} action={} object={} class={} apply={} plan={} receipt={} tombstone={} diagnostics={}",
            execute.execution_ref,
            execute.decision,
            execute.subsystem,
            execute.action,
            execute.object_ref,
            execute.retention_class,
            execute.apply_ref.as_deref().unwrap_or("none"),
            execute.plan_ref.as_deref().unwrap_or("none"),
            execute.retention_receipt_ref.as_deref().unwrap_or("none"),
            execute.tombstone_ref.as_deref().unwrap_or("none"),
            execute.diagnostics.join(",")
        ));
    }
    if let Ok(audit) = parse_gc_audit(value) {
        return Some(format!(
            "retention gc audit ref={} decision={} subsystem={} action={} object={} class={} plan={} apply={} execution={} receipt={} tombstone={} diagnostics={}",
            audit.audit_ref,
            audit.decision,
            audit.subsystem,
            audit.action,
            audit.object_ref,
            audit.retention_class,
            audit.plan_ref.as_deref().unwrap_or("none"),
            audit.apply_ref.as_deref().unwrap_or("none"),
            audit.execution_ref,
            audit.retention_receipt_ref.as_deref().unwrap_or("none"),
            audit.tombstone_ref.as_deref().unwrap_or("none"),
            audit.diagnostics.join(",")
        ));
    }
    None
}

fn review(value: &IoValue) -> Option<String> {
    if let Ok(explain) = parse_candidate_explain(value) {
        return Some(format!(
            "retention candidate explain ref={} object={} kind={} class={} action={} subsystem={} pins={} admissions={} clearances={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
            explain.explain_ref,
            explain.object_ref,
            explain.object_kind.as_deref().unwrap_or("any"),
            explain.retention_class.as_deref().unwrap_or("any"),
            explain.action.as_deref().unwrap_or("any"),
            explain.subsystem.as_deref().unwrap_or("any"),
            explain.pin_refs.len(),
            explain.admission_refs.len(),
            explain.remote_clearance_refs.len(),
            explain.gc_plan_refs.len(),
            explain.gc_apply_refs.len(),
            explain.gc_execution_refs.len(),
            explain.gc_audit_refs.len(),
            explain.retention_receipt_refs.len(),
            explain.tombstone_refs.len(),
            explain.diagnostics.join(",")
        ));
    }
    if let Ok(bundle) = parse_candidate_bundle(value) {
        return Some(format!(
            "retention candidate bundle ref={} explain={} object={} kind={} class={} action={} subsystem={} artifacts={} plans={} applies={} executes={} audits={} receipts={} tombstones={} diagnostics={}",
            bundle.bundle_ref,
            bundle.explain_ref,
            bundle.object_ref,
            bundle.object_kind.as_deref().unwrap_or("any"),
            bundle.retention_class.as_deref().unwrap_or("any"),
            bundle.action.as_deref().unwrap_or("any"),
            bundle.subsystem.as_deref().unwrap_or("any"),
            bundle.artifact_refs.len(),
            bundle.gc_plan_refs.len(),
            bundle.gc_apply_refs.len(),
            bundle.gc_execution_refs.len(),
            bundle.gc_audit_refs.len(),
            bundle.retention_receipt_refs.len(),
            bundle.tombstone_refs.len(),
            bundle.diagnostics.join(",")
        ));
    }
    None
}
