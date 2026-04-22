//! Trust initialization for Shamir cluster secret sharing.
//!
//! When trust is enabled during cluster init, the leader:
//! 1. Generates a 32-byte cluster root secret
//! 2. Splits it into K-of-N Shamir shares
//! 3. Computes SHA3-256 digests for each share
//! 4. Commits a `TrustInitialize` Raft request carrying per-node share bytes
//! 5. Lets each node persist its own share when the request is applied locally

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use aspen_cluster_types::InitRequest;
use aspen_cluster_types::NodeAddress;
use aspen_transport::TrustShareProvider;
use aspen_trust::chain;
use aspen_trust::protocol::ShareResponse;
use aspen_trust::protocol::TrustResponse;
use aspen_trust::reconfig::ReconfigAction;
use aspen_trust::reconfig::ReconfigCoordinator;
use aspen_trust::reconfig::ReconfigParams;
use aspen_trust::secret::ClusterSecret;
use aspen_trust::secret::Threshold;
use aspen_trust::shamir;
use async_trait::async_trait;
use iroh::EndpointId;
use rand::rngs::ThreadRng;
use tokio::task::JoinSet;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::RaftNode;
use crate::StateMachineVariant;
use crate::trust_share_client::ExpungedByPeer;
use crate::trust_share_client::TrustShareClient;
use crate::types::AppRequest;
use crate::types::TrustInitializePayload;
use crate::types::TrustReconfigurationPayload;

type PendingTrustReconfiguration = (BTreeSet<u64>, BTreeSet<u64>, u64);

struct TimeoutContext {
    old_epoch: u64,
    membership_epoch: u64,
    old_threshold: u8,
    new_threshold: u8,
    old_members: BTreeSet<u64>,
    new_members: BTreeSet<u64>,
    expected_digests: BTreeMap<u64, shamir::ShareDigest>,
    cluster_id: Vec<u8>,
}

struct ShareCollectionPlan {
    client: Arc<dyn TrustShareClient>,
    old_epoch: u64,
    old_threshold: u8,
    old_members: BTreeSet<u64>,
    old_member_addresses: BTreeMap<u64, NodeAddress>,
    local_node_id: u64,
    local_share: Option<shamir::Share>,
    expected_digests: BTreeMap<u64, shamir::ShareDigest>,
    timeout: Duration,
}

struct PeerExpungementProbePlan {
    current_epoch: u64,
    peers: BTreeMap<u64, NodeAddress>,
    expected_digests: BTreeMap<u64, shamir::ShareDigest>,
    required_peer_confirmations: usize,
}

