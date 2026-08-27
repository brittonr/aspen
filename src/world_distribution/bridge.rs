use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use molten_core::content_replication::Action;
use molten_core::content_replication::ActionKind;
use molten_core::content_replication::Plan;
use molten_core::dag_sync::DagFetchRequest;
use molten_core::dag_sync::DagResponseObservation;
use molten_core::dag_sync::DagSyncPlan;
use molten_core::world_distribution::WorldReplicationPlan;

use crate::content_replication::ContentPort;
use crate::content_replication::TransferEnvelope;
use crate::content_replication::TransferOutcome;
use crate::content_replication::TransportPort;
use crate::dag_sync::DagContentVerificationPort;
use crate::dag_sync::DagTransferOutcome;
use crate::dag_sync::DagTransportEnvelope;
use crate::dag_sync::DagTransportPort;
use crate::error::MoltenError;
use crate::error::Result;

#[derive(Debug, Default)]
struct BridgeState {
    actions: BTreeMap<String, Vec<Action>>,
    envelopes: BTreeMap<String, TransferEnvelope>,
}

#[derive(Debug, Clone)]
pub struct WorldReplicationBridge {
    state: Rc<RefCell<BridgeState>>,
}

impl WorldReplicationBridge {
    pub fn new(plan: &WorldReplicationPlan) -> Result<Self> {
        let mut actions = BTreeMap::<String, Vec<Action>>::new();
        for action in transferable_actions(&plan.shared_plan) {
            actions.entry(action.content_ref.clone()).or_default().push(action.clone());
        }
        for candidates in actions.values_mut() {
            candidates.sort();
        }
        if actions.len() != plan.manifest.contents.len() {
            return Err(MoltenError::invalid_harness(
                "world replication bridge requires one transferable action set for every closure object",
            ));
        }
        Ok(Self {
            state: Rc::new(RefCell::new(BridgeState {
                actions,
                envelopes: BTreeMap::new(),
            })),
        })
    }

    pub fn transport<'a, T>(&self, inner: &'a mut T) -> WorldReplicationTransport<'a, T>
    where T: TransportPort {
        WorldReplicationTransport {
            state: Rc::clone(&self.state),
            inner,
        }
    }

    pub fn verification<'a, C>(&self, inner: &'a mut C) -> WorldReplicationVerification<'a, C>
    where C: ContentPort {
        WorldReplicationVerification {
            state: Rc::clone(&self.state),
            inner,
        }
    }
}

pub struct WorldReplicationTransport<'a, T>
where T: TransportPort
{
    state: Rc<RefCell<BridgeState>>,
    inner: &'a mut T,
}

impl<T> DagTransportPort for WorldReplicationTransport<'_, T>
where T: TransportPort
{
    fn request(&mut self, request: &DagFetchRequest) -> Result<DagTransferOutcome> {
        let action = select_action(&self.state.borrow(), request)?.clone();
        match self.inner.fetch(&action)? {
            TransferOutcome::Received(envelope) => {
                validate_transfer_envelope(&action, &envelope)?;
                self.state.borrow_mut().envelopes.insert(request.object_ref.as_str().to_string(), envelope.clone());
                Ok(DagTransferOutcome::Received(DagTransportEnvelope {
                    object_ref: request.object_ref.clone(),
                    assigned_peer: request.assigned_peer.clone(),
                    encoded_bytes: envelope.encoded_bytes,
                    transport_observation_ref: envelope.transfer_ref,
                }))
            }
            TransferOutcome::Cancelled(observation_ref) => Ok(DagTransferOutcome::Cancelled(observation_ref)),
            TransferOutcome::Uncertain(observation_ref)
            | TransferOutcome::Unavailable(observation_ref)
            | TransferOutcome::TimedOut(observation_ref) => Ok(DagTransferOutcome::Deferred(observation_ref)),
        }
    }
}

pub struct WorldReplicationVerification<'a, C>
where C: ContentPort
{
    state: Rc<RefCell<BridgeState>>,
    inner: &'a mut C,
}

impl<C> DagContentVerificationPort for WorldReplicationVerification<'_, C>
where C: ContentPort
{
    fn verify(
        &mut self,
        plan: &DagSyncPlan,
        envelope: &DagTransportEnvelope,
        _authority_ref: &str,
    ) -> Result<DagResponseObservation> {
        let transfer = self.state.borrow_mut().envelopes.remove(envelope.object_ref.as_str()).ok_or_else(|| {
            MoltenError::invalid_harness("world replication bridge has no matching transfer envelope")
        })?;
        let action = select_action_for_transfer(&self.state.borrow(), &transfer)?.clone();
        let verification = self.inner.verify(&action, &transfer)?;
        if verification.operation_id != action.operation_id
            || verification.replica.content_ref != action.content_ref
            || verification.replica.peer_id != action.target_peer
        {
            return Err(MoltenError::invalid_harness(
                "world replication verification observation drifted from the transfer action",
            ));
        }
        Ok(DagResponseObservation {
            epoch_ref: plan.epoch_ref.clone(),
            generation: plan.generation,
            object_ref: envelope.object_ref.clone(),
            assigned_peer: envelope.assigned_peer.clone(),
            identity_verified: verification.identity_verified,
            authorization_admitted: verification.authorization_admitted,
            encoded_bytes: transfer.encoded_bytes,
        })
    }
}

fn transferable_actions(plan: &Plan) -> impl Iterator<Item = &Action> {
    plan.actions.iter().filter(|action| {
        matches!(action.kind, ActionKind::Transfer | ActionKind::Repair | ActionKind::Handoff | ActionKind::Reuse)
    })
}

fn select_action<'a>(state: &'a BridgeState, request: &DagFetchRequest) -> Result<&'a Action> {
    let candidates = state
        .actions
        .get(request.object_ref.as_str())
        .ok_or_else(|| MoltenError::invalid_harness("DAG requested an object outside the world replication plan"))?;
    candidates
        .iter()
        .find(|action| request.assigned_peer.as_ref().is_none_or(|peer| peer.as_str() == action.target_peer))
        .ok_or_else(|| MoltenError::invalid_harness("DAG peer assignment has no matching world replication action"))
}

fn select_action_for_transfer<'a>(state: &'a BridgeState, transfer: &TransferEnvelope) -> Result<&'a Action> {
    state
        .actions
        .get(transfer.content_ref.as_str())
        .and_then(|candidates| {
            candidates.iter().find(|action| {
                action.operation_id == transfer.operation_id && action.target_peer == transfer.target_peer
            })
        })
        .ok_or_else(|| MoltenError::invalid_harness("transfer envelope has no matching world replication action"))
}

fn validate_transfer_envelope(action: &Action, envelope: &TransferEnvelope) -> Result<()> {
    if envelope.operation_id != action.operation_id
        || envelope.content_ref != action.content_ref
        || envelope.target_peer != action.target_peer
        || envelope.encoded_bytes != action.encoded_bytes
        || envelope.protected != action.preserve_protected_form
    {
        return Err(MoltenError::invalid_harness(
            "content replication returned a substituted or drifted world object envelope",
        ));
    }
    crate::preserves_rail::validate_content_ref(&envelope.transfer_ref)
        .map_err(|_| MoltenError::invalid_harness("world transfer observation ref is invalid"))?;
    Ok(())
}
