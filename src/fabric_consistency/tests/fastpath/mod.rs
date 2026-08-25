#![allow(
    tigerstyle::non_trait_imports,
    reason = "test helpers keep deterministic collection owners local to the fast-path fixture module"
)]
#![allow(
    tigerstyle::path_segment_repetition,
    reason = "fast-path repro test names retain the bounded model artifact term for direct review"
)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(super) use crate::fabric_consistency::fastpath::*;

const INITIAL_VIEW: u64 = 1;
const NEXT_VIEW: u64 = 2;
const SESSION_SEQUENCE: u64 = 7;
const EXTENSION_GENERATION: u64 = 3;
const ENGINE_EPOCH: u64 = 4;
const THREE_NODES: usize = 3;
const FIVE_NODES: usize = 5;
const THREE_NODE_SUPERQUORUM: usize = 3;
const FIVE_NODE_SUPERQUORUM: usize = 4;
const MODEL_COMMAND_BOUND: usize = 8;
const MODEL_KEY_BOUND: usize = 4;
const MODEL_VIEW_BOUND: usize = 4;
const MODEL_STEP_BOUND: usize = 64;
const FIRST_DIVERGENCE: usize = 3;
const PARTIAL_SCENARIO_BOUND: usize = 2;
const DUPLICATE_COUNT: usize = 2;
const LATER_STEP_SEQUENCE: usize = 4;
const EXPLORED_TRANSITIONS: usize = 3;
const ELIGIBLE_TRANSITIONS: usize = 5;

fn strings(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn profile(node_count: usize) -> FastPathModelProfile {
    FastPathModelProfile {
        profile_id: if node_count == THREE_NODES {
            THREE_REPLICA_PROFILE.to_owned()
        } else {
            FIVE_REPLICA_PROFILE.to_owned()
        },
        source: SourceCohort {
            paper: JETPACK_PAPER.to_owned(),
            artifact_source: JETPACK_ARTIFACT_SOURCE.to_owned(),
            artifact_revision: JETPACK_ARTIFACT_REVISION.to_owned(),
            artifact_license: "MIT".to_owned(),
        },
        base_model_ref: "blake3:base-model".to_owned(),
        conflict_contract_ref: "blake3:conflict-contract".to_owned(),
        fault_model: CRASH_FAULT_MODEL.to_owned(),
        node_count,
        active_proposers: strings(&["node-a"]),
        max_commands: MODEL_COMMAND_BOUND,
        max_keys: MODEL_KEY_BOUND,
        max_views: MODEL_VIEW_BOUND,
        max_steps: MODEL_STEP_BOUND,
        base_ordering: BaseOrderingPrerequisites {
            receive_order_preserved: true,
            proposal_order_preserved: true,
            execution_order_preserved: true,
            acknowledgement_waits_for_proposal_evidence: false,
        },
        selection: SelectionMode::PureModel,
        claim_profile: MODEL_ONLY_CLAIM.to_owned(),
        invariant_names: strings(required_invariants()),
        non_claims: strings(required_non_claims()),
    }
}

fn operation(command: &str) -> OperationIdentity {
    OperationIdentity {
        command_ref: command.to_owned(),
        session_ref: "session-a".to_owned(),
        session_sequence: SESSION_SEQUENCE,
        group_ref: "group-a".to_owned(),
        extension_generation: EXTENSION_GENERATION,
        application_schema_ref: "schema-a".to_owned(),
        policy_ref: "policy-a".to_owned(),
        authority_ref: "authority-a".to_owned(),
        resource_ref: "resource-a".to_owned(),
        engine_epoch: ENGINE_EPOCH,
    }
}

fn attempt(node_count: usize) -> StableViewAttempt {
    let operation = operation("command-a");
    let acknowledgements = (0..node_count)
        .map(|index| FastAcknowledgement {
            replica_id: format!("node-{index}"),
            acceleration_view: INITIAL_VIEW,
            base_view: INITIAL_VIEW,
            operation: operation.clone(),
        })
        .collect();
    StableViewAttempt {
        operation: operation.clone(),
        original_operation: operation,
        acceleration_view: INITIAL_VIEW,
        base_view: INITIAL_VIEW,
        conflict_free: true,
        acknowledgements,
        promises: vec![ProposerPromise {
            proposer_id: "node-a".to_owned(),
            acceleration_view: INITIAL_VIEW,
            base_view: INITIAL_VIEW,
            proposal_order_preserved: true,
        }],
        active_proposers: strings(&["node-a"]),
        original_path_available: true,
    }
}

mod cases;

mod repro;
