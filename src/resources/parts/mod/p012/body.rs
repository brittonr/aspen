
// ---------------------------------------------------------------------------
// Pure core validation functions
// ---------------------------------------------------------------------------

/// Validate the identity and metadata of a canonical resource record.
pub fn validate_resource_record(resource: &ResourceRecord) -> Result<ResourceRecord> {
    validate_non_empty(&resource.resource_type, "resource type")?;
    require_ref(&resource.resource_ref, "resource ref")?;
    require_ref(&resource.scope_ref, "scope ref")?;
    validate_scoped_name(&resource.name)?;

    if resource.generation == 0 {
        return Err(MoltenError::invalid_harness(
            "resource generation must be at least 1",
        ));
    }

    require_ref(&resource.desired_ref, "desired ref")?;
    if let Some(ref observed_ref) = resource.observed_ref {
        require_ref(observed_ref, "observed ref")?;
    }

    validate_metadata(&resource.metadata)?;

    for evidence_ref in &resource.evidence_refs {
        require_ref(evidence_ref, "evidence ref")?;
    }

    Ok(resource.clone())
}

/// Validate resource metadata bounds and label/annotation shapes.
pub fn validate_metadata(metadata: &ResourceMetadata) -> Result<()> {
    if metadata.labels.len() > MAX_LABEL_COUNT {
        return Err(MoltenError::invalid_harness(format!(
            "label count {} exceeds maximum {MAX_LABEL_COUNT}",
            metadata.labels.len()
        )));
    }
    for (key, value) in &metadata.labels {
        validate_label_key(key)?;
        validate_label_value(value)?;
    }

    if metadata.annotations.len() > MAX_ANNOTATION_COUNT {
        return Err(MoltenError::invalid_harness(format!(
            "annotation count {} exceeds maximum {MAX_ANNOTATION_COUNT}",
            metadata.annotations.len()
        )));
    }
    for (key, value) in &metadata.annotations {
        validate_annotation_key(key)?;
        validate_annotation_value(value)?;
    }

    if metadata.owner_refs.len() > MAX_OWNER_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "owner ref count {} exceeds maximum {MAX_OWNER_REFS}",
            metadata.owner_refs.len()
        )));
    }
    for owner_ref in &metadata.owner_refs {
        require_ref(&owner_ref.resource_ref, "owner resource ref")?;
        validate_non_empty(&owner_ref.resource_type, "owner resource type")?;
    }

    if metadata.finalizers.len() > MAX_FINALIZERS {
        return Err(MoltenError::invalid_harness(format!(
            "finalizer count {} exceeds maximum {MAX_FINALIZERS}",
            metadata.finalizers.len()
        )));
    }
    for finalizer in &metadata.finalizers {
        validate_non_empty(finalizer, "finalizer")?;
    }

    if metadata.evidence_refs.len() > MAX_EVIDENCE_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "metadata evidence ref count {} exceeds maximum {MAX_EVIDENCE_REFS}",
            metadata.evidence_refs.len()
        )));
    }
    for evidence_ref in &metadata.evidence_refs {
        require_ref(evidence_ref, "metadata evidence ref")?;
    }

    Ok(())
}

/// Validate a status condition against the current resource generation.
pub fn validate_status_condition(
    condition: &StatusCondition,
    current_generation: u64,
) -> Result<()> {
    if condition.observed_generation > current_generation {
        return Err(MoltenError::invalid_harness(format!(
            "observed generation {} exceeds current generation {}",
            condition.observed_generation, current_generation
        )));
    }
    if condition.evidence_refs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "status condition must have at least one evidence ref",
        ));
    }
    for evidence_ref in &condition.evidence_refs {
        require_ref(evidence_ref, "condition evidence ref")?;
    }
    if let Some(ref observed_ref) = condition.observed_state_ref {
        require_ref(observed_ref, "observed state ref")?;
    }
    validate_non_empty(&condition.condition_type, "condition type")?;
    validate_non_empty(&condition.reason, "condition reason")?;
    validate_non_empty(&condition.message, "condition message")?;

    Ok(())
}

