
fn failure_lifecycle_receipt(
    suite: &ServiceSupervisionSuite,
    failure_status_ref: &str,
    supervision_refs: &[String],
) -> Result<IoValue> {
    crate::service_records::service_lifecycle_receipt_value(&crate::service_records::ServiceLifecycleReceiptInput {
        operation: "fail".to_string(),
        decision: "pass".to_string(),
        service_id: suite.manifest.service_id.clone(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        status_ref: Some(failure_status_ref.to_string()),
        authority_refs: suite.evidence.authority_refs.clone(),
        resource_refs: suite.evidence.resource_refs.clone(),
        effect_profile_refs: suite.evidence.effect_log_refs.clone(),
        supervision_refs: supervision_refs.to_vec(),
        diagnostics: Vec::new(),
    })
}

// r[impl molten.sam_service_supervision_cleanup.spec.bounded_restart]
fn evaluate_restart(suite: &ServiceSupervisionSuite) -> Result<RestartEvaluation> {
    let is_authority_present = !suite.evidence.authority_refs.is_empty();
    let is_resource_present = !suite.evidence.resource_refs.is_empty();
    let is_revoked = !suite.evidence.revocation_refs.is_empty();
    let backoff_slot = suite
        .restart_attempt
        .checked_mul(suite.restart_policy.backoff_steps)
        .ok_or_else(|| MoltenError::invalid_harness("service restart backoff overflow"))?;
    let attempt = if suite.restart_attempt >= suite.restart_policy.max_attempts {
        suite.restart_attempt
    } else {
        suite
            .restart_attempt
            .checked_add(1)
            .ok_or_else(|| MoltenError::invalid_harness("service restart attempt overflow"))?
    };
    if is_revoked {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "service owner authority revoked".to_string(),
        ]));
    }
    if !is_authority_present {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "missing restart authority evidence".to_string(),
        ]));
    }
    if !is_resource_present {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "missing restart resource evidence".to_string(),
        ]));
    }
    if suite.restart_attempt >= suite.restart_policy.max_attempts {
        return Ok(restart_evaluation("deny", attempt, backoff_slot, vec![
            "restart attempt budget exhausted".to_string(),
        ]));
    }
    if suite.logical_step < backoff_slot {
        return Ok(restart_evaluation("backoff", attempt, backoff_slot, vec![
            "logical backoff slot has not elapsed".to_string(),
        ]));
    }
    Ok(restart_evaluation("pass", attempt, backoff_slot, Vec::new()))
}

fn restart_evaluation(decision: &str, attempt: u64, backoff_slot: u64, diagnostics: Vec<String>) -> RestartEvaluation {
    RestartEvaluation {
        decision: decision.to_string(),
        attempt,
        backoff_slot,
        diagnostics,
    }
}

fn restart_decision_value(
    suite: &ServiceSupervisionSuite,
    restart: &RestartEvaluation,
    failure_lifecycle_ref: &str,
) -> Result<IoValue> {
    let mut prior_lifecycle_refs = suite.evidence.prior_lifecycle_refs.clone();
    prior_lifecycle_refs.push(failure_lifecycle_ref.to_string());
    crate::service_records::service_restart_decision_value(&crate::service_records::ServiceRestartDecisionInput {
        decision: restart.decision.clone(),
        service_id: suite.manifest.service_id.clone(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        policy_ref: suite.restart_policy.policy_ref.clone(),
        attempt: restart.attempt,
        max_attempts: suite.restart_policy.max_attempts,
        window_step: suite.logical_step,
        backoff_slot: restart.backoff_slot,
        prior_lifecycle_refs,
        authority_refs: suite.evidence.authority_refs.clone(),
        resource_refs: suite.evidence.resource_refs.clone(),
        diagnostics: restart.diagnostics.clone(),
    })
}

fn scheduled_demands(suite: &ServiceSupervisionSuite, restart: &RestartEvaluation) -> Result<Vec<IoValue>> {
    if restart.decision != "pass" {
        return Ok(Vec::new());
    }
    let requester_ref = suite
        .evidence
        .authority_refs
        .first()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("restart pass requires authority ref"))?;
    let demand = crate::service_records::service_demand_value(&crate::service_records::ServiceDemandInput {
        demand_id: format!("restart:{}:{}", suite.manifest.service_id, restart.attempt),
        service_id: suite.manifest.service_id.clone(),
        requester_ref,
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        policy_refs: suite.manifest.policy_refs.clone(),
    })?;
    Ok(vec![demand])
}

