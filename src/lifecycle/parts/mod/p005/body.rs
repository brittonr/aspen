
pub fn evaluate_service_demand(input: &ServiceDemandEvaluationInput<'_>) -> Result<ServiceDemandEvaluation> {
    validate_service_demand_input(input)?;
    let mut diagnostics = Vec::with_capacity(MAX_DIAGNOSTICS.min(4));
    for dependency_ref in input.required_dependency_refs {
        if !input.ready_dependency_refs.contains(dependency_ref) {
            diagnostics.push(format!("service dependency readiness missing {dependency_ref}"));
        }
    }
    if input.authority_refs.is_empty() {
        diagnostics.push("service start requires authority evidence".to_owned());
    }
    if input.resource_refs.is_empty() {
        diagnostics.push("service start requires resource evidence".to_owned());
    }
    if input.evidence_refs.is_empty() {
        diagnostics.push("service start requires lifecycle evidence".to_owned());
    }
    let is_missing_dependencies = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.starts_with("service dependency readiness missing"));
    let lifecycle_kind = if diagnostics.is_empty() {
        "start"
    } else if is_missing_dependencies {
        "dependency-wait"
    } else {
        "admission-wait"
    };
    let decision = if diagnostics.is_empty() { "pass" } else { "wait" };
    let readiness_assertion = if diagnostics.is_empty() {
        Some(service_lifecycle_assertion(
            input.service_id,
            ServiceAssertionKind::Ready,
            Some(input.manifest_ref),
            &service_readiness_evidence_refs(input)?,
        )?)
    } else {
        None
    };
    Ok(ServiceDemandEvaluation {
        decision: decision.to_owned(),
        lifecycle_kind: lifecycle_kind.to_owned(),
        diagnostics,
        start_side_effect_admitted: decision == "pass",
        readiness_assertion,
    })
}

pub fn service_lifecycle_assertion(
    service_id: &str,
    kind: ServiceAssertionKind,
    target_ref: Option<&str>,
    evidence_refs: &[String],
) -> Result<RuntimeValue> {
    if service_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("service lifecycle assertion id must be non-empty"));
    }
    if let Some(reference) = target_ref {
        validate_content_ref(reference)?;
    }
    validate_refs("evidence", evidence_refs)?;
    RuntimeValue::new(record("lifecycle-service-assertion-v1", vec![
        string(crate::preserves_rail::LIFECYCLE_SERVICE_ASSERTION_SCHEMA),
        record("service", vec![string(service_id)]),
        record("kind", vec![string(kind.as_str())]),
        record("target", vec![optional_ref_value(target_ref)]),
        record("evidence", vec![refs_sequence(evidence_refs)]),
        checks_value(),
    ]))
}

fn validate_service_demand_input(input: &ServiceDemandEvaluationInput<'_>) -> Result<()> {
    if input.service_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("service demand id must be non-empty"));
    }
    validate_content_ref(input.demand_ref)?;
    validate_content_ref(input.manifest_ref)?;
    validate_refs("required dependency", input.required_dependency_refs)?;
    validate_refs("ready dependency", input.ready_dependency_refs)?;
    validate_refs("authority", input.authority_refs)?;
    validate_refs("resource", input.resource_refs)?;
    validate_refs("evidence", input.evidence_refs)?;
    Ok(())
}

fn service_readiness_evidence_refs(input: &ServiceDemandEvaluationInput<'_>) -> Result<Vec<String>> {
    let mut evidence_refs = Vec::with_capacity(
        input
            .evidence_refs
            .len()
            .saturating_add(input.ready_dependency_refs.len())
            .saturating_add(SERVICE_READINESS_BASE_REF_COUNT),
    );
    evidence_refs.extend(input.evidence_refs.iter().cloned());
    evidence_refs.push(input.demand_ref.to_owned());
    evidence_refs.push(input.manifest_ref.to_owned());
    evidence_refs.extend(input.ready_dependency_refs.iter().cloned());
    evidence_refs.sort();
    evidence_refs.dedup();
    validate_refs("service readiness evidence", &evidence_refs)?;
    Ok(evidence_refs)
}

fn validate_transition_input(input: &TransitionInput) -> Result<()> {
    if input.entity_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("lifecycle entity id must be non-empty"));
    }
    if input.cause.trim().is_empty() {
        return Err(MoltenError::invalid_harness("lifecycle transition cause must be non-empty"));
    }
    validate_refs("policy", &input.policy_refs)?;
    validate_refs("resources", &input.resource_refs)?;
    validate_refs("evidence", &input.evidence_refs)?;
    if let Some(supervisor_ref) = &input.supervisor_ref {
        validate_content_ref(supervisor_ref)?;
    }
    Ok(())
}
