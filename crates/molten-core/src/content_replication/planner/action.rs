use super::super::*;

impl super::PlanningState<'_> {
    // r[impl molten.content_replication.retention_confidentiality]
    // r[impl molten.audit_f05.residual_policy]
    // r[impl molten.audit_f05.selection]
    pub(super) fn plan_cleanup(
        &mut self,
        content: &ReplicaRule,
        current: &[&Replica],
        excess: usize,
    ) -> Result<(), Issue> {
        self.cleanup_candidates.insert(content.content_ref.clone());
        if !self.input.manifest.repair.cleanup_after_handoff {
            return self.defer(content, Issue::ActiveRetentionPin);
        }
        let Some(authority_ref) = content.cleanup_authority_ref.as_ref() else {
            return self.defer(content, Issue::MissingCleanupAuthority);
        };
        let policy = &self.input.manifest.policy;
        let mut domain_counts = std::collections::BTreeMap::new();
        for replica in current {
            let count = domain_counts.entry(replica.fault_domain.as_str()).or_insert(0_usize);
            *count = count.saturating_add(1);
        }
        let mut remaining_domains = domain_counts.len();
        let mut remaining_replicas = current.len();
        let mut selected = 0_usize;
        let mut unresolved = None;
        for replica in current.iter().rev() {
            if selected == excess {
                break;
            }
            let residual_count = remaining_replicas.saturating_sub(1);
            if residual_count < policy.desired_replicas || residual_count < policy.minimum_verified_replicas {
                unresolved.get_or_insert(Issue::InsufficientPeers);
                continue;
            }
            let domain_count = domain_counts.get_mut(replica.fault_domain.as_str()).ok_or(Issue::InvalidPolicy)?;
            let residual_domains = remaining_domains.saturating_sub(usize::from(*domain_count == 1));
            if residual_domains < policy.minimum_fault_domains {
                unresolved.get_or_insert(Issue::InsufficientFaultDomains);
                continue;
            }
            if replica.pinned {
                unresolved.get_or_insert(Issue::ActiveRetentionPin);
                continue;
            }
            let Some(clearance_ref) = replica.cleanup_clearance_ref.as_ref() else {
                unresolved.get_or_insert(Issue::MissingRetentionPin);
                continue;
            };
            if self.actions.len() >= self.input.manifest.resources.max_queue_depth {
                return Err(Issue::QueueExhausted);
            }
            self.actions
                .push(build_cleanup(&self.input.manifest, content, replica, authority_ref, clearance_ref)?);
            *domain_count = domain_count.saturating_sub(1);
            remaining_domains = residual_domains;
            remaining_replicas = residual_count;
            selected = selected.saturating_add(1);
        }
        if selected < excess {
            return self.defer(content, unresolved.unwrap_or(Issue::InsufficientPeers));
        }
        Ok(())
    }

    // r[impl molten.content_replication.resources_failures]
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
        let plan_ref =
            identify_plan(&self.input.manifest, self.input.observed_tick, &self.actions, &self.replica_deficits)?;
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
            under_replicated: self.replica_deficits.iter().map(|deficit| deficit.content_ref.clone()).collect(),
            replica_deficits: self.replica_deficits,
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

// r[impl molten.content_replication.retention_confidentiality]
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
    let mut replica_deficits = input
        .manifest
        .contents
        .iter()
        .map(|content| ReplicaDeficit {
            content_ref: content.content_ref.clone(),
            missing_replicas: input.manifest.policy.desired_replicas,
        })
        .collect::<Vec<_>>();
    replica_deficits.sort();
    let plan_ref = identify_plan(&input.manifest, input.observed_tick, &actions, &replica_deficits)?;
    Ok(Plan {
        plan_ref,
        decision: Decision::Denied,
        generation: input.manifest.generation,
        membership_epoch: input.manifest.membership_epoch,
        placement_epoch: input.manifest.placement_epoch,
        actions,
        desired_replicas: input.manifest.policy.desired_replicas.saturating_mul(input.manifest.contents.len()),
        verified_replicas: 0,
        under_replicated: replica_deficits.iter().map(|deficit| deficit.content_ref.clone()).collect(),
        replica_deficits,
        deferred: Vec::new(),
        required_pins: Vec::new(),
        cleanup_candidates: Vec::new(),
        issues,
        non_claims: NON_CLAIMS.iter().map(ToString::to_string).collect(),
    })
}
