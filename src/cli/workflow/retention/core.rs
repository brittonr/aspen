pub(crate) fn class(args: super::command::base::Class) -> molten::error::Result<()> {
    let super::command::base::Class {
        class_name,
        minimum_age_seconds,
        maximum_age_seconds,
        deletion_authority_ref,
        policy_refs,
        has_secret_redaction_hook,
        has_remote_gc_plan,
        has_compaction,
        out,
    } = args;
    let value = molten::retention::class_profile_value(&molten::retention::ClassProfileInput {
        class_name: class_name.clone(),
        minimum_age_seconds,
        maximum_age_seconds,
        deletion_authority_ref,
        policy_refs,
        has_secret_redaction_hook,
        has_remote_gc_plan,
        can_compact: has_compaction,
    })?;
    let profile = molten::retention::parse_class_profile(&value)?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("retention class ref={} class={}", profile.profile_ref, profile.class_name),
    );
    Ok(())
}

pub(crate) fn pin(args: super::command::base::Pin) -> molten::error::Result<()> {
    let super::command::base::Pin {
        root,
        object_ref,
        object_kind,
        retention_class,
        source,
        reason,
        owner_ref,
        expiry_ref,
        policy_refs,
        evidence_refs,
        has_authority,
        pin_out,
        receipt_out,
    } = args;
    let operation = molten::retention::pin_object(&root, molten::retention::RetentionPinInput {
        object_ref,
        object_kind,
        retention_class,
        source,
        reason,
        owner_ref,
        expiry_ref,
        policy_refs,
        evidence_refs,
        has_authority,
    })?;
    super::io::write_optional_preserves(pin_out.as_ref(), &operation.pin.value)?;
    let is_receipt_written = super::io::write_optional_preserves(receipt_out.as_ref(), &operation.receipt.value)?;
    super::io::print_or_log_summary(
        is_receipt_written,
        &format!(
            "retention pin decision={} pin={} receipt={}",
            operation.receipt.decision, operation.pin.pin_ref, operation.receipt.receipt_ref
        ),
    );
    Ok(())
}

pub(crate) fn unpin(args: super::command::base::Unpin) -> molten::error::Result<()> {
    let super::command::base::Unpin {
        root,
        pin_ref,
        requester_ref,
        policy_refs,
        evidence_refs,
        has_authority,
        receipt_out,
    } = args;
    let receipt = molten::retention::unpin_object(molten::retention::UnpinObjectInput {
        root: &root,
        pin_ref: &pin_ref,
        requester_ref: &requester_ref,
        policy_refs: &policy_refs,
        evidence_refs: &evidence_refs,
        has_authority,
    })?;
    let is_written_to_file = super::io::write_optional_preserves(receipt_out.as_ref(), &receipt.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!("retention unpin decision={} pin={} receipt={}", receipt.decision, pin_ref, receipt.receipt_ref),
    );
    Ok(())
}

pub(crate) fn admit(args: super::command::base::Admit) -> molten::error::Result<()> {
    let super::command::base::Admit {
        root,
        kind,
        decision,
        requester_ref,
        object_ref,
        object_kind,
        retention_class,
        action,
        bound_refs,
        retained_refs,
        remote_refs,
        is_reference_index_complete,
        is_stale,
        revoked_refs,
        diagnostics,
        out,
    } = args;
    let admission = molten::retention::store_retention_evidence_admission(
        &root,
        &molten::retention::RetentionEvidenceAdmissionInput {
            kind: &kind,
            decision: &decision,
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            object_kind: &object_kind,
            retention_class: &retention_class,
            action: &action,
            bound_refs: &bound_refs,
            retained_refs: &retained_refs,
            remote_refs: &remote_refs,
            is_reference_index_complete,
            is_current: !is_stale,
            revoked_refs: &revoked_refs,
            diagnostics: &diagnostics,
        },
    )?;
    let is_written_to_file = super::io::write_optional_preserves(out.as_ref(), &admission.value)?;
    super::io::print_or_log_summary(
        is_written_to_file,
        &format!(
            "retention admission ref={} kind={} decision={}",
            admission.admission_ref, admission.kind, admission.decision
        ),
    );
    Ok(())
}
