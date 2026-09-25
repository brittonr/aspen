
fn refs_for_key_value(key: &Key, value: &Value) -> Vec<String> {
    let mut refs = vec![
        key.key_ref.clone(),
        value.value_ref.clone(),
        key.input_ref.clone(),
        key.dependency_closure_hash.clone(),
        key.tool_ref.clone(),
    ];
    refs.extend(key.artifact_refs.iter().cloned());
    refs.extend(key.input_refs.iter().cloned());
    refs.extend(key.dependency_refs.iter().cloned());
    refs.extend(key.schema_refs.iter().cloned());
    refs.extend(key.policy_refs.iter().cloned());
    refs.extend(key.policy_export_refs.iter().cloned());
    refs.extend(key.capability_refs.iter().cloned());
    refs.extend(key.revocation_refs.iter().cloned());
    refs.extend(key.resource_refs.iter().cloned());
    refs.extend(key.effect_manifest_refs.iter().cloned());
    refs.extend(key.provenance_refs.iter().cloned());
    refs.extend(key.source_gate_refs.iter().cloned());
    refs.extend(key.evidence_refs.iter().cloned());
    refs.extend(key.retention_refs.iter().cloned());
    refs.extend(key.compatibility_refs.iter().cloned());
    refs.extend(key.assumption_refs.iter().cloned());
    refs.extend(value.dependency_refs.iter().cloned());
    refs.extend(value.policy_refs.iter().cloned());
    refs.extend(value.evidence_refs.iter().cloned());
    if let Some(handler) = key.handler_profile_ref.as_ref() {
        refs.push(handler.clone());
    }
    sorted_unique(&refs)
}

