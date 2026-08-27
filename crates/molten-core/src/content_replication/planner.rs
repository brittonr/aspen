#![allow(
    tigerstyle::non_trait_imports,
    reason = "the pure planner uses ordered maps and sets to keep placement and evidence deterministic"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::action::TransferBuild;
use super::action::build_reuse;
use super::action::build_transfer;
use super::action::denied_plan;
use super::*;

pub fn plan(input: &ReconcileInput) -> Result<Plan, Issue> {
    let issues = validate_input(input);
    if !issues.is_empty() {
        return denied_plan(input, issues);
    }
    let mut planner = PlanningState::new(input);
    let mut contents = input.manifest.contents.iter().collect::<Vec<_>>();
    contents.sort();
    for content in contents {
        planner.reconcile(content)?;
    }
    planner.finish()
}

pub(super) struct PlanningState<'a> {
    pub(super) input: &'a ReconcileInput,
    pub(super) history: BTreeMap<&'a str, &'a PriorOperation>,
    pub(super) actions: Vec<Action>,
    pub(super) under_replicated: BTreeSet<String>,
    pub(super) deferred: BTreeSet<String>,
    pub(super) required_pins: BTreeSet<String>,
    pub(super) cleanup_candidates: BTreeSet<String>,
    pub(super) issues: BTreeSet<Issue>,
    pub(super) verified_replicas: usize,
    pub(super) planned_transfers: usize,
    pub(super) planned_bytes: u64,
}

impl<'a> PlanningState<'a> {
    fn new(input: &'a ReconcileInput) -> Self {
        Self {
            input,
            history: input.history.iter().map(|operation| (operation.operation_id.as_str(), operation)).collect(),
            actions: Vec::with_capacity(input.manifest.resources.max_queue_depth),
            under_replicated: BTreeSet::new(),
            deferred: BTreeSet::new(),
            required_pins: BTreeSet::new(),
            cleanup_candidates: BTreeSet::new(),
            issues: BTreeSet::new(),
            verified_replicas: 0,
            planned_transfers: 0,
            planned_bytes: 0,
        }
    }

    fn reconcile(&mut self, content: &ReplicaRule) -> Result<(), Issue> {
        let current = self.current_replicas(content);
        self.verified_replicas = self.verified_replicas.saturating_add(current.len());
        let desired = self.input.manifest.policy.desired_replicas;
        if current.len() < desired {
            self.under_replicated.insert(content.content_ref.clone());
            self.plan_missing(content, &current, desired.saturating_sub(current.len()))?;
        } else if current.len() > desired {
            self.plan_cleanup(content, &current, current.len().saturating_sub(desired))?;
        }
        Ok(())
    }

