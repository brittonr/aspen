use super::*;

impl super::planner::PlanningState<'_> {
    pub(super) fn plan_cleanup(
        &mut self,
        content: &ReplicaRule,
        current: &[&Replica],
        excess: usize,
    ) -> Result<(), Issue> {
        for replica in current.iter().rev().take(excess) {
            self.cleanup_candidates.insert(content.content_ref.clone());
            if replica.pinned || !self.input.manifest.repair.cleanup_after_handoff {
                self.defer(content, Issue::ActiveRetentionPin)?;
                continue;
            }
            let Some(authority_ref) = content.cleanup_authority_ref.as_ref() else {
                self.defer(content, Issue::MissingCleanupAuthority)?;
                continue;
            };
            let Some(clearance_ref) = replica.cleanup_clearance_ref.as_ref() else {
                self.defer(content, Issue::MissingRetentionPin)?;
                continue;
            };
            let action = build_cleanup(&self.input.manifest, content, replica, authority_ref, clearance_ref)?;
            self.actions.push(action);
        }
        Ok(())
    }

    pub(super) fn defer(&mut self, content: &ReplicaRule, issue: Issue) -> Result<(), Issue> {
        self.issues.insert(issue);
        self.deferred.insert(content.content_ref.clone());
        if self.actions.len() >= self.input.manifest.resources.max_queue_depth {
            return Err(Issue::QueueExhausted);
        }
        self.actions.push(build_defer(&self.input.manifest, content, issue)?);
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<Plan, Issue> {
        self.actions.sort();
        if self.actions.len() > MAX_ACTIONS {
            return Err(Issue::TooManyActions);
        }
        let decision = if self.deferred.is_empty() {
            Decision::Ready
        } else {
            Decision::Partial
        };
        let plan_ref = identify_plan(&self.input.manifest, self.input.observed_tick, &self.actions)?;
        Ok(Plan {
            plan_ref,
            decision,
            generation: self.input.manifest.generation,
            membership_epoch: self.input.manifest.membership_epoch,
            placement_epoch: self.input.manifest.placement_epoch,
            actions: self.actions,
            desired_replicas: self
                .input
                .manifest
                .policy
                .desired_replicas
                .saturating_mul(self.input.manifest.contents.len()),
            verified_replicas: self.verified_replicas,
            under_replicated: self.under_replicated.into_iter().collect(),
            deferred: self.deferred.into_iter().collect(),
            required_pins: self.required_pins.into_iter().collect(),
            cleanup_candidates: self.cleanup_candidates.into_iter().collect(),
            issues: self.issues.into_iter().collect(),
            non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
        })
    }
}

pub(super) struct TransferBuild<'a> {
    pub content: &'a ReplicaRule,
    pub source: &'a Replica,
    pub target: &'a Peer,
    pub kind: ActionKind,
    pub attempt: u32,
    pub operation_id: String,
}

pub(super) fn build_transfer(input: TransferBuild<'_>) -> Result<Action, Issue> {
    Ok(Action {
        action_id: identify_action(&input.operation_id, input.kind)?,
        operation_id: input.operation_id,
        kind: input.kind,
        attempt: input.attempt,
        content_ref: input.content.content_ref.clone(),
        source_peer: Some(input.source.peer_id.clone()),
        target_peer: input.target.peer_id.clone(),
        fault_domain: input.target.fault_domain.clone(),
        encoded_bytes: input.content.encoded_bytes,
        pin_required: true,
        preserve_protected_form: input.content.protected,
        cleanup_authority_ref: None,
        prior_result_ref: None,
        diagnostic: None,
    })
}

pub(super) fn build_reuse(
    content: &ReplicaRule,
    source: &Replica,
    target: &Peer,
    prior: &PriorOperation,
) -> Result<Action, Issue> {
    Ok(Action {
        action_id: identify_action(&prior.operation_id, ActionKind::Reuse)?,
        operation_id: prior.operation_id.clone(),
        kind: ActionKind::Reuse,
        attempt: prior.attempt,
        content_ref: content.content_ref.clone(),
        source_peer: Some(source.peer_id.clone()),
        target_peer: target.peer_id.clone(),
        fault_domain: target.fault_domain.clone(),
        encoded_bytes: content.encoded_bytes,
        pin_required: true,
        preserve_protected_form: content.protected,
        cleanup_authority_ref: None,
        prior_result_ref: prior.result_ref.clone(),
        diagnostic: None,
    })
}