struct TrustRotationContext {
    client: Arc<dyn TrustShareClient>,
    cluster_id: Vec<u8>,
    old_epoch: u64,
    local_share: Option<shamir::Share>,
    expected_digests: BTreeMap<u64, shamir::ShareDigest>,
    threshold_override: Option<u8>,
    old_threshold: u8,
    new_threshold: u8,
    old_member_addresses: BTreeMap<u64, NodeAddress>,
    timeout_context: TimeoutContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeerExpungementProbeOutcome {
    NotApplicable,
    Healthy,
    RetryNeeded,
    Expunged { epoch: u64 },
}

pub(crate) fn build_trust_initialize_payload(request: &InitRequest) -> Result<TrustInitializePayload, String> {
    let total_members = request.initial_members.len();
    let threshold = resolve_trust_threshold(total_members, request.trust.threshold)?;

    if threshold.value() as usize > total_members {
        return Err(format!("threshold {} exceeds cluster size {total_members}", threshold.value()));
    }

    info!(
        threshold = threshold.value(),
        total = total_members,
        "generating cluster secret and splitting into shares"
    );

    let secret = ClusterSecret::generate();
    let mut rng: ThreadRng = rand::rng();
    let shares = shamir::split_secret(secret.as_bytes(), threshold.value(), total_members as u8, &mut rng)
        .map_err(|e| format!("failed to split secret: {e}"))?;

    let shares: Vec<(u64, Vec<u8>)> = request
        .initial_members
        .iter()
        .zip(shares.iter())
        .map(|(member, share)| (member.id, share.to_bytes().to_vec()))
        .collect();

    let digests: Vec<(u64, shamir::ShareDigest)> = request
        .initial_members
        .iter()
        .zip(shares_from_payload(&shares)?.iter())
        .map(|(member, share)| (member.id, shamir::share_digest(share)))
        .collect();

    let members = request
        .initial_members
        .iter()
        .map(|member| {
            let node_addr = member
                .node_addr
                .clone()
                .ok_or_else(|| format!("missing node address for trust member {}", member.id))?;
            Ok((member.id, node_addr))
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(TrustInitializePayload {
        epoch: 1,
        threshold_override: request.trust.threshold,
        shares,
        digests,
        members,
    })
}

fn shares_from_payload(shares: &[(u64, Vec<u8>)]) -> Result<Vec<shamir::Share>, String> {
    let mut decoded = Vec::with_capacity(shares.len());
    for (_node_id, share_bytes) in shares {
        if share_bytes.len() != shamir::SECRET_SIZE + 1 {
            return Err(format!("invalid share length {}, expected {}", share_bytes.len(), shamir::SECRET_SIZE + 1));
        }
        let mut array = [0u8; shamir::SECRET_SIZE + 1];
        array.copy_from_slice(share_bytes);
        let share = shamir::Share::from_bytes(&array).map_err(|e| format!("failed to decode share bytes: {e}"))?;
        decoded.push(share);
    }
    Ok(decoded)
}

const TRUST_SHARE_COLLECTION_TIMEOUT_MS: u64 = 10_000;
const TRUST_SHARE_COLLECTION_TIMEOUT: Duration = Duration::from_millis(TRUST_SHARE_COLLECTION_TIMEOUT_MS);

fn resolve_trust_threshold(member_count: usize, threshold_override: Option<u8>) -> Result<Threshold, String> {
    let count_u32 =
        u32::try_from(member_count).map_err(|_| "too many trust members for reconfiguration".to_string())?;
    let threshold = match threshold_override {
        Some(value) => Threshold::new(value).ok_or_else(|| "threshold must be >= 1".to_string())?,
        None => Threshold::default_for_cluster_size(count_u32),
    };
    if usize::from(threshold.value()) > member_count {
        return Err(format!("threshold {} exceeds cluster size {member_count}", threshold.value()));
    }
    Ok(threshold)
}

fn validate_threshold_invariant(member_count: usize, threshold: u8, label: &str) -> Result<(), String> {
    if usize::from(threshold) > member_count {
        return Err(format!("{label} threshold {threshold} exceeds cluster size {member_count}"));
    }
    Ok(())
}

fn share_collection_target(plan: &ShareCollectionPlan) -> Result<usize, String> {
    validate_threshold_invariant(plan.old_members.len(), plan.old_threshold, "old")?;
    Ok(usize::from(plan.old_threshold))
}

fn required_peer_confirmations(peer_count: usize, threshold: u8) -> usize {
    usize::min(peer_count, usize::from(threshold))
}

fn proposal_from_actions(
    actions: Vec<ReconfigAction>,
    threshold_override: Option<u8>,
    members: Vec<(u64, NodeAddress)>,
) -> Option<TrustReconfigurationPayload> {
    for action in actions {
        if let ReconfigAction::ProposeNewConfig {
            new_shares,
            new_digests,
            encrypted_chain,
            new_epoch,
        } = action
        {
            return Some(TrustReconfigurationPayload {
                epoch: new_epoch,
                threshold_override,
                shares: new_shares.into_iter().map(|(node_id, share)| (node_id, share.to_bytes().to_vec())).collect(),
                digests: new_digests.into_iter().collect(),
                members,
                encrypted_chain,
            });
        }
    }

    None
}

fn build_timeout_error(context: &TimeoutContext, collected_shares: &BTreeMap<u64, shamir::Share>) -> String {
    let mut coordinator = ReconfigCoordinator::new(ReconfigParams {
        old_epoch: context.old_epoch,
        new_epoch: context.membership_epoch,
        old_threshold: context.old_threshold,
        new_threshold: context.new_threshold,
        old_members: context.old_members.clone(),
        new_members: context.new_members.clone(),
        expected_digests: context.expected_digests.clone(),
        cluster_id: context.cluster_id.clone(),
        prior_secrets: BTreeMap::new(),
    });

    for (node_id, share) in collected_shares {
        if let Err(error) = coordinator.on_share_received(*node_id, share.clone()) {
            return error.to_string();
        }
    }

    match coordinator.on_timeout() {
        Err(error) => error.to_string(),
        Ok(()) => "trust share collection timed out".to_string(),
    }
}

fn validate_share_response(
    node_id: u64,
    old_epoch: u64,
    response: ShareResponse,
    expected_digests: &BTreeMap<u64, shamir::ShareDigest>,
) -> Option<shamir::Share> {
    if response.epoch != old_epoch {
        warn!(
            node_id,
            returned_epoch = response.epoch,
            expected_epoch = old_epoch,
            "ignoring trust share for wrong epoch"
        );
        return None;
    }

    if let Some(expected_digest) = expected_digests.get(&node_id) {
        let actual_digest = shamir::share_digest(&response.share);
        if &actual_digest != expected_digest {
            warn!(node_id, "ignoring trust share with digest mismatch");
            return None;
        }
    }

    Some(response.share)
}

fn response_reports_current_epoch(response: &ShareResponse, expected_current_epoch: u64) -> bool {
    response.current_epoch == expected_current_epoch
}

fn node_endpoint_addr(node_addr: &NodeAddress) -> Result<iroh::EndpointAddr, String> {
    node_addr
        .try_into_iroh()
        .map_err(|error| format!("invalid node address {}: {error}", node_addr.endpoint_id()))
}

/// Spawn async tasks to request shares from all non-local old members.
///
/// Skips members with missing stored endpoints (logs a warning instead).
fn spawn_peer_share_requests(
    plan: &ShareCollectionPlan,
    pending_requests: &mut JoinSet<(u64, anyhow::Result<ShareResponse>)>,
) {
    let old_epoch = plan.old_epoch;
    for old_member in &plan.old_members {
        if *old_member == plan.local_node_id {
            continue;
        }
        let Some(node_addr) = plan.old_member_addresses.get(old_member).cloned() else {
            warn!(
                node_id = *old_member,
                epoch = old_epoch,
                "missing stored node address for trust share request; waiting for timeout"
            );
            continue;
        };
        let endpoint = match node_endpoint_addr(&node_addr) {
            Ok(endpoint) => endpoint,
            Err(error) => {
                warn!(node_id = *old_member, epoch = old_epoch, error = %error, "invalid stored node address");
                continue;
            }
        };
        let client = plan.client.clone();
        let target_node = *old_member;
        pending_requests.spawn(async move {
            let response = client
                .get_share(endpoint, old_epoch)
                .await
                .with_context(|| format!("failed to fetch share from node {target_node}"));
            (target_node, response)
        });
    }
}

async fn collect_old_shares_for_reconfiguration(
    plan: ShareCollectionPlan,
) -> Result<BTreeMap<u64, shamir::Share>, String> {
    let target_share_count = share_collection_target(&plan)?;
    let mut collected = BTreeMap::new();
    let mut pending_requests = JoinSet::new();

    if plan.old_members.contains(&plan.local_node_id) {
        let share = plan
            .local_share
            .clone()
            .ok_or_else(|| format!("local share for epoch {} is missing", plan.old_epoch))?;
        collected.insert(plan.local_node_id, share);
    }
    spawn_peer_share_requests(&plan, &mut pending_requests);

    let share_deadline = tokio::time::sleep(plan.timeout);
    tokio::pin!(share_deadline);
    while collected.len() < target_share_count {
        if pending_requests.is_empty() {
            (&mut share_deadline).await;
            break;
        }

        tokio::select! {
            _ = &mut share_deadline => break,
            joined = pending_requests.join_next() => {
                let Some(joined) = joined else {
                    continue;
                };
                let (node_id, response) = joined.map_err(|e| format!("trust share collection task failed: {e}"))?;
                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        // If a peer told us we've been expunged, propagate immediately
                        if let Some(expunged) = error.downcast_ref::<ExpungedByPeer>() {
                            return Err(format!("node expunged by peer at epoch {}", expunged.epoch));
                        }
                        warn!(node_id, error = %error, "failed to collect trust share");
                        continue;
                    }
                };
                let Some(share) = validate_share_response(node_id, plan.old_epoch, response, &plan.expected_digests) else {
                    continue;
                };
                collected.insert(node_id, share);
            }
        }
    }

    Ok(collected)
}

fn build_peer_expungement_probe_plan(
    storage: &crate::storage_shared::SharedRedbStorage,
    local_node_id: u64,
) -> Result<Option<PeerExpungementProbePlan>, String> {
    let current_epoch = match storage.load_current_trust_epoch().map_err(|e| e.to_string())? {
        Some(epoch) => epoch,
        None => {
            if storage.load_digests(1).map_err(|e| e.to_string())?.is_empty() {
                return Ok(None);
            }
            1
        }
    };

    let mut peers = storage.load_members(current_epoch).map_err(|e| e.to_string())?;
    peers.remove(&local_node_id);
    if peers.is_empty() {
        return Ok(None);
    }

    let expected_digests = storage.load_digests(current_epoch).map_err(|e| e.to_string())?;
    let threshold_override = storage.load_trust_threshold_override().map_err(|e| e.to_string())?;
    let threshold = resolve_trust_threshold(peers.len().saturating_add(1), threshold_override)?.value();
    validate_threshold_invariant(peers.len().saturating_add(1), threshold, "probe")?;

    Ok(Some(PeerExpungementProbePlan {
        current_epoch,
        required_peer_confirmations: required_peer_confirmations(peers.len(), threshold),
        peers,
        expected_digests,
    }))
}

fn build_trust_rotation_context(
    node: &RaftNode,
    storage: &crate::storage_shared::SharedRedbStorage,
    old_members: &BTreeSet<u64>,
    new_members: &BTreeSet<u64>,
    membership_epoch: u64,
) -> Result<TrustRotationContext, String> {
    let client = node
        .trust_share_client()
        .cloned()
        .ok_or_else(|| "trust share client is not configured".to_string())?;
    let cluster_id = node.trust_cluster_id().ok_or_else(|| "trust cluster id is not configured".to_string())?.to_vec();
    let old_epoch = storage.load_current_trust_epoch().map_err(|e| e.to_string())?.unwrap_or(1);
    let local_share = if old_members.contains(&node.node_id().0) {
        let Some(share) = storage.load_share(old_epoch).map_err(|e| e.to_string())? else {
            return Err(format!("local share for epoch {old_epoch} is missing"));
        };
        Some(share)
    } else {
        None
    };
    let expected_digests = storage.load_digests(old_epoch).map_err(|e| e.to_string())?;
    let threshold_override = storage.load_trust_threshold_override().map_err(|e| e.to_string())?;
    let old_threshold = resolve_trust_threshold(old_members.len(), threshold_override)?.value();
    let new_threshold = resolve_trust_threshold(new_members.len(), threshold_override)?.value();
    validate_threshold_invariant(old_members.len(), old_threshold, "old")?;
    validate_threshold_invariant(new_members.len(), new_threshold, "new")?;
    let old_member_addresses = storage.load_members(old_epoch).map_err(|e| e.to_string())?;
    let timeout_context = TimeoutContext {
        old_epoch,
        membership_epoch,
        old_threshold,
        new_threshold,
        old_members: old_members.clone(),
        new_members: new_members.clone(),
        expected_digests: expected_digests.clone(),
        cluster_id: cluster_id.clone(),
    };

    Ok(TrustRotationContext {
        client,
        cluster_id,
        old_epoch,
        local_share,
        expected_digests,
        threshold_override,
        old_threshold,
        new_threshold,
        old_member_addresses,
        timeout_context,
    })
}

/// Build a `ShareCollectionPlan` from a trust rotation context.
fn build_share_collection_plan(
    context: &TrustRotationContext,
    local_node_id: u64,
    old_members: BTreeSet<u64>,
) -> ShareCollectionPlan {
    ShareCollectionPlan {
        client: context.client.clone(),
        old_epoch: context.old_epoch,
        old_threshold: context.old_threshold,
        old_members,
        old_member_addresses: context.old_member_addresses.clone(),
        local_node_id,
        local_share: context.local_share.clone(),
        expected_digests: context.expected_digests.clone(),
        timeout: TRUST_SHARE_COLLECTION_TIMEOUT,
    }
}

/// Resolve endpoint addresses for each new cluster member from Raft membership.
fn collect_new_member_addresses(
    membership: &openraft::Membership<crate::types::AppTypeConfig>,
    new_members: &BTreeSet<u64>,
) -> Result<Vec<(u64, NodeAddress)>, String> {
    new_members
        .iter()
        .map(|node_id| {
            let node_addr = membership
                .get_node(&(*node_id).into())
                .map(|node| node.node_addr.clone())
                .ok_or_else(|| format!("missing node address for new member {node_id}"))?;
            Ok((*node_id, node_addr))
        })
        .collect()
}

fn removed_nodes(old_members: &BTreeSet<u64>, new_members: &BTreeSet<u64>) -> BTreeSet<u64> {
    old_members.difference(new_members).copied().collect()
}

fn spawn_expungement_notification(
    client: Arc<dyn TrustShareClient>,
    endpoint: iroh::EndpointAddr,
    epoch: u64,
    node_id: u64,
) {
    tokio::spawn(async move {
        info!(node_id, epoch, "sending expungement notification to removed node");
        if let Err(error) = client.send_expunged(endpoint, epoch).await {
            warn!(node_id, epoch, error = %error, "failed to send expungement notification (peer enforcement will handle)");
        }
    });
}

fn notify_removed_nodes(
    client: Arc<dyn TrustShareClient>,
    old_member_addresses: &BTreeMap<u64, NodeAddress>,
    removed_nodes: &BTreeSet<u64>,
    new_epoch: u64,
    leader_id: u64,
) {
    for removed_id in removed_nodes {
        if let Some(node_addr) = old_member_addresses.get(removed_id) {
            match node_endpoint_addr(node_addr) {
                Ok(endpoint) => {
                    spawn_expungement_notification(client.clone(), endpoint, new_epoch, *removed_id);
                }
                Err(error) => {
                    warn!(node_id = *removed_id, error = %error, "invalid stored node address for removed node");
                }
            }
        } else {
            warn!(
                node_id = *removed_id,
                "no stored address for removed node; relying on peer enforcement for expungement"
            );
        }
    }
    info!(
        removed_count = removed_nodes.len(),
        leader_id, new_epoch, "expungement notifications sent to removed nodes"
    );
}

impl RaftNode {
    pub(crate) fn pending_trust_reconfiguration(
        &self,
        metrics: &openraft::RaftMetrics<crate::types::AppTypeConfig>,
    ) -> Result<Option<PendingTrustReconfiguration>, String> {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return Ok(None);
        };