    fn current_replicas(&self, content: &ReplicaRule) -> Vec<&'a Replica> {
        let mut replicas = self
            .input
            .inventory
            .replicas
            .iter()
            .filter(|replica| {
                replica.content_ref == content.content_ref
                    && replica.generation == self.input.manifest.generation
                    && replica.membership_epoch == self.input.manifest.membership_epoch
                    && replica.placement_epoch == self.input.manifest.placement_epoch
                    && replica.present
                    && replica.identity_verified
                    && replica.manifest_ref == content.manifest_ref
                    && replica.protected == content.protected
            })
            .collect::<Vec<_>>();
        replicas.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));
        replicas
    }

    fn plan_missing(&mut self, content: &ReplicaRule, current: &[&Replica], needed: usize) -> Result<(), Issue> {
        let Some(source) = self.verified_source(content) else {
            let issue = if self.has_protected_form_mismatch(content) {
                Issue::ProtectedFormMismatch
            } else {
                Issue::NoVerifiedSource
            };
            return self.defer(content, issue);
        };
        let mut current_peers = current.iter().map(|replica| replica.peer_id.as_str()).collect::<BTreeSet<_>>();
        current_peers.insert(source.peer_id.as_str());
        let current_domains = current.iter().map(|replica| replica.fault_domain.as_str()).collect::<BTreeSet<_>>();
        let mut candidates = self.current_peers(content, &current_peers);
        candidates.sort_by_key(|peer| {
            (
                current_domains.contains(peer.fault_domain.as_str()),
                peer.fault_domain.as_str(),
                peer.peer_id.as_str(),
            )
        });
        if candidates.len() < needed {
            self.issues.insert(Issue::InsufficientPeers);
        }
        let selected = candidates.into_iter().take(needed).collect::<Vec<_>>();
        let selected_domains = selected.iter().map(|peer| peer.fault_domain.as_str()).collect::<BTreeSet<_>>();
        let domain_total = current_domains.union(&selected_domains).count();
        if domain_total < self.input.manifest.policy.minimum_fault_domains {
            self.issues.insert(Issue::InsufficientFaultDomains);
            return self.defer(content, Issue::InsufficientFaultDomains);
        }
        for target in selected {
            self.plan_transfer(content, source, target)?;
        }
        Ok(())
    }

    fn current_peers(&self, content: &ReplicaRule, current: &BTreeSet<&str>) -> Vec<&'a Peer> {
        self.input
            .peers
            .iter()
            .filter(|peer| {
                peer.available
                    && peer.membership_epoch == self.input.manifest.membership_epoch
                    && peer.placement_epoch == self.input.manifest.placement_epoch
                    && peer.capacity_bytes >= content.encoded_bytes
                    && !current.contains(peer.peer_id.as_str())
            })
            .collect()
    }

    fn has_protected_form_mismatch(&self, content: &ReplicaRule) -> bool {
        self.input.inventory.replicas.iter().any(|replica| {
            replica.content_ref == content.content_ref
                && replica.present
                && replica.identity_verified
                && replica.manifest_ref == content.manifest_ref
                && replica.protected != content.protected
        })
    }

    fn verified_source(&self, content: &ReplicaRule) -> Option<&'a Replica> {
        let mut sources = self
            .input
            .inventory
            .replicas
            .iter()
            .filter(|replica| {
                replica.content_ref == content.content_ref
                    && replica.present
                    && replica.identity_verified
                    && replica.manifest_ref == content.manifest_ref
                    && replica.protected == content.protected
            })
            .collect::<Vec<_>>();
        sources.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));
        sources.into_iter().next()
    }

    fn plan_transfer(&mut self, content: &ReplicaRule, source: &Replica, target: &Peer) -> Result<(), Issue> {
        let kind = self.transfer_kind(content, target);
        let attempt = self.next_attempt(content, target, kind);
        if attempt > self.input.manifest.repair.max_attempts {
            return self.defer(content, Issue::RepairExhausted);
        }
        if !self.resource_available(content.encoded_bytes) {
            return self.defer(content, Issue::ByteBudgetExhausted);
        }
        let operation_id = identify_operation(OperationFrame {
            manifest: &self.input.manifest,
            content,
            source_peer: Some(&source.peer_id),
            target_peer: &target.peer_id,
            kind,
            attempt,
        })?;
        if let Some(prior) = self.history.get(operation_id.as_str()) {
            return self.reuse_or_conflict(content, source, target, prior);
        }
        let action = build_transfer(TransferBuild {
            content,
            source,
            target,
            kind,
            attempt,
            operation_id,
        })?;
        self.record_transfer(action, content)
    }

    fn transfer_kind(&self, content: &ReplicaRule, target: &Peer) -> ActionKind {
        let prior = self.input.inventory.replicas.iter().find(|replica| {
            replica.content_ref == content.content_ref && replica.peer_id == target.peer_id && replica.present
        });
        match prior {
            Some(replica) if !replica.identity_verified => ActionKind::Repair,
            Some(_) if self.input.manifest.repair.allow_handoff => ActionKind::Handoff,
            _ => ActionKind::Transfer,
        }
    }

    fn next_attempt(&self, content: &ReplicaRule, target: &Peer, _kind: ActionKind) -> u32 {
        let matching = self.input.history.iter().filter(|operation| {
            operation.content_ref == content.content_ref
                && operation.target_peer == target.peer_id
                && operation.generation == self.input.manifest.generation
                && operation.membership_epoch == self.input.manifest.membership_epoch
                && operation.placement_epoch == self.input.manifest.placement_epoch
        });
        if let Some(verified) = matching.clone().find(|operation| operation.outcome == OperationOutcome::Verified) {
            return verified.attempt;
        }
        matching.map(|operation| operation.attempt).max().map_or(1, |attempt| attempt.saturating_add(1))
    }

    fn resource_available(&self, encoded_bytes: u64) -> bool {
        self.planned_transfers < self.input.manifest.resources.max_concurrent_transfers
            && self.actions.len() < self.input.manifest.resources.max_queue_depth
            && self.planned_bytes.checked_add(encoded_bytes).is_some_and(|bytes| {
                bytes <= self.input.manifest.resources.max_transfer_bytes && bytes <= MAX_REPLICATION_BYTES
            })
    }

    fn reuse_or_conflict(
        &mut self,
        content: &ReplicaRule,
        source: &Replica,
        target: &Peer,
        prior: &PriorOperation,
    ) -> Result<(), Issue> {
        if prior.content_ref != content.content_ref
            || prior.source_peer.as_deref() != Some(source.peer_id.as_str())
            || prior.target_peer != target.peer_id
            || prior.generation != self.input.manifest.generation
            || prior.membership_epoch != self.input.manifest.membership_epoch
            || prior.placement_epoch != self.input.manifest.placement_epoch
        {
            self.issues.insert(Issue::ConflictingOperation);
            return Err(Issue::ConflictingOperation);
        }
        if prior.outcome == OperationOutcome::Verified {
            let action = build_reuse(content, source, target, prior)?;
            self.actions.push(action);
            return Ok(());
        }
        self.defer(content, Issue::ConflictingOperation)
    }

    fn record_transfer(&mut self, action: Action, content: &ReplicaRule) -> Result<(), Issue> {
        self.planned_transfers = self.planned_transfers.checked_add(1).ok_or(Issue::TooManyActions)?;
        self.planned_bytes = self.planned_bytes.checked_add(content.encoded_bytes).ok_or(Issue::ByteBudgetExhausted)?;
        self.required_pins.insert(content.content_ref.clone());
        self.actions.push(action);
        Ok(())
    }
}
