
pub fn evaluate_service_dependencies(state: &RuntimeServiceDependenciesState) -> Result<ServiceDependenciesResult> {
    let diagnostics = validate_service_dependencies(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let dependency_ref = state.dependency_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-service-dependencies-input-v1", vec![
        crate::preserves_rail::record("dependency-ref", vec![crate::preserves_rail::string(&dependency_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "service-refs-canonical".to_string(),
        "demand-dependencies-ready".to_string(),
        "failed-dependency-admission".to_string(),
        "restart-refs-match-failures".to_string(),
        "shutdown-reverse-dependencies-first".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(2);
    state_refs.push(dependency_ref);
    if crate::preserves_rail::validate_content_ref(&state.service_ref).is_ok() {
        state_refs.push(state.service_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: SERVICE_DEPENDENCIES_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(ServiceDependenciesResult { is_allowed, receipt })
}

pub fn evaluate_near_far_refs(state: &RuntimeNearFarRefState) -> Result<NearFarRefsResult> {
    let diagnostics = validate_near_far_refs(state);
    let is_allowed = diagnostics.is_empty();
    let decision = if is_allowed {
        PredicateDecision::Pass
    } else {
        PredicateDecision::Deny
    };
    let call_ref = state.call_ref()?;
    let input_value = crate::preserves_rail::record("runtime-predicate-near-far-refs-input-v1", vec![
        crate::preserves_rail::record("call-ref", vec![crate::preserves_rail::string(&call_ref)]),
        state.to_value(),
    ]);
    let checks = vec![
        "reference-ref-canonical".to_string(),
        "live-reference-required".to_string(),
        "near-ref-synchronous-same-vat".to_string(),
        "far-ref-asynchronous-only".to_string(),
    ];
    let mut state_refs = Vec::with_capacity(2);
    state_refs.push(call_ref);
    if crate::preserves_rail::validate_content_ref(&state.reference_ref).is_ok() {
        state_refs.push(state.reference_ref.clone());
    }
    let receipt = build_runtime_predicate_receipt(RuntimePredicateReceiptInput {
        predicate: NEAR_FAR_REFS_PREDICATE,
        input_value,
        decision,
        state_refs,
        checks,
        diagnostics,
    })?;

    Ok(NearFarRefsResult { is_allowed, receipt })
}

fn validate_object_authority(state: &RuntimeObjectAuthorityState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(16);
    if crate::preserves_rail::validate_content_ref(&state.object_ref).is_err() {
        diagnostics.push("object-authority-object-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.requested_authority_ref).is_err() {
        diagnostics.push("object-authority-requested-ref-noncanonical".to_string());
    }
    diagnostics.extend(validate_sorted_content_refs(&state.endowed_authority_refs, "object-authority", "endowed"));
    diagnostics.extend(validate_sorted_content_refs(&state.admitted_authority_refs, "object-authority", "admitted"));

    let requested_ref = state.requested_authority_ref.as_str();
    let endowed_refs = string_set(&state.endowed_authority_refs);
    let admitted_refs = string_set(&state.admitted_authority_refs);
    if !endowed_refs.contains(requested_ref) {
        diagnostics.push("object-authority-not-endowed".to_string());
    }
    if !admitted_refs.contains(requested_ref) {
        diagnostics.push("object-authority-not-policy-admitted".to_string());
    }
    if !is_subset(&endowed_refs, &admitted_refs) {
        diagnostics.push("object-authority-endowment-not-admitted".to_string());
    }
    diagnostics
}

fn validate_rights_amplification(state: &RuntimeRightsAmplificationState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(16);
    if crate::preserves_rail::validate_content_ref(&state.holder_object_ref).is_err() {
        diagnostics.push("rights-amplification-holder-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.sealed_value_ref).is_err() {
        diagnostics.push("rights-amplification-sealed-value-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.sealer_brand_ref).is_err() {
        diagnostics.push("rights-amplification-sealer-brand-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.unsealer_brand_ref).is_err() {
        diagnostics.push("rights-amplification-unsealer-brand-ref-noncanonical".to_string());
    }
    diagnostics.extend(validate_sorted_content_refs(
        &state.sealed_authority_refs,
        "rights-amplification",
        "sealed-authority",
    ));
    diagnostics.extend(validate_sorted_content_refs(
        &state.recovered_authority_refs,
        "rights-amplification",
        "recovered-authority",
    ));

    if state.sealed_authority_refs.is_empty() {
        diagnostics.push("rights-amplification-empty-sealed-authority".to_string());
    }
    if state.recovered_authority_refs.is_empty() {
        diagnostics.push("rights-amplification-empty-recovered-authority".to_string());
    }
    if state.sealer_brand_ref != state.unsealer_brand_ref {
        diagnostics.push("rights-amplification-brand-mismatch".to_string());
    }
    let sealed_refs = string_set(&state.sealed_authority_refs);
    let recovered_refs = string_set(&state.recovered_authority_refs);
    if !is_subset(&recovered_refs, &sealed_refs) {
        diagnostics.push("rights-amplification-recovered-authority-not-sealed".to_string());
    }
    diagnostics
}

fn validate_distributed_ref_lifetime(state: &RuntimeDistributedRefLifetimeState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(24);
    if crate::preserves_rail::validate_content_ref(&state.far_ref).is_err() {
        diagnostics.push("distributed-ref-far-ref-noncanonical".to_string());
    }
    if crate::preserves_rail::validate_content_ref(&state.session_ref).is_err() {
        diagnostics.push("distributed-ref-session-ref-noncanonical".to_string());
    }
    if state
        .replacement_ref
        .as_ref()
        .is_some_and(|replacement_ref| crate::preserves_rail::validate_content_ref(replacement_ref).is_err())
    {
        diagnostics.push("distributed-ref-replacement-ref-noncanonical".to_string());
    }
    diagnostics.extend(validate_sorted_content_refs(&state.pending_call_refs, "distributed-ref", "pending-call"));
    diagnostics.extend(validate_sorted_content_refs(
        &state.failed_pending_call_refs,
        "distributed-ref",
        "failed-pending-call",
    ));
    diagnostics.extend(validate_sorted_content_refs(&state.attempted_use_refs, "distributed-ref", "attempted-use"));

    let pending_refs = string_set(&state.pending_call_refs);
    let failed_refs = string_set(&state.failed_pending_call_refs);
    let attempted_refs = string_set(&state.attempted_use_refs);
    let far_ref = state.far_ref.as_str();

    if state.is_session_live && state.is_handoff_admitted {
        diagnostics.push("distributed-ref-live-session-with-handoff".to_string());
    }
    if state.is_session_live && state.replacement_ref.is_some() {
        diagnostics.push("distributed-ref-live-session-has-replacement".to_string());
    }
    if !state.is_session_live && !state.is_handoff_admitted && !is_subset(&pending_refs, &failed_refs) {
        diagnostics.push("distributed-ref-disconnected-pending-calls-not-failed".to_string());
    }
    if !state.is_session_live && attempted_refs.contains(far_ref) {
        diagnostics.push("distributed-ref-stale-descriptor-used".to_string());
    }
    if state.is_handoff_admitted {
        match state.replacement_ref.as_deref() {
            Some(replacement_ref) => {
                let mut replacement_refs = OrderedSet::new();
                replacement_refs.insert(replacement_ref);
                if !is_subset(&attempted_refs, &replacement_refs) {
                    diagnostics.push("distributed-ref-handoff-use-not-replacement".to_string());
                }
            }
            None => diagnostics.push("distributed-ref-handoff-replacement-missing".to_string()),
        }
    }
    diagnostics
}

fn validate_service_dependencies(state: &RuntimeServiceDependenciesState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(32);
    if crate::preserves_rail::validate_content_ref(&state.service_ref).is_err() {
        diagnostics.push("service-ref-noncanonical".to_string());
    }
    diagnostics.extend(validate_sorted_content_refs(&state.demanded_service_refs, "service", "demanded"));
    diagnostics.extend(validate_sorted_content_refs(&state.dependency_refs, "service", "dependency"));
    diagnostics.extend(validate_sorted_content_refs(&state.ready_service_refs, "service", "ready"));
    diagnostics.extend(validate_sorted_content_refs(&state.failed_service_refs, "service", "failed"));
    diagnostics.extend(validate_sorted_content_refs(&state.force_run_refs, "service", "force-run"));
    diagnostics.extend(validate_sorted_content_refs(&state.restart_refs, "service", "restart"));
    diagnostics.extend(validate_sorted_content_refs(&state.reverse_dependency_refs, "service", "reverse-dependency"));
    diagnostics.extend(validate_sorted_content_refs(&state.shutdown_refs, "service", "shutdown"));

    let service_ref = state.service_ref.as_str();
    let demanded_refs = string_set(&state.demanded_service_refs);
    let dependency_refs = string_set(&state.dependency_refs);
    let ready_refs = string_set(&state.ready_service_refs);
    let failed_refs = string_set(&state.failed_service_refs);
    let force_run_refs = string_set(&state.force_run_refs);
    let restart_refs = string_set(&state.restart_refs);
    let reverse_dependency_refs = string_set(&state.reverse_dependency_refs);
    let shutdown_refs = string_set(&state.shutdown_refs);

    if has_set_intersection(&ready_refs, &failed_refs) {
        diagnostics.push("service-ready-and-failed".to_string());
    }
    if !is_subset(&restart_refs, &failed_refs) {
        diagnostics.push("service-restart-without-failure".to_string());
    }

    let is_demanded = demanded_refs.contains(service_ref);
    let is_force_run = force_run_refs.contains(service_ref);
    let is_ready = ready_refs.contains(service_ref);
    if (is_demanded || is_ready) && !is_force_run && !is_subset(&dependency_refs, &ready_refs) {
        diagnostics.push("service-dependencies-not-ready".to_string());
    }

    let failed_dependencies = set_intersection(&dependency_refs, &failed_refs);
    if !failed_dependencies.is_empty() && !is_force_run && !is_subset(&failed_dependencies, &restart_refs) {
        diagnostics.push("service-failed-dependency-without-admission".to_string());
    }

    if shutdown_refs.contains(service_ref) && !is_subset(&reverse_dependency_refs, &shutdown_refs) {
        diagnostics.push("service-shutdown-before-reverse-dependencies".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn validate_snapshot_authority(state: &RuntimeSnapshotAuthorityState) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(24);
    if crate::preserves_rail::validate_content_ref(&state.snapshot_ref).is_err() {
        diagnostics.push("snapshot-ref-noncanonical".to_string());
    }
    diagnostics.extend(validate_sorted_content_refs(&state.admitted_authority_refs, "snapshot", "admitted-authority"));
    diagnostics.extend(validate_sorted_content_refs(&state.claimed_authority_refs, "snapshot", "claimed-authority"));
    diagnostics.extend(validate_sorted_content_refs(
        &state.requested_assertion_refs,
        "snapshot",
        "requested-assertion",
    ));
    diagnostics.extend(validate_sorted_content_refs(&state.readable_assertion_refs, "snapshot", "readable-assertion"));
    diagnostics.extend(validate_sorted_content_refs(&state.redacted_assertion_refs, "snapshot", "redacted-assertion"));

    let admitted_refs = string_set(&state.admitted_authority_refs);
    let claimed_refs = string_set(&state.claimed_authority_refs);
    let requested_refs = string_set(&state.requested_assertion_refs);
    let readable_refs = string_set(&state.readable_assertion_refs);
    let redacted_refs = string_set(&state.redacted_assertion_refs);

    if !is_subset(&claimed_refs, &admitted_refs) {
        diagnostics.push("snapshot-claimed-authority-not-admitted".to_string());
    }
    if !is_subset(&readable_refs, &admitted_refs) {
        diagnostics.push("snapshot-readable-assertion-not-authorized".to_string());
    }
    if has_set_intersection(&readable_refs, &redacted_refs) {
        diagnostics.push("snapshot-assertion-readable-and-redacted".to_string());
    }
    let mut covered_refs = readable_refs.clone();
    for redacted_ref in &redacted_refs {
        covered_refs.insert(*redacted_ref);
    }
    if !is_subset(&requested_refs, &covered_refs) {
        diagnostics.push("snapshot-requested-assertion-uncovered".to_string());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}