        let old_epoch = storage.load_current_trust_epoch().map_err(|e| e.to_string())?.unwrap_or(1);
        let old_members: BTreeSet<u64> =
            storage.load_digests(old_epoch).map_err(|e| e.to_string())?.into_keys().collect();
        if old_members.is_empty() {
            return Ok(None);
        }

        let Some(membership_log_id) = metrics.membership_config.log_id().as_ref() else {
            return Ok(None);
        };
        let membership_epoch = membership_log_id.index();
        if membership_epoch <= old_epoch {
            return Ok(None);
        }

        let new_members: BTreeSet<u64> = metrics.membership_config.membership().voter_ids().map(|id| id.0).collect();
        if new_members.is_empty() || new_members == old_members {
            return Ok(None);
        }

        Ok(Some((old_members, new_members, membership_epoch)))
    }

    fn is_trust_share_requester_authorized(&self, requester: EndpointId) -> bool {
        let metrics = self.raft().metrics().borrow().clone();
        let membership = metrics.membership_config.membership();
        let voters: BTreeSet<_> = membership.voter_ids().collect();
        let requester_id = requester.to_string();
        membership
            .nodes()
            .any(|(node_id, node)| voters.contains(node_id) && node.endpoint_id() == requester_id)
    }

    pub(crate) async fn probe_for_peer_expungement(&self) -> Result<PeerExpungementProbeOutcome, String> {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return Ok(PeerExpungementProbeOutcome::NotApplicable);
        };
        if storage.is_expunged().map_err(|e| e.to_string())? {
            let epoch = storage.load_expunged().map_err(|e| e.to_string())?.map(|metadata| metadata.epoch).unwrap_or(0);
            return Ok(PeerExpungementProbeOutcome::Expunged { epoch });
        }

