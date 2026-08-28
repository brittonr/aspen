use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use molten_core::world_commit::WorldCommitRef;
use molten_core::world_head::*;
use molten_core::world_promotion::*;
use molten_node_host::node_state::NodeStateNamespace;
use molten_node_host::node_state::NodeStateNamespaceKind;
use molten_node_host::node_state::NodeStateRoot;

use super::super::*;
use crate::error::Result;
use crate::world_head::WorldHeadFreshAdmission;
use crate::world_head::WorldHeadMutationOutcome;
use crate::world_head::WorldHeadStatePort;
use crate::world_head::WorldHeadTransitionReceiptInput;
use crate::world_head::canonical_world_head_transition_receipt;

pub(super) const CURRENT_GENERATION: u64 = 1;
pub(super) const EXPECTED_RESERVATIONS: usize = 1;

pub(super) struct TestState {
    pub _temporary: cap_tempfile::TempDir,
    pub storage: NodeStateNamespace,
    pub store: LocalWorldPromotionStore,
}

pub(super) fn test_state(request: &WorldPromotionRequest) -> TestState {
    let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
    let root = NodeStateRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
    root.create_layout().expect("state layout");
    let storage = root.namespace(NodeStateNamespaceKind::Storage).expect("storage namespace");
    let mut store = LocalWorldPromotionStore::open(&storage).expect("promotion store");
    seed_head(&mut store, request);
    TestState {
        _temporary: temporary,
        storage,
        store,
    }
}

pub(super) fn promotion_request() -> WorldPromotionRequest {
    let policy_ref = WorldHeadPolicyRef::new(reference("promotion-policy")).expect("policy ref");
    WorldPromotionRequest {
        operation_ref: WorldPromotionOperationRef::new(reference("promotion-operation")).expect("operation ref"),
        branch_id: WorldBranchId::new("release").expect("branch"),
        branch_class: WorldBranchClass::Candidate,
        expected_head: commit("active"),
        candidate_head: commit("candidate"),
        expected_generation: CURRENT_GENERATION,
        policy_ref: policy_ref.clone(),
        authority: WorldPromotionAuthorityObservation {
            authority_ref: WorldPromotionAuthorityRef::new(reference("promotion-authority")).expect("authority ref"),
            policy_ref,
            observed_generation: CURRENT_GENERATION,
            admitted: true,
        },
        intent_closure_complete: true,
        simulation_only: false,
        intents: vec![
            intent("release", WorldIntentReleaseClass::Release),
            intent("retain", WorldIntentReleaseClass::Retain),
        ],
        bounds: WorldPromotionBounds::standard(),
    }
}

pub(super) fn dispatch_facts() -> WorldDispatchFacts {
    WorldDispatchFacts {
        observed_generation: CURRENT_GENERATION + 1,
        authority_admitted: true,
        policy_admitted: true,
        capability_admitted: true,
        handler_matches: true,
        adapter_matches: true,
    }
}

pub(super) struct Current {
    pub is_admitted: bool,
}

impl WorldPromotionCurrentPort for Current {
    fn observe_transaction(&mut self, plan: &WorldPromotionPlan) -> Result<WorldPromotionTransactionFacts> {
        Ok(WorldPromotionTransactionFacts {
            observed_head: Some(plan.before.clone()),
            authority_ref: plan.authority_ref.clone(),
            authority_admitted: self.is_admitted,
            authority_generation: plan.before.generation,
            policy_ref: plan.after.policy_ref.clone(),
            intent_closure_complete: true,
            reservation_refs: plan.reservations.iter().map(|reservation| reservation.reservation_ref.clone()).collect(),
        })
    }
}

pub(super) struct Admission {
    pub facts: WorldDispatchFacts,
}

impl WorldEffectAdmissionPort for Admission {
    fn observe_dispatch(
        &mut self,
        _plan: &WorldPromotionPlan,
        _reservation: &WorldReleaseReservation,
    ) -> Result<WorldDispatchFacts> {
        Ok(self.facts.clone())
    }
}

pub(super) struct Dispatcher {
    pub calls: usize,
    pub observation: WorldAttemptObservation,
}

impl WorldEffectDispatcherPort for Dispatcher {
    fn dispatch(&mut self, _plan: &WorldDispatchPlan) -> Result<WorldAttemptObservation> {
        self.calls = self.calls.saturating_add(1);
        Ok(self.observation.clone())
    }
}

pub(super) struct Receipts {
    pub events: Rc<RefCell<Vec<&'static str>>>,
    pub count: usize,
}

impl WorldPromotionReceiptPort for Receipts {
    fn publish_promotion_receipt(&mut self, _receipt: &CanonicalWorldPromotionRecord) -> Result<()> {
        self.events.borrow_mut().push("receipt");
        self.count = self.count.saturating_add(1);
        Ok(())
    }
}