// r[impl molten.sam_service_supervision.spec.cleanup]
// r[impl molten.sam_service_supervision_cleanup.spec.owned_cleanup]
fn evaluate_cleanup(
    suite: &ServiceSupervisionSuite,
    restart: &RestartEvaluation,
    restart_decision_ref: &str,
) -> Result<CleanupEvaluation> {
    let is_cleanup_required = restart.decision == "deny" || !suite.evidence.revocation_refs.is_empty();
    if !is_cleanup_required {
        return Ok(CleanupEvaluation {
            cleanup_receipt: None,
            retractions: Vec::new(),
            retention_input: None,
        });
    }
    if !suite.owned_state.foreign_ref_claims.is_empty() {
        return foreign_claim_cleanup_evaluation(suite, restart_decision_ref);
    }
    owned_state_cleanup_evaluation(suite, restart_decision_ref)
}

fn foreign_claim_cleanup_evaluation(
    suite: &ServiceSupervisionSuite,
    restart_decision_ref: &str,
) -> Result<CleanupEvaluation> {
    let cleanup_receipt =
        crate::service_records::service_cleanup_receipt_value(&crate::service_records::ServiceCleanupReceiptInput {
            decision: "deny".to_string(),
            service_id: suite.manifest.service_id.clone(),
            manifest_ref: Some(suite.manifest.manifest_ref.clone()),
            authority_refs: suite.evidence.authority_refs.clone(),
            owned_assertion_refs: suite.owned_state.owned_assertion_refs.clone(),
            observer_refs: suite.owned_state.observer_refs.clone(),
            live_ref_refs: suite.owned_state.live_ref_refs.clone(),
            exposed_ref_refs: suite.owned_state.exposed_ref_refs.clone(),
            pending_effect_refs: suite.owned_state.pending_effect_refs.clone(),
            retraction_refs: Vec::new(),
            revocation_refs: suite.evidence.revocation_refs.clone(),
            retention_refs: suite.evidence.retention_policy_refs.clone(),
            diagnostics: vec!["foreign service-owned state cannot be proven".to_string()],
        })?;
    let retention_input =
        retention_input_value(suite, &crate::preserves_rail::canonical_hash(&cleanup_receipt)?, restart_decision_ref)?;
    Ok(CleanupEvaluation {
        cleanup_receipt: Some(cleanup_receipt),
        retractions: Vec::new(),
        retention_input: Some(retention_input),
    })
}

fn owned_state_cleanup_evaluation(
    suite: &ServiceSupervisionSuite,
    restart_decision_ref: &str,
) -> Result<CleanupEvaluation> {
    let targets = cleanup_targets(&suite.owned_state)?;
    let mut retractions = Vec::with_capacity(targets.len());
    let mut retraction_refs = Vec::with_capacity(targets.len());
    for target in targets {
        let retraction = retraction_value(suite, &target)?;
        retraction_refs.push(crate::preserves_rail::canonical_hash(&retraction)?);
        retractions.push(retraction);
    }
    let cleanup_receipt =
        crate::service_records::service_cleanup_receipt_value(&crate::service_records::ServiceCleanupReceiptInput {
            decision: "pass".to_string(),
            service_id: suite.manifest.service_id.clone(),
            manifest_ref: Some(suite.manifest.manifest_ref.clone()),
            authority_refs: suite.evidence.authority_refs.clone(),
            owned_assertion_refs: suite.owned_state.owned_assertion_refs.clone(),
            observer_refs: suite.owned_state.observer_refs.clone(),
            live_ref_refs: suite.owned_state.live_ref_refs.clone(),
            exposed_ref_refs: suite.owned_state.exposed_ref_refs.clone(),
            pending_effect_refs: suite.owned_state.pending_effect_refs.clone(),
            retraction_refs,
            revocation_refs: suite.evidence.revocation_refs.clone(),
            retention_refs: suite.evidence.retention_policy_refs.clone(),
            diagnostics: Vec::new(),
        })?;
    let retention_input =
        retention_input_value(suite, &crate::preserves_rail::canonical_hash(&cleanup_receipt)?, restart_decision_ref)?;
    Ok(CleanupEvaluation {
        cleanup_receipt: Some(cleanup_receipt),
        retractions,
        retention_input: Some(retention_input),
    })
}