        let Some(client) = self.trust_share_client().cloned() else {
            return Ok(PeerExpungementProbeOutcome::NotApplicable);
        };
        let Some(plan) = build_peer_expungement_probe_plan(storage.as_ref(), self.node_id().0)? else {
            return Ok(PeerExpungementProbeOutcome::NotApplicable);
        };

        let mut confirmed_peer_count = 0usize;
        let mut saw_retryable_error = false;
        for (node_id, node_addr) in plan.peers {
            let endpoint = match node_endpoint_addr(&node_addr) {
                Ok(endpoint) => endpoint,
                Err(error) => {
                    saw_retryable_error = true;
                    warn!(node_id, epoch = plan.current_epoch, error = %error, "peer expungement probe skipped invalid node address");
                    continue;
                }
            };
            match client.get_share(endpoint, plan.current_epoch).await {
                Ok(response) => {
                    if !response_reports_current_epoch(&response, plan.current_epoch) {
                        saw_retryable_error = true;
                        warn!(
                            node_id,
                            requested_epoch = plan.current_epoch,
                            peer_current_epoch = response.current_epoch,
                            "peer expungement probe rejected share from non-current epoch source"
                        );
                        continue;
                    }
                    if validate_share_response(node_id, plan.current_epoch, response, &plan.expected_digests).is_some()
                    {
                        confirmed_peer_count = confirmed_peer_count.saturating_add(1);
                        continue;
                    }
                    saw_retryable_error = true;
                    warn!(
                        node_id,
                        epoch = plan.current_epoch,
                        "peer expungement probe rejected non-authoritative share response"
                    );
                }
                Err(error) => {
                    if let Some(expunged) = error.downcast_ref::<ExpungedByPeer>() {
                        self.handle_peer_expungement(expunged.epoch, node_id)?;
                        return Ok(PeerExpungementProbeOutcome::Expunged { epoch: expunged.epoch });
                    }
                    saw_retryable_error = true;
                    warn!(node_id, epoch = plan.current_epoch, error = %error, "peer expungement probe failed");
                }
            }
        }

