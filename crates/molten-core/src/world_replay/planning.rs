use super::*;

// r[impl molten.world_replay.transition_chain]
// r[impl molten.world_replay.capsule]
pub fn plan_world_replay(request: &WorldReplayPlanRequest) -> Result<WorldReplayPlan, Vec<WorldReplayIssue>> {
    let issues = validate_world_replay_plan_request(request);
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut plan = WorldReplayPlan {
        schema: WORLD_REPLAY_PLAN_SCHEMA.to_string(),
        plan_ref: placeholder_ref(),
        trace_ref: request.trace.trace_ref.clone(),
        capsule_ref: request.capsule.capsule_ref.clone(),
        profile: request.trace.profile.clone(),
        operations: build_operations(request)?,
        current_admission_required: true,
        non_claims: world_replay_non_claims(),
    };
    plan.plan_ref = identify_world_replay_plan(&plan).map_err(|issue| vec![issue])?;
    let plan_issues = validate_world_replay_plan(&plan);
    if !plan_issues.is_empty() {
        return Err(plan_issues);
    }
    Ok(plan)
}

fn build_operations(request: &WorldReplayPlanRequest) -> Result<Vec<WorldReplayOperation>, Vec<WorldReplayIssue>> {
    let capacity = operation_capacity(request)?;
    let mut operations = Vec::with_capacity(capacity);
    operations.extend(request.capsule.members.iter().map(|member| WorldReplayOperation {
        kind: WorldReplayOperationKind::MaterializeMember,
        position: None,
        subject_ref: member.object_ref.clone(),
    }));
    operations.push(WorldReplayOperation {
        kind: match request.trace.profile.kind {
            WorldReplayProfileKind::Logical => WorldReplayOperationKind::RestoreLogicalProfile,
            WorldReplayProfileKind::Opaque => WorldReplayOperationKind::RestoreOpaqueProfile,
        },
        position: None,
        subject_ref: request.trace.initial_commit.as_str().to_string(),
    });
    operations.push(WorldReplayOperation {
        kind: WorldReplayOperationKind::RecheckCurrentAdmission,
        position: None,
        subject_ref: request.trace.profile.profile_ref.as_str().to_string(),
    });
    operations.extend(request.trace.steps.iter().flat_map(|step| {
        [
            WorldReplayOperation {
                kind: WorldReplayOperationKind::ExecuteTransition,
                position: Some(step.position),
                subject_ref: step.input.input_ref.clone(),
            },
            WorldReplayOperation {
                kind: WorldReplayOperationKind::CaptureSuccessor,
                position: Some(step.position),
                subject_ref: step.expected_successor.as_str().to_string(),
            },
            WorldReplayOperation {
                kind: WorldReplayOperationKind::CompareSuccessor,
                position: Some(step.position),
                subject_ref: step.expected_successor.as_str().to_string(),
            },
        ]
    }));
    operations.push(WorldReplayOperation {
        kind: WorldReplayOperationKind::PublishReceipt,
        position: None,
        subject_ref: request.capsule.capsule_ref.clone(),
    });
    Ok(operations)
}

fn operation_capacity(request: &WorldReplayPlanRequest) -> Result<usize, Vec<WorldReplayIssue>> {
    let step_operations = request
        .trace
        .steps
        .len()
        .checked_mul(WORLD_REPLAY_OPERATIONS_PER_STEP)
        .ok_or_else(operation_count_issue)?;
    request
        .capsule
        .members
        .len()
        .checked_add(step_operations)
        .and_then(|count| count.checked_add(WORLD_REPLAY_FIXED_OPERATIONS))
        .ok_or_else(operation_count_issue)
}

fn operation_count_issue() -> Vec<WorldReplayIssue> {
    vec![WorldReplayIssue::InvalidBounds("operation-count")]
}

fn placeholder_ref() -> String {
    const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    format!("blake3:{ZERO_DIGEST}")
}