pub(super) fn receipts() -> Receipts {
    Receipts {
        events: Rc::new(RefCell::new(Vec::new())),
        count: 0,
    }
}

pub(super) struct UnknownTransaction {
    pub readback_calls: Cell<usize>,
}

impl WorldPromotionTransactionPort for UnknownTransaction {
    fn commit_promotion(
        &mut self,
        _plan: &WorldPromotionPlan,
        _canonical_plan: &CanonicalWorldPromotionRecord,
        _reservations: &[CanonicalWorldPromotionRecord],
        _facts: &WorldPromotionTransactionFacts,
    ) -> Result<WorldPromotionCommitObservation> {
        Ok(WorldPromotionCommitObservation::OutcomeUnknown)
    }

    fn read_back_promotion(&self, _plan: &WorldPromotionPlan) -> Result<WorldPromotionReadBackObservation> {
        self.readback_calls.set(self.readback_calls.get().saturating_add(1));
        Ok(WorldPromotionReadBackObservation::Reservation)
    }

    fn read_reservation(
        &self,
        _reservation_ref: &WorldReleaseReservationRef,
    ) -> Result<Option<WorldReleaseReservation>> {
        Ok(None)
    }

    fn list_reservations(&self) -> Result<Vec<WorldReleaseReservation>> {
        Ok(Vec::new())
    }

    fn claim_reservation(
        &mut self,
        _reservation_ref: &WorldReleaseReservationRef,
    ) -> Result<Option<WorldReleaseReservation>> {
        Ok(None)
    }

    fn update_reservation(&mut self, _reservation: &WorldReleaseReservation) -> Result<()> {
        Ok(())
    }

    fn store_attempt(&mut self, _attempt: &WorldAttemptRecord) -> Result<()> {
        Ok(())
    }

    fn read_attempt(&self, _attempt_ref: &WorldReleaseAttemptRef) -> Result<Option<WorldAttemptRecord>> {
        Ok(None)
    }
}

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

fn seed_head(store: &mut LocalWorldPromotionStore, request: &WorldPromotionRequest) {
    let state = WorldHeadState {
        branch_id: request.branch_id.clone(),
        branch_class: request.branch_class,
        head: request.expected_head.clone(),
        generation: request.expected_generation,
        policy_ref: request.policy_ref.clone(),
    };
    let claim_ref = WorldHeadClaimRef::new(reference("seed-claim")).expect("claim ref");
    let plan = WorldHeadTransitionPlan {
        claim_ref: claim_ref.clone(),
        before: None,
        after: state,
        choregraph_before_identity: None,
        choregraph_after_identity: reference("seed-choregraph"),
        currentness: WorldHeadCurrentnessClass::RelativeToObservedStore,
    };
    let statement_ref = WorldHeadStatementRef::new(reference("seed-statement")).expect("statement ref");
    let receipt = canonical_world_head_transition_receipt(&WorldHeadTransitionReceiptInput {
        decision: "admitted",
        plan: Some(&plan),
        claim_ref: &claim_ref,
        statement_ref: &statement_ref,
        authentication_decision_ref: &reference("seed-authentication"),
        authority_ref: &reference("seed-authority"),
        issue_codes: &[],
    })
    .expect("seed receipt");
    let outcome = store
        .head_store_mut()
        .apply_transition(&plan, &receipt, |_observed| {
            Ok(WorldHeadFreshAdmission {
                authentication_passed: true,
                authority: WorldHeadAuthorityObservation {
                    authority_ref: WorldHeadAuthorityRef::new(reference("seed-authority")).expect("authority ref"),
                    policy_ref: request.policy_ref.clone(),
                    admitted: true,
                    observed_generation: 0,
                },
            })
        })
        .expect("seed head");
    assert_eq!(outcome, WorldHeadMutationOutcome::Applied);
}

fn intent(label: &str, class: WorldIntentReleaseClass) -> WorldEffectIntent {
    WorldEffectIntent {
        intent_ref: WorldEffectIntentRef::new(reference(&format!("intent:{label}"))).expect("intent ref"),
        semantic_ref: WorldSemanticIntentRef::new(reference(&format!("semantic:{label}"))).expect("semantic ref"),
        handler_ref: WorldPromotionHandlerRef::new(reference(&format!("handler:{label}"))).expect("handler ref"),
        adapter_ref: WorldPromotionAdapterRef::new(reference(&format!("adapter:{label}"))).expect("adapter ref"),
        release_class: Some(class),
    }
}

fn commit(label: &str) -> WorldCommitRef {
    WorldCommitRef::new(reference(label)).expect("commit ref")
}