        if confirmed_peer_count >= plan.required_peer_confirmations && !saw_retryable_error {
            info!(
                epoch = plan.current_epoch,
                confirmed_peer_count,
                required_peer_confirmations = plan.required_peer_confirmations,
                "peer expungement probe confirmed local membership"
            );
            return Ok(PeerExpungementProbeOutcome::Healthy);
        }
        Ok(PeerExpungementProbeOutcome::RetryNeeded)
    }

    pub(crate) async fn rotate_trust_after_membership_change(
        &self,
        old_members: BTreeSet<u64>,
        new_members: BTreeSet<u64>,
        membership_epoch: u64,
    ) -> Result<(), String> {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return Ok(());
        };
        let context =
            build_trust_rotation_context(self, storage.as_ref(), &old_members, &new_members, membership_epoch)?;

        let plan = build_share_collection_plan(&context, self.node_id().0, old_members.clone());
        let old_shares = collect_old_shares_for_reconfiguration(plan).await?;
        if old_shares.len() < usize::from(context.old_threshold) {
            return Err(build_timeout_error(&context.timeout_context, &old_shares));
        }

        let collected: Vec<_> = old_shares.values().take(usize::from(context.old_threshold)).cloned().collect();
        let current_secret =
            shamir::reconstruct_secret(&collected).map_err(|e| format!("failed to reconstruct current secret: {e}"))?;
        let prior_secrets = if context.old_epoch > 1 {
            let chain = storage
                .load_encrypted_chain(context.old_epoch)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("missing encrypted chain for epoch {}", context.old_epoch))?;
            chain::decrypt_chain(&chain, &current_secret, &context.cluster_id)
                .map_err(|e| format!("failed to decrypt historical secret chain: {e}"))?
        } else {
            BTreeMap::new()
        };

        let mut coordinator = ReconfigCoordinator::new(ReconfigParams {
            old_epoch: context.old_epoch,
            new_epoch: membership_epoch,
            old_threshold: context.old_threshold,
            new_threshold: context.new_threshold,
            old_members: old_members.clone(),
            new_members: new_members.clone(),
            expected_digests: context.expected_digests.clone(),
            cluster_id: context.cluster_id.clone(),
            prior_secrets,
        });
        let mut actions = Vec::new();
        for (node_id, share) in old_shares {
            let new_actions = coordinator.on_share_received(node_id, share).map_err(|e| e.to_string())?;
            actions.extend(new_actions);
        }

        let metrics = self.raft().metrics().borrow().clone();
        let membership = metrics.membership_config.membership();
        let new_member_addresses = collect_new_member_addresses(membership, &new_members)?;
        let payload = proposal_from_actions(actions, context.threshold_override, new_member_addresses)
            .ok_or_else(|| "reconfiguration coordinator produced no proposal".to_string())?;

        let new_epoch = payload.epoch;
        self.raft()
            .client_write(AppRequest::TrustReconfiguration(payload))
            .await
            .map_err(|e| format!("failed to commit trust reconfiguration: {e}"))?;

        let removed = removed_nodes(&old_members, &new_members);
        if !removed.is_empty() {
            notify_removed_nodes(context.client, &context.old_member_addresses, &removed, new_epoch, self.node_id().0);
        }
        Ok(())
    }

    /// Handle notification that this node has been expunged from the cluster.
    ///
    /// Validates that the epoch is >= our latest known config epoch,
    /// then permanently marks the node as expunged and zeroizes all shares.
    pub(crate) fn handle_peer_expungement(&self, epoch: u64, from: u64) -> Result<(), String> {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return Err("trust requires redb storage".to_string());
        };

        // Already expunged — nothing to do
        if storage.is_expunged().map_err(|e| e.to_string())? {
            return Ok(());
        }

        let current_epoch = storage.load_current_trust_epoch().map_err(|e| e.to_string())?.unwrap_or(0);
        if epoch < current_epoch {
            warn!(expunge_epoch = epoch, current_epoch, "ignoring stale expungement notification");
            return Ok(());
        }

        let timestamp_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let metadata = aspen_cluster_types::ExpungedMetadata {
            epoch,
            removed_by: from,
            timestamp_ms,
        };

        storage.mark_expunged(metadata).map_err(|e| e.to_string())?;
        self.expunged_flag.store(true, std::sync::atomic::Ordering::Release);
        error!(
            epoch,
            removed_by = from,
            "THIS NODE HAS BEEN PERMANENTLY EXPUNGED — factory reset required to rejoin"
        );
        Ok(())
    }

    /// Initialize trust (Shamir secret sharing) during cluster bootstrap.
    ///
    /// Called by the leader after successful Raft initialization.
    pub(crate) async fn initialize_trust(&self, request: &InitRequest) -> Result<(), String> {
        match self.state_machine() {
            StateMachineVariant::Redb(_) => {}
            StateMachineVariant::InMemory(_) => {
                return Err("trust requires redb storage (not in-memory)".to_string());
            }
        }

        let payload = build_trust_initialize_payload(request)?;
        let local_node_id: u64 = self.node_id().into();
        let share_x = payload
            .shares
            .iter()
            .find_map(|(node_id, share_bytes)| {
                if *node_id == local_node_id {
                    share_bytes.first().copied()
                } else {
                    None
                }
            })
            .ok_or_else(|| format!("this node {local_node_id} not found in initial_members"))?;

        self.raft()
            .client_write(AppRequest::TrustInitialize(payload.clone()))
            .await
            .map_err(|e| format!("failed to commit trust initialization: {e}"))?;

        info!(
            epoch = payload.epoch,
            node_id = local_node_id,
            share_x,
            digests = payload.digests.len(),
            "trust initialized through committed raft request"
        );

        Ok(())
    }
}