pub fn evaluate_cache_hit_validity(input: CacheHitValidityInput<'_>) -> CacheHitValidityDecision {
    let mut diagnostics = Vec::with_capacity(CACHE_HIT_VALIDITY_DIAGNOSTIC_CAPACITY);
    if input.value.key_ref != input.key.key_ref {
        diagnostics.push("cache-key-value-ref-mismatch".to_string());
    }
    if input.semantic && input.value.tier == TIER_PRODUCTION_TRACE_ONLY {
        diagnostics.push("trace-only-not-semantic".to_string());
    }
    if input.value.status == STATUS_TRACE_ONLY {
        diagnostics.push("diagnostic-only-cache-entry".to_string());
    }
    if input.value.tier == TIER_POLICY_CURRENT && !policy_current_refs_match_parts(input.key, input) {
        diagnostics.push("policy-current-revalidation".to_string());
    }
    diagnostics.extend(admission_freshness_diagnostics(input));
    if !input.requested_dependency_refs.is_empty() {
        let requested = sorted_unique(input.requested_dependency_refs);
        let mut cached = input.key.dependency_refs.clone();
        cached.extend(input.value.dependency_refs.iter().cloned());
        if sorted_unique(&cached) != requested {
            diagnostics.push("dependency-refs-changed".to_string());
        }
    }
    if input
        .current_revocation_refs
        .iter()
        .any(|revocation_ref| input.key.capability_refs.iter().any(|capability_ref| capability_ref == revocation_ref))
    {
        diagnostics.push("capability-revoked".to_string());
    }
    if let Some(expected_output_ref) = input.expected_output_ref {
        let actual_output_ref = match &input.value.output {
            OutputRef::Inline { output_ref, .. } | OutputRef::ContentRef { output_ref, .. } => Some(output_ref.as_str()),
            OutputRef::None => None,
        };
        if actual_output_ref != Some(expected_output_ref) {
            diagnostics.push("output-ref-mismatch".to_string());
        }
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    CacheHitValidityDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn policy_current_refs_match_parts(key: &Key, input: CacheHitValidityInput<'_>) -> bool {
    let compatibility_refs = sorted_unique(input.compatibility_refs);
    refs_match_or_compatible(&key.policy_refs, input.current_policy_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.policy_export_refs, input.current_policy_export_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.capability_refs, input.current_capability_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.revocation_refs, input.current_revocation_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.resource_refs, input.current_resource_refs, &compatibility_refs)
        && optional_ref_matches_or_compatible(
            key.handler_profile_ref.as_deref(),
            input.current_handler_profile_ref,
            &compatibility_refs,
        )
        && refs_match_or_compatible(&key.provenance_refs, input.current_provenance_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.source_gate_refs, input.current_source_gate_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.retention_refs, input.current_retention_refs, &compatibility_refs)
        && refs_match_or_compatible(&key.evidence_refs, input.current_evidence_refs, &compatibility_refs)
}

fn admission_freshness_diagnostics(input: CacheHitValidityInput<'_>) -> Vec<String> {
    let compatibility_refs = sorted_unique(input.compatibility_refs);
    let mut diagnostics = Vec::new();
    let ref_checks: [(&str, &[String], &[String]); 9] = [
        ("policy-ref-stale", &input.key.policy_refs, input.current_policy_refs),
        ("policy-export-ref-stale", &input.key.policy_export_refs, input.current_policy_export_refs),
        ("capability-context-stale", &input.key.capability_refs, input.current_capability_refs),
        ("revocation-epoch-changed", &input.key.revocation_refs, input.current_revocation_refs),
        ("resource-context-stale", &input.key.resource_refs, input.current_resource_refs),
        ("provenance-context-stale", &input.key.provenance_refs, input.current_provenance_refs),
        ("source-gate-context-stale", &input.key.source_gate_refs, input.current_source_gate_refs),
        ("retention-context-stale", &input.key.retention_refs, input.current_retention_refs),
        ("evidence-context-stale", &input.key.evidence_refs, input.current_evidence_refs),
    ];
    for (label, cached_refs, current_refs) in ref_checks {
        push_changed_ref_diagnostic(&mut diagnostics, label, cached_refs, current_refs, &compatibility_refs);
    }
    if input.key.handler_profile_ref.as_deref() != input.current_handler_profile_ref
        && !optional_ref_change_is_compatible(
            input.key.handler_profile_ref.as_deref(),
            input.current_handler_profile_ref,
            &compatibility_refs,
        )
    {
        diagnostics.push("handler-profile-changed".to_string());
    }
    diagnostics
}

fn push_changed_ref_diagnostic(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    label: &str,
    cached_refs: &[String],
    current_refs: &[String],
    compatibility_refs: &[String],
) {
    if refs_match_or_compatible(cached_refs, current_refs, compatibility_refs) {
        return;
    }
    diagnostics.push_item(label.to_string());
}

fn refs_match_or_compatible(cached_refs: &[String], current_refs: &[String], compatibility_refs: &[String]) -> bool {
    sorted_unique(cached_refs) == sorted_unique(current_refs)
        || all_ref_changes_compatible(cached_refs, current_refs, compatibility_refs)
}

fn optional_ref_matches_or_compatible(
    cached_ref: Option<&str>,
    current_ref: Option<&str>,
    compatibility_refs: &[String],
) -> bool {
    cached_ref == current_ref || optional_ref_change_is_compatible(cached_ref, current_ref, compatibility_refs)
}

fn all_ref_changes_compatible(cached_refs: &[String], current_refs: &[String], compatibility_refs: &[String]) -> bool {
    !compatibility_refs.is_empty()
        && cached_refs.iter().all(|reference| compatibility_refs.contains(reference))
        && current_refs.iter().all(|reference| compatibility_refs.contains(reference))
}

fn optional_ref_change_is_compatible(
    cached_ref: Option<&str>,
    current_ref: Option<&str>,
    compatibility_refs: &[String],
) -> bool {
    !compatibility_refs.is_empty()
        && cached_ref.is_some_and(|reference| compatibility_refs.iter().any(|compat| compat == reference))
        && current_ref.is_some_and(|reference| compatibility_refs.iter().any(|compat| compat == reference))
}
