
pub fn plan_remote_execution_closure(input: RemoteExecutionClosurePlanInput) -> Result<RemoteExecutionClosurePlan> {
    let descriptor = parse_remote_execution_closure_descriptor(&input.closure_descriptor)?;
    validate_refs(&input.receiver_present_refs, "remote execution receiver present ref")?;
    validate_refs(&input.sender_payload_refs, "remote execution sender payload ref")?;
    let present = sorted_unique(&input.receiver_present_refs);
    let (already_present_refs, missing_refs) = partition_remote_execution_closure_refs(&descriptor.dependency_refs, &present)?;
    let sender_extra_refs = remote_execution_sender_extra_refs(&input.sender_payload_refs, &missing_refs)?;
    let diagnostics = remote_execution_sender_extra_diagnostics(&sender_extra_refs)?;
    let selected_fetch_refs = missing_refs.clone();
    let value = remote_execution_closure_plan_value(&RemoteExecutionClosurePlanValueInput {
        root_artifact_ref: &descriptor.root_artifact_ref,
        dependency_refs: &descriptor.dependency_refs,
        already_present_refs: &already_present_refs,
        missing_refs: &missing_refs,
        selected_fetch_refs: &selected_fetch_refs,
        sender_extra_refs: &sender_extra_refs,
        diagnostics: &diagnostics,
    })?;
    Ok(RemoteExecutionClosurePlan {
        plan_ref: crate::preserves_rail::canonical_hash(&value)?,
        root_artifact_ref: descriptor.root_artifact_ref,
        dependency_refs: descriptor.dependency_refs,
        already_present_refs,
        selected_fetch_refs,
        missing_refs,
        sender_extra_refs,
        diagnostics,
        value,
    })
}

pub fn parse_remote_execution_closure_plan(value: &IoValue) -> Result<RemoteExecutionClosurePlan> {
    let fields = simple_record(value, "remote-execution-closure-plan-v1", REMOTE_EXECUTION_CLOSURE_PLAN_FIELDS + 1)?;
    require_schema(&fields[0], REMOTE_EXECUTION_CLOSURE_PLAN_SCHEMA, "remote execution closure plan schema")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "receiver-selected-fetch-set", "remote execution closure plan")?;
    Ok(RemoteExecutionClosurePlan {
        plan_ref: crate::preserves_rail::canonical_hash(value)?,
        root_artifact_ref: record_ref(&fields[1], "root")?,
        dependency_refs: record_ref_sequence(&fields[2], "dependencies")?,
        already_present_refs: record_ref_sequence(&fields[3], "already-present")?,
        missing_refs: record_ref_sequence(&fields[4], "missing")?,
        selected_fetch_refs: record_ref_sequence(&fields[5], "selected-fetch")?,
        sender_extra_refs: record_ref_sequence(&fields[6], "sender-extra")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
        value: value.clone(),
    })
}

fn partition_remote_execution_closure_refs(
    dependency_refs: &[String],
    present_refs: &[String],
) -> Result<(Vec<String>, Vec<String>)> {
    let mut already_present_refs = Vec::new();
    let mut missing_refs = Vec::new();
    for dependency_ref in dependency_refs {
        if present_refs.iter().any(|reference| reference == dependency_ref) {
            push_bounded(
                &mut already_present_refs,
                dependency_ref.clone(),
                MAX_JOB_REFS,
                "remote execution present refs",
            )?;
        } else {
            push_bounded(&mut missing_refs, dependency_ref.clone(), MAX_JOB_REFS, "remote execution missing refs")?;
        }
    }
    Ok((already_present_refs, missing_refs))
}

fn remote_execution_sender_extra_refs(sender_payload_refs: &[String], missing_refs: &[String]) -> Result<Vec<String>> {
    let mut sender_extra_refs = Vec::new();
    for sender_ref in sender_payload_refs {
        if !missing_refs.iter().any(|missing_ref| missing_ref == sender_ref) {
            push_bounded(
                &mut sender_extra_refs,
                sender_ref.clone(),
                MAX_JOB_REFS,
                "remote execution sender extra refs",
            )?;
        }
    }
    Ok(sender_extra_refs)
}

fn remote_execution_sender_extra_diagnostics(sender_extra_refs: &[String]) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for reference in sender_extra_refs {
        push_bounded(
            &mut diagnostics,
            format!("sender pushed unrequested remote execution ref {reference}"),
            MAX_REMOTE_EXECUTION_DIAGNOSTICS,
            "remote execution closure diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn remote_execution_closure_plan_value(input: &RemoteExecutionClosurePlanValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.root_artifact_ref, "remote execution closure plan root ref")?;
    validate_refs(input.dependency_refs, "remote execution closure plan dependency ref")?;
    validate_refs(input.already_present_refs, "remote execution closure plan present ref")?;
    validate_refs(input.missing_refs, "remote execution closure plan missing ref")?;
    validate_refs(input.selected_fetch_refs, "remote execution closure plan selected fetch ref")?;
    validate_refs(input.sender_extra_refs, "remote execution closure plan sender extra ref")?;
    Ok(crate::preserves_rail::record("remote-execution-closure-plan-v1", vec![
        crate::preserves_rail::string(REMOTE_EXECUTION_CLOSURE_PLAN_SCHEMA),
        crate::preserves_rail::record("root", vec![crate::preserves_rail::string(input.root_artifact_ref)]),
        crate::preserves_rail::record("dependencies", vec![refs_sequence(&sorted_unique(input.dependency_refs))]),
        crate::preserves_rail::record("already-present", vec![refs_sequence(&sorted_unique(input.already_present_refs))]),
        crate::preserves_rail::record("missing", vec![refs_sequence(&sorted_unique(input.missing_refs))]),
        crate::preserves_rail::record("selected-fetch", vec![refs_sequence(&sorted_unique(input.selected_fetch_refs))]),
        crate::preserves_rail::record("sender-extra", vec![refs_sequence(&sorted_unique(input.sender_extra_refs))]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        checks_value(&["receiver-selected-fetch-set", "sender-extras-non-authority"]),
    ]))
}