/// Evaluate deletion eligibility from owner refs, finalizers, pins, retention, and authority.
pub fn evaluate_deletion_gate(input: &DeletionGateInput) -> Result<DeletionDecision> {
    let mut cleared =
        Vec::with_capacity(input.owner_refs.len() + input.finalizers.len() + input.deletion_authority_refs.len());
    let mut unresolved = Vec::with_capacity(
        input.owner_refs.len() + input.finalizers.len() + input.pin_refs.len() + input.retention_policy_refs.len() + 1,
    );

    for owner_ref in &input.owner_refs {
        if input.live_owner_refs.contains(&owner_ref.resource_ref) {
            unresolved.push(format!(
                "live owner ref: {} (type: {})",
                owner_ref.resource_ref, owner_ref.resource_type
            ));
        } else {
            cleared.push(format!("owner ref: {}", owner_ref.resource_ref));
        }
    }

    for finalizer in &input.finalizers {
        let has_cleanup = input
            .finalizer_cleanup_receipts
            .iter()
            .any(|receipt| receipt.contains(finalizer));
        if has_cleanup {
            cleared.push(format!("finalizer cleanup: {finalizer}"));
        } else {
            unresolved.push(format!("unresolved finalizer: {finalizer}"));
        }
    }

    for pin_ref in &input.pin_refs {
        unresolved.push(format!("active pin: {pin_ref}"));
    }
    for retention_ref in &input.retention_policy_refs {
        unresolved.push(format!("retention hold: {retention_ref}"));
    }
    if input.deletion_authority_refs.is_empty() {
        unresolved.push("missing deletion authority evidence".to_string());
    } else {
        for auth_ref in &input.deletion_authority_refs {
            cleared.push(format!("deletion authority: {auth_ref}"));
        }
    }

    let is_ready = unresolved.is_empty();
    let decision = if is_ready { "deletion-ready" } else { "blocked" };

    Ok(DeletionDecision {
        decision: decision.to_string(),
        diagnostics: if is_ready {
            cleared.clone()
        } else {
            unresolved.clone()
        },
        cleared_blockers: cleared,
        unresolved_blockers: unresolved,
    })
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn resource_record_to_value(resource: &ResourceRecord) -> IoValue {
    record("resource-record-v1", vec![
        string(&resource.resource_type),
        string(&resource.resource_ref),
        string(&resource.scope_ref),
        string(&resource.name),
        u64_value(resource.generation),
        string(&resource.desired_ref),
        optional_ref_value(resource.observed_ref.as_deref()),
        resource_metadata_to_value(&resource.metadata),
        refs_sequence(&resource.evidence_refs),
    ])
}

pub fn resource_metadata_to_value(metadata: &ResourceMetadata) -> IoValue {
    let labels: Vec<IoValue> = metadata
        .labels
        .iter()
        .map(|(key, value)| record("label", vec![string(key), string(value)]))
        .collect();
    let annotations: Vec<IoValue> = metadata
        .annotations
        .iter()
        .map(|(key, value)| record("annotation", vec![string(key), string(value)]))
        .collect();
    let owner_refs: Vec<IoValue> = metadata
        .owner_refs
        .iter()
        .map(|owner| {
            record("owner-ref", vec![
                string(&owner.resource_ref),
                string(&owner.resource_type),
                bool_value(owner.block_delete_on_deletion),
            ])
        })
        .collect();
    let finalizers: Vec<IoValue> = metadata.finalizers.iter().map(string).collect();

    record("resource-metadata-v1", vec![
        record("labels", vec![sequence(labels)]),
        record("annotations", vec![sequence(annotations)]),
        record("owner-refs", vec![sequence(owner_refs)]),
        record("finalizers", vec![sequence(finalizers)]),
        refs_sequence(&metadata.evidence_refs),
    ])
}

pub fn status_condition_to_value(condition: &StatusCondition) -> IoValue {
    record("resource-condition-v1", vec![
        u64_value(condition.observed_generation),
        string(&condition.condition_type),
        symbol(condition.status.as_str()),
        string(&condition.reason),
        symbol(condition.severity.as_str()),
        string(&condition.message),
        refs_sequence(&condition.evidence_refs),
        optional_ref_value(condition.observed_state_ref.as_deref()),
    ])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