#[async_trait]
impl TrustShareProvider for RaftNode {
    async fn get_share(&self, requester: EndpointId, epoch: u64) -> anyhow::Result<Option<TrustResponse>> {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return Ok(None);
        };

        // Expunged nodes reject all trust protocol messages
        if storage.is_expunged()? {
            warn!(requester = %requester, epoch, "expunged node rejecting trust share request");
            return Ok(None);
        }

        // If requester is not in current membership, notify them they've been expunged
        if !self.is_trust_share_requester_authorized(requester) {
            warn!(requester = %requester, epoch, "trust share request from non-member, sending expungement notification");
            let current_epoch = storage.load_current_trust_epoch()?.unwrap_or(1);
            return Ok(Some(TrustResponse::Expunged { epoch: current_epoch }));
        }

        let Some(share) = storage.load_share(epoch)? else {
            warn!(requester = %requester, epoch, "trust share requested for unknown epoch");
            return Ok(None);
        };

        let current_epoch = storage.load_current_trust_epoch()?.unwrap_or(1);
        Ok(Some(TrustResponse::Share(ShareResponse {
            epoch,
            current_epoch,
            share,
        })))
    }

    fn is_expunged(&self) -> bool {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return false;
        };
        storage.is_expunged().unwrap_or(false)
    }

    async fn on_expunged(&self, from: EndpointId, epoch: u64) -> anyhow::Result<()> {
        let StateMachineVariant::Redb(storage) = self.state_machine() else {
            return Ok(());
        };

        // Only accept expungement if the epoch is >= our latest known trust epoch.
        let current_epoch = storage.load_current_trust_epoch()?.unwrap_or(0);
        if epoch < current_epoch {
            warn!(
                from = %from,
                their_epoch = epoch,
                our_epoch = current_epoch,
                "ignoring stale expungement notification"
            );
            return Ok(());
        }

        let now_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let metadata = aspen_cluster_types::ExpungedMetadata {
            epoch,
            removed_by: 0, // We don't know which node initiated it from a peer notification
            timestamp_ms: now_ms,
        };

        storage.mark_expunged(metadata)?;
        self.expunged_flag.store(true, std::sync::atomic::Ordering::Release);
        error!(
            from = %from,
            epoch,
            "node has been expunged by peer — all trust shares zeroized"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Instant;

    use aspen_cluster_types::ClusterNode;
    use aspen_trust::secret::ClusterSecret;
    use iroh::EndpointAddr;
    use iroh::SecretKey;

    use super::*;

    fn endpoint_addr() -> EndpointAddr {
        let key = SecretKey::generate(&mut rand::rng());
        EndpointAddr::new(key.public())
    }

    fn setup_reconfig_shares() -> (Vec<shamir::Share>, BTreeMap<u64, shamir::ShareDigest>) {
        let secret = ClusterSecret::generate();
        let mut rng = rand::rng();
        let shares = shamir::split_secret(secret.as_bytes(), 2, 3, &mut rng).unwrap();
        let digests = [1_u64, 2, 3]
            .iter()
            .zip(shares.iter())
            .map(|(&node_id, share)| (node_id, shamir::share_digest(share)))
            .collect();
        (shares, digests)
    }

    fn request_with_ids(ids: &[u64], threshold: Option<u8>) -> InitRequest {
        InitRequest {
            initial_members: ids.iter().map(|id| ClusterNode::with_iroh_addr(*id, endpoint_addr())).collect(),
            trust: match threshold {
                Some(value) => aspen_cluster_types::TrustConfig::with_threshold(value),
                None => aspen_cluster_types::TrustConfig::enabled(),
            },
        }
    }

    #[test]
    fn test_build_trust_initialize_payload_assigns_every_member() {
        let request = request_with_ids(&[1, 2, 3], None);
        let payload = build_trust_initialize_payload(&request).unwrap();

        assert_eq!(payload.epoch, 1);
        assert_eq!(payload.threshold_override, None);
        assert_eq!(payload.shares.len(), 3);
        assert_eq!(payload.digests.len(), 3);
        assert_eq!(payload.members.len(), 3);

        let decoded = shares_from_payload(&payload.shares).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(shamir::reconstruct_secret(&decoded[..2]).unwrap().len(), shamir::SECRET_SIZE);

        for ((node_id, share_bytes), (_, digest)) in payload.shares.iter().zip(payload.digests.iter()) {
            let mut share_array = [0u8; shamir::SECRET_SIZE + 1];
            share_array.copy_from_slice(share_bytes);
            let share = shamir::Share::from_bytes(&share_array).unwrap();
            assert_eq!(shamir::share_digest(&share), *digest);
            assert!([1u64, 2, 3].contains(node_id));
        }
    }

    #[test]
    fn test_required_peer_confirmations_caps_at_peer_count() {
        assert_eq!(required_peer_confirmations(1, 3), 1);
        assert_eq!(required_peer_confirmations(2, 2), 2);
        assert_eq!(required_peer_confirmations(5, 2), 2);
    }

    #[test]
    fn test_share_collection_target_rejects_threshold_above_membership() {
        let plan = ShareCollectionPlan {
            client: Arc::new(MockTrustShareClient {
                responses: Mutex::new(BTreeMap::new()),
            }),
            old_epoch: 1,
            old_threshold: 3,
            old_members: [1u64, 2].into(),
            old_member_addresses: BTreeMap::new(),
            local_node_id: 1,
            local_share: None,
            expected_digests: BTreeMap::new(),
            timeout: Duration::from_millis(1),
        };

        let error = share_collection_target(&plan).unwrap_err();
        assert!(error.contains("old threshold 3 exceeds cluster size 2"));
    }

    #[test]
    fn test_build_trust_initialize_payload_rejects_invalid_threshold() {
        let request = request_with_ids(&[1, 2, 3], Some(4));
        let err = build_trust_initialize_payload(&request).unwrap_err();
        assert!(err.contains("exceeds cluster size 3"));
    }

    #[test]
    fn test_timeout_error_reports_partial_share_collection() {
        let (shares, digests) = setup_reconfig_shares();
        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let new_members: BTreeSet<u64> = [1, 2, 4].into();
        let old_threshold = resolve_trust_threshold(old_members.len(), None).unwrap().value();
        let new_threshold = resolve_trust_threshold(new_members.len(), None).unwrap().value();

        let mut collected = BTreeMap::new();
        collected.insert(1, shares[0].clone());

        let error = build_timeout_error(
            &TimeoutContext {
                old_epoch: 1,
                membership_epoch: 2,
                old_threshold,
                new_threshold,
                old_members: old_members.clone(),
                new_members: new_members.clone(),
                expected_digests: digests.clone(),
                cluster_id: b"cluster".to_vec(),
            },
            &collected,
        );

        assert!(error.contains("share collection timed out: got 1 of 2 shares"));
    }

    struct MockTrustShareClient {
        responses: Mutex<BTreeMap<String, MockResponse>>,
    }

    enum MockResponse {
        Share(ShareResponse),
        Delay(Duration),
    }

    #[async_trait]
    impl TrustShareClient for MockTrustShareClient {
        async fn send_expunged(&self, _target: EndpointAddr, _epoch: u64) -> anyhow::Result<()> {
            Ok(())
        }

        async fn get_share(&self, target: EndpointAddr, _epoch: u64) -> anyhow::Result<ShareResponse> {
            let response = self
                .responses
                .lock()
                .unwrap()
                .remove(&target.id.to_string())
                .unwrap_or(MockResponse::Delay(Duration::from_millis(50)));
            match response {
                MockResponse::Share(response) => Ok(response),
                MockResponse::Delay(delay) => {
                    tokio::time::sleep(delay).await;
                    anyhow::bail!("delayed past timeout")
                }
            }
        }
    }

    #[tokio::test]
    async fn test_collect_old_shares_honors_custom_threshold_override() {
        let (shares, digests) = setup_reconfig_shares();
        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let collected = collect_old_shares_for_reconfiguration(ShareCollectionPlan {
            client: Arc::new(MockTrustShareClient {
                responses: Mutex::new(BTreeMap::new()),
            }),
            old_epoch: 1,
            old_threshold: resolve_trust_threshold(old_members.len(), Some(1)).unwrap().value(),
            old_members: old_members.clone(),
            old_member_addresses: BTreeMap::new(),
            local_node_id: 1,
            local_share: Some(shares[0].clone()),
            expected_digests: digests.clone(),
            timeout: Duration::from_millis(5),
        })
        .await
        .unwrap();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected.get(&1), Some(&shares[0]));
    }

    #[tokio::test]
    async fn test_collect_old_shares_accepts_valid_remote_share_from_stored_address() {
        let (shares, digests) = setup_reconfig_shares();
        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let remote_addr = endpoint_addr();
        let mut addresses = BTreeMap::new();
        addresses.insert(2, NodeAddress::new(remote_addr.clone()));

        let collected = collect_old_shares_for_reconfiguration(ShareCollectionPlan {
            client: Arc::new(MockTrustShareClient {
                responses: Mutex::new(BTreeMap::from([(
                    remote_addr.id.to_string(),
                    MockResponse::Share(ShareResponse {
                        epoch: 1,
                        current_epoch: 1,
                        share: shares[1].clone(),
                    }),
                )])),
            }),
            old_epoch: 1,
            old_threshold: 2,
            old_members: old_members.clone(),
            old_member_addresses: addresses,
            local_node_id: 1,
            local_share: Some(shares[0].clone()),
            expected_digests: digests.clone(),
            timeout: Duration::from_millis(20),
        })
        .await
        .unwrap();

        assert_eq!(collected.len(), 2);
        assert_eq!(collected.get(&2), Some(&shares[1]));
    }

    #[tokio::test]
    async fn test_collect_old_shares_waits_for_timeout_when_no_requests_exist() {
        let (_, digests) = setup_reconfig_shares();
        let old_members: BTreeSet<u64> = [1, 2, 3].into();
        let start = Instant::now();
        let collected = collect_old_shares_for_reconfiguration(ShareCollectionPlan {
            client: Arc::new(MockTrustShareClient {
                responses: Mutex::new(BTreeMap::new()),
            }),
            old_epoch: 1,
            old_threshold: 2,
            old_members: old_members.clone(),
            old_member_addresses: BTreeMap::new(),
            local_node_id: 99,
            local_share: None,
            expected_digests: digests.clone(),
            timeout: Duration::from_millis(25),
        })
        .await
        .unwrap();

        assert!(start.elapsed() >= Duration::from_millis(20));
        assert!(collected.is_empty());
    }

    #[test]
    fn test_validate_share_response_rejects_digest_mismatch_without_aborting() {
        let (shares, digests) = setup_reconfig_shares();
        let valid = ShareResponse {
            epoch: 1,
            current_epoch: 1,
            share: shares[0].clone(),
        };
        let invalid = ShareResponse {
            epoch: 1,
            current_epoch: 1,
            share: shares[1].clone(),
        };

        let accepted = validate_share_response(1, 1, valid, &digests);
        let rejected = validate_share_response(1, 1, invalid, &digests);

        assert!(accepted.is_some());
        assert!(rejected.is_none());
    }
}