fn cleanup_targets(owned_state: &ServiceOwnedState) -> Result<Vec<CleanupTarget>> {
    let total = owned_state
        .owned_assertion_refs
        .len()
        .checked_add(owned_state.observer_refs.len())
        .and_then(|total| total.checked_add(owned_state.live_ref_refs.len()))
        .and_then(|total| total.checked_add(owned_state.exposed_ref_refs.len()))
        .and_then(|total| total.checked_add(owned_state.pending_effect_refs.len()))
        .ok_or_else(|| MoltenError::invalid_harness("service cleanup target count overflow"))?;
    ensure_count_at_most(total, "service cleanup targets")?;
    let mut targets = OrderedSet::new();
    insert_targets(&mut targets, "owned-assertion", &owned_state.owned_assertion_refs);
    insert_targets(&mut targets, "observer", &owned_state.observer_refs);
    insert_targets(&mut targets, "live-ref", &owned_state.live_ref_refs);
    insert_targets(&mut targets, "exposed-ref", &owned_state.exposed_ref_refs);
    insert_targets(&mut targets, "pending-effect", &owned_state.pending_effect_refs);
    Ok(targets.into_iter().collect())
}

fn insert_targets(targets: &mut OrderedSet<CleanupTarget>, kind: &str, refs: &[String]) {
    for target_ref in refs {
        targets.insert(CleanupTarget {
            kind: kind.to_string(),
            target_ref: target_ref.clone(),
        });
    }
}

fn retraction_value(suite: &ServiceSupervisionSuite, target: &CleanupTarget) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("service-retraction-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_RETRACTION_SCHEMA),
        crate::preserves_rail::record("service-id", vec![crate::preserves_rail::string(&suite.manifest.service_id)]),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(&suite.manifest.manifest_ref)]),
        crate::preserves_rail::record("kind", vec![crate::preserves_rail::string(&target.kind)]),
        crate::preserves_rail::record("target", vec![crate::preserves_rail::string(&target.target_ref)]),
        crate::preserves_rail::record("authority", vec![refs_sequence(&suite.evidence.authority_refs)]),
        crate::preserves_rail::record("revocations", vec![refs_sequence(&suite.evidence.revocation_refs)]),
        checks_value(&["service-owned-retraction", "no-foreign-delete", "retention-still-gates"]),
    ]))
}

// r[impl molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention]
fn retention_input_value(
    suite: &ServiceSupervisionSuite,
    cleanup_receipt_ref: &str,
    restart_decision_ref: &str,
) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("service-retention-input-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_RETENTION_INPUT_SCHEMA),
        crate::preserves_rail::record("service-id", vec![crate::preserves_rail::string(&suite.manifest.service_id)]),
        crate::preserves_rail::record("cleanup", vec![crate::preserves_rail::string(cleanup_receipt_ref)]),
        crate::preserves_rail::record("restart-decision", vec![crate::preserves_rail::string(restart_decision_ref)]),
        crate::preserves_rail::record("retention-policy", vec![refs_sequence(&suite.evidence.retention_policy_refs)]),
        checks_value(&[
            "cleanup-is-input-evidence",
            "retention-policy-still-decides",
            "no-physical-delete",
        ]),
    ]))
}

fn status_values(failure_status: &IoValue, final_statuses: &[IoValue]) -> Result<Vec<IoValue>> {
    let total = final_statuses
        .len()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("service status count overflow"))?;
    let mut statuses = Vec::with_capacity(total);
    statuses.push(failure_status.clone());
    statuses.extend_from_slice(final_statuses);
    Ok(statuses)
}

fn refs_for_values(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    Ok(refs)
}

fn supervision_refs(
    suite: &ServiceSupervisionSuite,
    monitor_refs: &[String],
    notification_refs: &[String],
) -> Result<Vec<String>> {
    let total = suite
        .links
        .len()
        .checked_add(monitor_refs.len())
        .and_then(|total| total.checked_add(notification_refs.len()))
        .ok_or_else(|| MoltenError::invalid_harness("service supervision ref count overflow"))?;
    let mut refs = Vec::with_capacity(total);
    refs.extend(suite.links.iter().map(|link| link.link_ref.clone()));
    refs.extend_from_slice(monitor_refs);
    refs.extend_from_slice(notification_refs);
    Ok(refs)
}