fn build_cleanup(
    manifest: &Manifest,
    content: &ReplicaRule,
    replica: &Replica,
    authority_ref: &str,
    clearance_ref: &str,
) -> Result<Action, Issue> {
    let operation_id = identify_operation(OperationFrame {
        manifest,
        content,
        source_peer: Some(&replica.peer_id),
        target_peer: &replica.peer_id,
        kind: ActionKind::Cleanup,
        attempt: 1,
    })?;
    Ok(Action {
        action_id: identify_action(&operation_id, ActionKind::Cleanup)?,
        operation_id,
        kind: ActionKind::Cleanup,
        attempt: 1,
        content_ref: content.content_ref.clone(),
        source_peer: Some(replica.peer_id.clone()),
        target_peer: replica.peer_id.clone(),
        fault_domain: replica.fault_domain.clone(),
        encoded_bytes: content.encoded_bytes,
        pin_required: false,
        preserve_protected_form: content.protected,
        cleanup_authority_ref: Some(authority_ref.to_string()),
        prior_result_ref: Some(clearance_ref.to_string()),
        diagnostic: None,
    })
}

fn build_defer(manifest: &Manifest, content: &ReplicaRule, issue: Issue) -> Result<Action, Issue> {
    let target = "unassigned";
    let operation_id = identify_operation(OperationFrame {
        manifest,
        content,
        source_peer: None,
        target_peer: target,
        kind: ActionKind::Defer,
        attempt: 1,
    })?;
    Ok(Action {
        action_id: identify_action(&operation_id, ActionKind::Defer)?,
        operation_id,
        kind: ActionKind::Defer,
        attempt: 1,
        content_ref: content.content_ref.clone(),
        source_peer: None,
        target_peer: target.to_string(),
        fault_domain: "unassigned".to_string(),
        encoded_bytes: content.encoded_bytes,
        pin_required: false,
        preserve_protected_form: content.protected,
        cleanup_authority_ref: None,
        prior_result_ref: None,
        diagnostic: Some(issue.as_str().to_string()),
    })
}

pub(super) fn denied_plan(input: &ReconcileInput, issues: Vec<Issue>) -> Result<Plan, Issue> {
    let actions = Vec::new();
    let plan_ref = identify_plan(&input.manifest, input.observed_tick, &actions)?;
    Ok(Plan {
        plan_ref,
        decision: Decision::Denied,
        generation: input.manifest.generation,
        membership_epoch: input.manifest.membership_epoch,
        placement_epoch: input.manifest.placement_epoch,
        actions,
        desired_replicas: input.manifest.policy.desired_replicas.saturating_mul(input.manifest.contents.len()),
        verified_replicas: 0,
        under_replicated: input.manifest.contents.iter().map(|content| content.content_ref.clone()).collect(),
        deferred: Vec::new(),
        required_pins: Vec::new(),
        cleanup_candidates: Vec::new(),
        issues,
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    })
}

pub fn status(plan: &Plan, history: &[PriorOperation]) -> Status {
    let terminal = history
        .iter()
        .map(|operation| (operation.operation_id.as_str(), operation))
        .collect::<std::collections::BTreeMap<_, _>>();
    let data_actions = plan.actions.iter().filter(|action| {
        matches!(action.kind, ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff | ActionKind::Reuse)
    });
    let successful = data_actions
        .clone()
        .filter(|action| {
            terminal
                .get(action.operation_id.as_str())
                .is_some_and(|operation| operation.outcome == OperationOutcome::Verified)
        })
        .collect::<Vec<_>>();
    let successful_ids = successful
        .iter()
        .map(|action| action.operation_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut active_operations = data_actions
        .filter(|action| !terminal.contains_key(action.operation_id.as_str()))
        .map(|action| action.operation_id.clone())
        .collect::<Vec<_>>();
    active_operations.sort();
    let plan_operation_ids = plan
        .actions
        .iter()
        .map(|action| action.operation_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut failures = terminal
        .values()
        .filter(|operation| {
            plan_operation_ids.contains(operation.operation_id.as_str())
                && operation.outcome != OperationOutcome::Verified
        })
        .map(|operation| operation.operation_id.clone())
        .collect::<Vec<_>>();
    failures.sort();
    let under_replicated = plan
        .under_replicated
        .iter()
        .filter(|content_ref| {
            plan.actions.iter().any(|action| {
                action.content_ref == **content_ref
                    && (action.kind == ActionKind::Defer
                        || (matches!(
                            action.kind,
                            ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff | ActionKind::Reuse
                        ) && !successful_ids.contains(action.operation_id.as_str())))
            })
        })
        .cloned()
        .collect();
    Status {
        plan_ref: plan.plan_ref.clone(),
        generation: plan.generation,
        placement_epoch: plan.placement_epoch,
        desired_replicas: plan.desired_replicas,
        verified_replicas: plan.verified_replicas.saturating_add(successful.len()).min(plan.desired_replicas),
        under_replicated,
        active_operations,
        failures,
        pins: plan.required_pins.clone(),
        non_claims: plan.non_claims.clone(),
    }
}
