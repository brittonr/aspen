type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type Result<T> = crate::error::Result<T>;

const MULTINODE_SCENARIO_FIXTURE_SCHEMA: &str = "molten.testing.multinode.scenario-fixture.v1";
const MULTINODE_SCENARIO_METADATA_SCHEMA: &str = "molten.testing.multinode.scenario-metadata.v1";
const MULTINODE_TOPOLOGY_PROFILE_SCHEMA: &str = "molten.testing.multinode.topology-profile.v1";
const MULTINODE_TOPOLOGY_MATRIX_SCHEMA: &str = "molten.testing.multinode.topology-profile-matrix.v1";
const MULTINODE_TOPOLOGY_MEMBERSHIP_GATE_SCHEMA: &str = "molten.testing.multinode.topology-membership-gate.v1";
const MULTINODE_RECONCILIATION_SCHEMA: &str = "molten.testing.multinode.reconciliation-gate.v1";
const LOCAL_MULTIPROCESS_PLAN_SCHEMA: &str = "molten.testing.multinode.local-multiprocess-plan.v1";
const LOCAL_MULTIPROCESS_RUN_SCHEMA: &str = "molten.testing.multinode.local-multiprocess-run.v1";
const GENERATED_DISTRIBUTED_CASE_SCHEMA: &str = "molten.testing.distributed-simulation.generated-case.v1";
const GENERATED_DISTRIBUTED_REPRO_SCHEMA: &str = "molten.testing.distributed-simulation.generated-repro.v1";
const MULTINODE_FAILURE_REPRO_PAYLOAD_SCHEMA: &str = "molten.testing.multinode.failure-repro-payload.v1";
const MULTINODE_FAILURE_REPRO_BUNDLE_SCHEMA: &str = "molten.testing.multinode.failure-repro-bundle.v1";
const MULTINODE_FAILURE_REPRO_VERIFY_SCHEMA: &str = "molten.testing.multinode.failure-repro-verify.v1";
const MULTINODE_FAILURE_REPRO_PASS_GATE_SCHEMA: &str = "molten.testing.multinode.failure-repro-pass-gate.v1";
const LIVE_TRANSPORT_VM_GATE_SCHEMA: &str = "molten.testing.nixos-vm.live-transport-gate.v1";
const VM_FAULT_SUPPORT_MATRIX_SCHEMA: &str = "molten.testing.nixos-vm.fault-support-matrix.v1";

const PASS_DECISION: &str = "pass";
const DENY_DECISION: &str = "deny";
const DIAGNOSTIC_ONLY: &str = "diagnostic-only";
const REPLAYABLE_SIMULATION: &str = "deterministic-simulation-replayable";
const NON_REPLAYABLE_VM: &str = "non-replayable-vm-observation";
const SUPPORTED: &str = "supported";
const UNAVAILABLE: &str = "unavailable";
const PROFILE_PAIRWISE_TRANSPORT: &str = "pairwise-transport";
const PROFILE_CONTROL_QUORUM: &str = "control-quorum";
const PROFILE_RESTART_REJOIN: &str = "restart-rejoin";
const PROFILE_SUBSCRIBER_PEER: &str = "subscriber-peer";
const PROFILE_WRONG_TOPOLOGY: &str = "wrong-topology-negative";
const ROLE_SENDER: &str = "sender";
const ROLE_RECEIVER: &str = "receiver";
const ROLE_VOTER: &str = "voter";
const ROLE_SUBSCRIBER: &str = "subscriber";
const ROLE_TRANSPORT_ONLY: &str = "transport-only";
const ROLE_RESTARTING_MEMBER: &str = "restarting-member";
const MEMBERSHIP_VOTER: &str = "voter";
const MEMBERSHIP_SUBSCRIBER: &str = "subscriber";
const MEMBERSHIP_TRANSPORT_ONLY: &str = "transport-only";
const CLEANUP_POLICY_REQUIRED: &str = "cleanup-required";
const MAX_MULTINODE_ITEMS: usize = 512;
const REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT: usize = 5;

const _: () = assert!(MAX_MULTINODE_ITEMS > 0);
const _: () = assert!(REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeScenarioFixture {
    pub scenario_id: String,
    pub purpose: String,
    pub evidence_scope: String,
    pub topology_profile_id: String,
    pub execution_profile_id: String,
    pub command_surface: String,
    pub expected_artifact_kinds: Vec<String>,
    pub topology_ref: String,
    pub seed_ref: String,
    pub fault_plan_ref: String,
    pub receipt_refs: Vec<String>,
    pub variance_refs: Vec<String>,
    pub diagnostic_log_refs: Vec<String>,
    pub unavailable_policy: String,
    pub unsupported_claims_pass: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeScenarioMetadata {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub fixture_ref: String,
    pub topology_profile_ref: String,
    pub metadata_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeTopologyRole {
    pub node_id: String,
    pub role: String,
    pub membership: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeTopologyLink {
    pub from: String,
    pub to: String,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeTopologyProfile {
    pub id: String,
    pub roles: Vec<MultinodeTopologyRole>,
    pub allowed_links: Vec<MultinodeTopologyLink>,
    pub evidence_scope: String,
    pub required_receipt_kinds: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeTopologyMatrix {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub profile_refs: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyMembershipClaim {
    pub profile_id: String,
    pub topology_ref: String,
    pub scenario_topology_ref: String,
    pub node_roles: Vec<MultinodeTopologyRole>,
    pub quorum_ref: Option<String>,
    pub transport_only_authority_claim: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyMembershipGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCommitEvidence {
    pub operation_id: String,
    pub commit_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeNodeSummary {
    pub node_id: String,
    pub topology_ref: String,
    pub scenario_fixture_ref: String,
    pub receipt_refs: Vec<String>,
    pub queue_ref: String,
    pub ledger_ref: String,
    pub dispatch_ref: String,
    pub ack_ref: String,
    pub protocol_ref: String,
    pub semantic_commits: Vec<SemanticCommitEvidence>,
    pub diagnostic_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationEqualityClass {
    pub name: String,
    pub refs: Vec<String>,
    pub variance_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationInput {
    pub topology_ref: String,
    pub scenario_fixture_ref: String,
    pub required_receipt_refs: Vec<String>,
    pub node_summaries: Vec<MultinodeNodeSummary>,
    pub equality_classes: Vec<ReconciliationEqualityClass>,
    pub allowed_variance_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalProcessNodePlan {
    pub node_id: String,
    pub state_root_handle: String,
    pub transport_handle: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessPlanInput {
    pub fixture_ref: String,
    pub nodes: Vec<LocalProcessNodePlan>,
    pub command_plan_ref: String,
    pub expected_receipt_refs: Vec<String>,
    pub cleanup_policy: String,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessPlan {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub plan_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessRunInput {
    pub plan_ref: String,
    pub startup_refs: Vec<String>,
    pub workflow_refs: Vec<String>,
    pub shutdown_refs: Vec<String>,
    pub cleanup_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMultiprocessRunReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedDistributedCase {
    pub case_id: String,
    pub invariant_name: String,
    pub simulation: crate::distributed_core::DistributedSimulationInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedDistributedRepro {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub case_ref: String,
    pub run_ref: String,
    pub replay_run_ref: String,
    pub repro_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeFailureReproBundleInput {
    pub scenario_fixture_ref: String,
    pub topology_ref: String,
    pub scheduler_ref: String,
    pub seed_ref: String,
    pub fault_plan_ref: String,
    pub command_refs: Vec<String>,
    pub node_summary_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub diagnostic_refs: Vec<String>,
    pub log_refs: Vec<String>,
    pub redaction_policy_ref: String,
    pub replay_status: String,
    pub diagnostic_only: bool,
    pub sealed: bool,
    pub private_attachment_refs: Vec<String>,
    pub reveal_receipt_refs: Vec<String>,
    pub claimed_payload_ref: Option<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeFailureReproBundle {
    pub payload_ref: String,
    pub bundle_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeFailureReproVerification {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub payload_ref: String,
    pub verification_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultinodeFailureReproPassGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTransportVmEvidenceInput {
    pub expected_sender_node: String,
    pub actual_sender_node: String,
    pub expected_receiver_node: String,
    pub actual_receiver_node: String,
    pub expected_peer: String,
    pub actual_peer: String,
    pub topic: String,
    pub operation_id: String,
    pub ticket_ref: String,
    pub peer_admission_ref: String,
    pub authority_ref: String,
    pub send_ref: String,
    pub receive_ref: String,
    pub ingress_ref: String,
    pub queue_ref: String,
    pub dispatch_ref: String,
    pub reconcile_ref: String,
    pub ack_ref: String,
    pub protocol_gate_ref: String,
    pub log_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveTransportVmGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmFaultSupportCase {
    pub fault_kind: String,
    pub required_capability: String,
    pub target: String,
    pub command_profile: String,
    pub expected_outcome: String,
    pub host_support: String,
    pub preflight_refs: Vec<String>,
    pub injection_refs: Vec<String>,
    pub child_refs: Vec<String>,
    pub post_fault_refs: Vec<String>,
    pub diagnostic_refs: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmFaultSupportMatrix {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub matrix_ref: String,
    pub value: IoValue,
}

pub fn default_multinode_topology_profiles() -> Vec<MultinodeTopologyProfile> {
    vec![
        MultinodeTopologyProfile {
            id: PROFILE_PAIRWISE_TRANSPORT.to_string(),
            roles: vec![
                role("sender", ROLE_SENDER, MEMBERSHIP_TRANSPORT_ONLY),
                role("receiver", ROLE_RECEIVER, MEMBERSHIP_TRANSPORT_ONLY),
            ],
            allowed_links: vec![link("sender", "receiver", "node-control")],
            evidence_scope: "live or simulated sender-to-receiver transport handoff".to_string(),
            required_receipt_kinds: vec!["send".to_string(), "receive".to_string(), "ingress".to_string()],
            caveats: vec!["transport evidence does not grant authority".to_string()],
        },
        MultinodeTopologyProfile {
            id: PROFILE_CONTROL_QUORUM.to_string(),
            roles: vec![
                role("node-a", ROLE_VOTER, MEMBERSHIP_VOTER),
                role("node-b", ROLE_VOTER, MEMBERSHIP_VOTER),
                role("node-c", ROLE_VOTER, MEMBERSHIP_VOTER),
            ],
            allowed_links: vec![
                link("node-a", "node-b", "raft-control"),
                link("node-b", "node-c", "raft-control"),
                link("node-c", "node-a", "raft-control"),
            ],
            evidence_scope: "replicated control-plane quorum evidence".to_string(),
            required_receipt_kinds: vec!["quorum".to_string(), "dispatch".to_string(), "ledger".to_string()],
            caveats: vec!["quorum evidence is scoped to declared voters".to_string()],
        },
        MultinodeTopologyProfile {
            id: PROFILE_RESTART_REJOIN.to_string(),
            roles: vec![
                role("stable", ROLE_VOTER, MEMBERSHIP_VOTER),
                role("restarting", ROLE_RESTARTING_MEMBER, MEMBERSHIP_VOTER),
            ],
            allowed_links: vec![link("stable", "restarting", "restart-rejoin")],
            evidence_scope: "state-root restart and rejoin evidence".to_string(),
            required_receipt_kinds: vec!["startup".to_string(), "shutdown".to_string(), "rejoin".to_string()],
            caveats: vec!["restart evidence does not imply WAN durability".to_string()],
        },
        MultinodeTopologyProfile {
            id: PROFILE_SUBSCRIBER_PEER.to_string(),
            roles: vec![
                role("voter", ROLE_VOTER, MEMBERSHIP_VOTER),
                role("subscriber", ROLE_SUBSCRIBER, MEMBERSHIP_SUBSCRIBER),
            ],
            allowed_links: vec![link("voter", "subscriber", "observation")],
            evidence_scope: "non-voting subscriber observation evidence".to_string(),
            required_receipt_kinds: vec!["observe".to_string(), "membership-denial".to_string()],
            caveats: vec!["subscriber evidence cannot satisfy voter membership".to_string()],
        },
        MultinodeTopologyProfile {
            id: PROFILE_WRONG_TOPOLOGY.to_string(),
            roles: vec![role("wrong-node", ROLE_TRANSPORT_ONLY, MEMBERSHIP_TRANSPORT_ONLY)],
            allowed_links: vec![link("wrong-node", "missing-node", "negative")],
            evidence_scope: "negative wrong-topology fixture".to_string(),
            required_receipt_kinds: vec!["deny".to_string()],
            caveats: vec!["negative fixture cannot satisfy pass evidence".to_string()],
        },
    ]
}

pub fn build_multinode_topology_matrix(profiles: &[MultinodeTopologyProfile]) -> Result<MultinodeTopologyMatrix> {
    let mut diagnostics = Vec::new();
    ensure_count_at_most(profiles.len(), MAX_MULTINODE_ITEMS, "topology profiles")?;
    let mut ids = OrderedSet::new();
    let mut profile_refs = Vec::with_capacity(profiles.len());
    for profile in profiles {
        collect_topology_profile_diagnostics(profile, &mut diagnostics)?;
        if !ids.insert(profile.id.as_str()) {
            push_diagnostic(&mut diagnostics, format!("duplicate-topology-profile:{}", profile.id))?;
        }
        profile_refs.push(canonical_hash(&topology_profile_value(profile)?)?);
    }
    for required in required_topology_profiles() {
        if !ids.contains(required) {
            push_diagnostic(&mut diagnostics, format!("missing-topology-profile:{required}"))?;
        }
    }
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = topology_matrix_value(&decision, profiles, &profile_refs, &diagnostics)?;
    let matrix_ref = canonical_hash(&value)?;
    Ok(MultinodeTopologyMatrix {
        decision,
        diagnostics,
        profile_refs,
        matrix_ref,
        value,
    })
}

pub fn derive_multinode_scenario_metadata(
    fixture: &MultinodeScenarioFixture,
    execution_profiles: &[crate::distributed_core::DistributedCiProfile],
    topology_profiles: &[MultinodeTopologyProfile],
) -> Result<MultinodeScenarioMetadata> {
    let mut diagnostics = scenario_fixture_diagnostics(fixture, execution_profiles, topology_profiles)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let fixture_value = scenario_fixture_value(fixture)?;
    let fixture_ref = canonical_hash(&fixture_value)?;
    let topology_profile_ref = topology_profiles
        .iter()
        .find(|profile| profile.id == fixture.topology_profile_id)
        .map(topology_profile_value)
        .transpose()?
        .map(|value| canonical_hash(&value))
        .transpose()?
        .unwrap_or_else(|| content_ref_from_text("missing-topology-profile"));
    let value = scenario_metadata_value(&ScenarioMetadataValueInput {
        decision: &decision,
        diagnostics: &diagnostics,
        fixture_ref: &fixture_ref,
        topology_profile_ref: &topology_profile_ref,
        fixture,
    })?;
    let metadata_ref = canonical_hash(&value)?;
    Ok(MultinodeScenarioMetadata {
        decision,
        diagnostics,
        fixture_ref,
        topology_profile_ref,
        metadata_ref,
        value,
    })
}

pub fn evaluate_topology_membership_claim(
    profile: &MultinodeTopologyProfile,
    claim: &TopologyMembershipClaim,
) -> Result<TopologyMembershipGate> {
    let mut diagnostics = Vec::new();
    if claim.profile_id != profile.id {
        push_diagnostic(&mut diagnostics, "topology-profile-mismatch".to_string())?;
    }
    if claim.topology_ref != claim.scenario_topology_ref {
        push_diagnostic(&mut diagnostics, "wrong-topology".to_string())?;
    }
    collect_invalid_ref_diagnostics(
        "topology claim",
        &[claim.topology_ref.clone(), claim.scenario_topology_ref.clone()],
        &mut diagnostics,
    )?;
    collect_invalid_optional_ref_diagnostics("quorum", claim.quorum_ref.as_deref(), &mut diagnostics)?;
    let declared_nodes = profile.roles.iter().map(|role| role.node_id.as_str()).collect::<OrderedSet<_>>();
    let declared_subscribers = profile
        .roles
        .iter()
        .filter(|role| role.membership == MEMBERSHIP_SUBSCRIBER)
        .map(|role| role.node_id.as_str())
        .collect::<OrderedSet<_>>();
    let requires_quorum = profile.required_receipt_kinds.iter().any(|kind| kind == "quorum");
    for role_claim in &claim.node_roles {
        if !declared_nodes.contains(role_claim.node_id.as_str()) {
            push_diagnostic(&mut diagnostics, format!("undeclared-node:{}", role_claim.node_id))?;
        }
        if declared_subscribers.contains(role_claim.node_id.as_str()) && role_claim.membership == MEMBERSHIP_VOTER {
            push_diagnostic(&mut diagnostics, format!("subscriber-promoted-to-voter:{}", role_claim.node_id))?;
        }
    }
    if claim.transport_only_authority_claim {
        push_diagnostic(&mut diagnostics, "transport-only-authority-claim".to_string())?;
    }
    if requires_quorum && claim.quorum_ref.is_none() {
        push_diagnostic(&mut diagnostics, "missing-quorum-evidence".to_string())?;
    }
    if claim.caveats.is_empty() {
        push_diagnostic(&mut diagnostics, "missing-evidence-scope-caveat".to_string())?;
    }
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = topology_membership_gate_value(&decision, profile, claim, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(TopologyMembershipGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn evaluate_reconciliation(input: &ReconciliationInput) -> Result<ReconciliationGate> {
    let mut diagnostics = reconciliation_diagnostics(input)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = reconciliation_gate_value(input, &decision, &diagnostics)?;
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReconciliationGate {
        decision,
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn build_local_multiprocess_plan(input: &LocalMultiprocessPlanInput) -> Result<LocalMultiprocessPlan> {
    let mut diagnostics = Vec::new();
    collect_invalid_ref_diagnostics(
        "local plan",
        &[input.fixture_ref.clone(), input.command_plan_ref.clone()],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("local expected receipt", &input.expected_receipt_refs, &mut diagnostics)?;
    push_if(&mut diagnostics, input.nodes.is_empty(), "local-plan-missing-nodes")?;
    push_if(&mut diagnostics, input.expected_receipt_refs.is_empty(), "local-plan-missing-expected-receipts")?;
    push_if(&mut diagnostics, input.cleanup_policy.trim().is_empty(), "local-plan-missing-cleanup-policy")?;
    push_if(
        &mut diagnostics,
        input.cleanup_policy != CLEANUP_POLICY_REQUIRED,
        "local-plan-cleanup-policy-not-reviewed",
    )?;
    collect_process_plan_collisions(input, &mut diagnostics)?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = local_multiprocess_plan_value(input, &decision, &diagnostics)?;
    let plan_ref = canonical_hash(&value)?;
    Ok(LocalMultiprocessPlan {
        decision,
        diagnostics,
        plan_ref,
        value,
    })
}

pub fn build_local_multiprocess_run_receipt(input: &LocalMultiprocessRunInput) -> Result<LocalMultiprocessRunReceipt> {
    let mut diagnostics = input.diagnostics.clone();
    collect_invalid_ref_diagnostics("local run plan", &[input.plan_ref.clone()], &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run startup", &input.startup_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run workflow", &input.workflow_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run shutdown", &input.shutdown_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("local run cleanup", &input.cleanup_refs, &mut diagnostics)?;
    push_if(&mut diagnostics, input.startup_refs.is_empty(), "local-run-missing-startup-receipts")?;
    push_if(&mut diagnostics, input.workflow_refs.is_empty(), "local-run-missing-workflow-receipts")?;
    push_if(&mut diagnostics, input.shutdown_refs.is_empty(), "local-run-missing-shutdown-receipts")?;
    push_if(&mut diagnostics, input.cleanup_refs.is_empty(), "local-run-missing-cleanup-receipts")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "local-run-missing-evidence-caveats")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = local_multiprocess_run_value(input, &decision, &diagnostics)?;
    let receipt_ref = canonical_hash(&value)?;
    Ok(LocalMultiprocessRunReceipt {
        decision,
        diagnostics,
        receipt_ref,
        value,
    })
}

pub fn run_generated_distributed_case(case: &GeneratedDistributedCase) -> Result<GeneratedDistributedRepro> {
    let case_value = generated_case_value(case)?;
    let case_ref = canonical_hash(&case_value)?;
    let first = crate::distributed_core::run_distributed_simulation(&case.simulation)?;
    let replay = crate::distributed_core::run_distributed_simulation(&case.simulation)?;
    let mut diagnostics = Vec::new();
    push_if(&mut diagnostics, first.receipt_ref != replay.receipt_ref, "generated-replay-run-ref-mismatch")?;
    push_if(
        &mut diagnostics,
        first.final_state_ref != replay.final_state_ref,
        "generated-replay-final-state-mismatch",
    )?;
    push_if(&mut diagnostics, first.event_refs != replay.event_refs, "generated-replay-event-ref-mismatch")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = generated_repro_value(case, &case_ref, &first, &replay, &decision, &diagnostics)?;
    let repro_ref = canonical_hash(&value)?;
    Ok(GeneratedDistributedRepro {
        decision,
        diagnostics,
        case_ref,
        run_ref: first.receipt_ref,
        replay_run_ref: replay.receipt_ref,
        repro_ref,
        value,
    })
}

pub fn build_multinode_failure_repro_bundle(
    input: &MultinodeFailureReproBundleInput,
) -> Result<MultinodeFailureReproBundle> {
    let payload = failure_repro_payload_value(input)?;
    let payload_ref = canonical_hash(&payload)?;
    let claimed_payload_ref = input.claimed_payload_ref.as_deref().unwrap_or(payload_ref.as_str());
    let value = failure_repro_bundle_value(input, &payload, claimed_payload_ref)?;
    let bundle_ref = canonical_hash(&value)?;
    Ok(MultinodeFailureReproBundle {
        payload_ref,
        bundle_ref,
        value,
    })
}

pub fn verify_multinode_failure_repro_bundle(
    input: &MultinodeFailureReproBundleInput,
) -> Result<MultinodeFailureReproVerification> {
    let payload = failure_repro_payload_value(input)?;
    let payload_ref = canonical_hash(&payload)?;
    let mut diagnostics = Vec::new();
    collect_failure_repro_diagnostics(input, &payload_ref, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = failure_repro_verify_value(input, &payload_ref, &decision, &diagnostics)?;
    let verification_ref = canonical_hash(&value)?;
    Ok(MultinodeFailureReproVerification {
        decision,
        diagnostics,
        payload_ref,
        verification_ref,
        value,
    })
}

pub fn gate_multinode_failure_repro_as_pass(
    verification: &MultinodeFailureReproVerification,
    diagnostic_only: bool,
) -> Result<MultinodeFailureReproPassGate> {
    let mut diagnostics = Vec::new();
    if verification.decision != PASS_DECISION {
        push_diagnostic(&mut diagnostics, "repro-verification-not-pass".to_string())?;
    }
    if diagnostic_only {
        push_diagnostic(&mut diagnostics, "diagnostic-bundle-cannot-satisfy-pass".to_string())?;
    }
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = failure_repro_pass_gate_value(verification, diagnostic_only, &decision, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(MultinodeFailureReproPassGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn evaluate_live_transport_vm_gate(input: &LiveTransportVmEvidenceInput) -> Result<LiveTransportVmGate> {
    let mut diagnostics = Vec::new();
    push_if(
        &mut diagnostics,
        input.expected_sender_node != input.actual_sender_node,
        "live-transport-sender-node-mismatch",
    )?;
    push_if(
        &mut diagnostics,
        input.expected_receiver_node != input.actual_receiver_node,
        "live-transport-receiver-node-mismatch",
    )?;
    push_if(&mut diagnostics, input.expected_peer != input.actual_peer, "live-transport-peer-mismatch")?;
    push_if(&mut diagnostics, input.topic.trim().is_empty(), "live-transport-missing-topic")?;
    push_if(&mut diagnostics, input.operation_id.trim().is_empty(), "live-transport-missing-operation-id")?;
    collect_invalid_ref_diagnostics("live transport", &live_transport_refs(input), &mut diagnostics)?;
    push_if(&mut diagnostics, input.ticket_ref.trim().is_empty(), "live-transport-missing-ticket")?;
    push_if(&mut diagnostics, input.receive_ref.trim().is_empty(), "live-transport-missing-receive")?;
    push_if(&mut diagnostics, input.protocol_gate_ref.trim().is_empty(), "live-transport-missing-protocol-gate")?;
    push_if(&mut diagnostics, input.log_refs.is_empty(), "live-transport-missing-diagnostic-logs")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "live-transport-missing-scope-caveat")?;
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = live_transport_vm_gate_value(input, &decision, &diagnostics)?;
    let gate_ref = canonical_hash(&value)?;
    Ok(LiveTransportVmGate {
        decision,
        diagnostics,
        gate_ref,
        value,
    })
}

pub fn build_vm_fault_support_matrix(cases: &[VmFaultSupportCase]) -> Result<VmFaultSupportMatrix> {
    let mut diagnostics = Vec::new();
    ensure_count_at_most(cases.len(), MAX_MULTINODE_ITEMS, "VM fault support cases")?;
    push_if(&mut diagnostics, cases.is_empty(), "vm-fault-support-matrix-empty")?;
    for case in cases {
        collect_vm_fault_support_diagnostics(case, &mut diagnostics)?;
    }
    diagnostics.sort();
    diagnostics.dedup();
    let decision = decision_from_diagnostics(&diagnostics).to_string();
    let value = vm_fault_support_matrix_value(cases, &decision, &diagnostics)?;
    let matrix_ref = canonical_hash(&value)?;
    Ok(VmFaultSupportMatrix {
        decision,
        diagnostics,
        matrix_ref,
        value,
    })
}

struct ScenarioMetadataValueInput<'a> {
    decision: &'a str,
    diagnostics: &'a [String],
    fixture_ref: &'a str,
    topology_profile_ref: &'a str,
    fixture: &'a MultinodeScenarioFixture,
}

fn scenario_fixture_diagnostics(
    fixture: &MultinodeScenarioFixture,
    execution_profiles: &[crate::distributed_core::DistributedCiProfile],
    topology_profiles: &[MultinodeTopologyProfile],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    collect_required_text_diagnostic("scenario-id", &fixture.scenario_id, &mut diagnostics)?;
    collect_required_text_diagnostic("purpose", &fixture.purpose, &mut diagnostics)?;
    collect_required_text_diagnostic("evidence-scope", &fixture.evidence_scope, &mut diagnostics)?;
    collect_required_text_diagnostic("topology-profile", &fixture.topology_profile_id, &mut diagnostics)?;
    collect_required_text_diagnostic("execution-profile", &fixture.execution_profile_id, &mut diagnostics)?;
    collect_required_text_diagnostic("command-surface", &fixture.command_surface, &mut diagnostics)?;
    collect_required_text_diagnostic("unavailable-policy", &fixture.unavailable_policy, &mut diagnostics)?;
    push_if(&mut diagnostics, fixture.expected_artifact_kinds.is_empty(), "fixture-missing-artifact-kind")?;
    push_if(&mut diagnostics, fixture.receipt_refs.is_empty(), "fixture-missing-receipt-ref")?;
    push_if(&mut diagnostics, fixture.variance_refs.is_empty(), "fixture-missing-variance-ref")?;
    push_if(&mut diagnostics, fixture.diagnostic_log_refs.is_empty(), "fixture-missing-diagnostic-log-ref")?;
    push_if(&mut diagnostics, fixture.caveats.is_empty(), "fixture-missing-evidence-caveat")?;
    if fixture.unsupported_claims_pass {
        push_diagnostic(&mut diagnostics, "fixture-unsupported-pass-claim".to_string())?;
    }
    collect_invalid_ref_diagnostics(
        "scenario fixture",
        &[
            fixture.topology_ref.clone(),
            fixture.seed_ref.clone(),
            fixture.fault_plan_ref.clone(),
        ],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("scenario receipt", &fixture.receipt_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("scenario variance", &fixture.variance_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("scenario diagnostic log", &fixture.diagnostic_log_refs, &mut diagnostics)?;
    match execution_profiles.iter().find(|profile| profile.id == fixture.execution_profile_id) {
        Some(profile) => {
            if fixture.command_surface != profile.command {
                push_diagnostic(&mut diagnostics, "fixture-command-profile-mismatch".to_string())?;
            }
            if fixture.expected_artifact_kinds != profile.expected_artifact_kinds {
                push_diagnostic(&mut diagnostics, "fixture-artifact-kind-mismatch".to_string())?;
            }
        }
        None => push_diagnostic(&mut diagnostics, "fixture-execution-profile-missing".to_string())?,
    }
    if !topology_profiles.iter().any(|profile| profile.id == fixture.topology_profile_id) {
        push_diagnostic(&mut diagnostics, "fixture-topology-profile-missing".to_string())?;
    }
    Ok(diagnostics)
}

fn collect_topology_profile_diagnostics(
    profile: &MultinodeTopologyProfile,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    collect_required_text_diagnostic("topology-profile-id", &profile.id, diagnostics)?;
    collect_required_text_diagnostic("topology-profile-scope", &profile.evidence_scope, diagnostics)?;
    push_if(diagnostics, profile.roles.is_empty(), "topology-profile-missing-roles")?;
    push_if(diagnostics, profile.allowed_links.is_empty(), "topology-profile-missing-links")?;
    push_if(diagnostics, profile.required_receipt_kinds.is_empty(), "topology-profile-missing-required-receipts")?;
    push_if(diagnostics, profile.caveats.is_empty(), "topology-profile-missing-caveats")?;
    let mut role_nodes = OrderedSet::new();
    for profile_role in &profile.roles {
        collect_required_text_diagnostic("topology-role-node", &profile_role.node_id, diagnostics)?;
        collect_required_text_diagnostic("topology-role", &profile_role.role, diagnostics)?;
        validate_membership(&profile_role.membership, diagnostics)?;
        if !role_nodes.insert(profile_role.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("duplicate-topology-role-node:{}", profile_role.node_id))?;
        }
    }
    for profile_link in &profile.allowed_links {
        collect_required_text_diagnostic("topology-link-from", &profile_link.from, diagnostics)?;
        collect_required_text_diagnostic("topology-link-to", &profile_link.to, diagnostics)?;
        collect_required_text_diagnostic("topology-link-topic", &profile_link.topic, diagnostics)?;
    }
    Ok(())
}

fn validate_membership(membership: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match membership {
        MEMBERSHIP_VOTER | MEMBERSHIP_SUBSCRIBER | MEMBERSHIP_TRANSPORT_ONLY => Ok(()),
        _ => push_diagnostic(diagnostics, format!("unsupported-topology-membership:{membership}")),
    }
}

fn required_topology_profiles() -> [&'static str; REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT] {
    [
        PROFILE_PAIRWISE_TRANSPORT,
        PROFILE_CONTROL_QUORUM,
        PROFILE_RESTART_REJOIN,
        PROFILE_SUBSCRIBER_PEER,
        PROFILE_WRONG_TOPOLOGY,
    ]
}

fn reconciliation_diagnostics(input: &ReconciliationInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    collect_invalid_ref_diagnostics(
        "reconciliation root",
        &[input.topology_ref.clone(), input.scenario_fixture_ref.clone()],
        &mut diagnostics,
    )?;
    collect_invalid_ref_diagnostics("reconciliation required receipt", &input.required_receipt_refs, &mut diagnostics)?;
    collect_invalid_ref_diagnostics("reconciliation variance", &input.allowed_variance_refs, &mut diagnostics)?;
    push_if(&mut diagnostics, input.node_summaries.is_empty(), "reconciliation-missing-node-summaries")?;
    push_if(&mut diagnostics, input.required_receipt_refs.is_empty(), "reconciliation-log-only-claim")?;
    push_if(&mut diagnostics, input.caveats.is_empty(), "reconciliation-missing-evidence-caveat")?;
    let mut available_receipts = OrderedSet::new();
    let mut commits = OrderedMap::<String, OrderedSet<String>>::new();
    for summary in &input.node_summaries {
        collect_node_summary_diagnostics(input, summary, &mut diagnostics)?;
        for receipt_ref in &summary.receipt_refs {
            available_receipts.insert(receipt_ref.as_str());
        }
        for commit in &summary.semantic_commits {
            commits.entry(commit.operation_id.clone()).or_default().insert(commit.commit_ref.clone());
        }
    }
    for required_ref in &input.required_receipt_refs {
        if !available_receipts.contains(required_ref.as_str()) {
            push_diagnostic(&mut diagnostics, format!("required-receipt-missing:{required_ref}"))?;
        }
    }
    for equality in &input.equality_classes {
        collect_equality_class_diagnostics(input, equality, &mut diagnostics)?;
    }
    for (operation_id, commit_refs) in commits {
        if commit_refs.len() > 1 {
            push_diagnostic(&mut diagnostics, format!("duplicate-semantic-commit:{operation_id}"))?;
        }
    }
    Ok(diagnostics)
}

fn collect_node_summary_diagnostics(
    input: &ReconciliationInput,
    summary: &MultinodeNodeSummary,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    collect_required_text_diagnostic("node-summary-node", &summary.node_id, diagnostics)?;
    if summary.topology_ref != input.topology_ref {
        push_diagnostic(diagnostics, format!("node-summary-topology-mismatch:{}", summary.node_id))?;
    }
    if summary.scenario_fixture_ref != input.scenario_fixture_ref {
        push_diagnostic(diagnostics, format!("node-summary-scenario-mismatch:{}", summary.node_id))?;
    }
    collect_invalid_ref_diagnostics(
        "node summary",
        &[
            summary.topology_ref.clone(),
            summary.scenario_fixture_ref.clone(),
            summary.queue_ref.clone(),
            summary.ledger_ref.clone(),
            summary.dispatch_ref.clone(),
            summary.ack_ref.clone(),
            summary.protocol_ref.clone(),
        ],
        diagnostics,
    )?;
    collect_invalid_ref_diagnostics("node summary receipt", &summary.receipt_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("node summary log", &summary.diagnostic_log_refs, diagnostics)?;
    push_if(diagnostics, summary.receipt_refs.is_empty(), "node-summary-missing-receipts")?;
    push_if(diagnostics, summary.diagnostic_log_refs.is_empty(), "node-summary-missing-diagnostic-logs")?;
    let mut node_commits = OrderedSet::new();
    for commit in &summary.semantic_commits {
        collect_required_text_diagnostic("semantic-commit-operation", &commit.operation_id, diagnostics)?;
        collect_invalid_ref_diagnostics("semantic commit", &[commit.commit_ref.clone()], diagnostics)?;
        if !node_commits.insert(commit.operation_id.as_str()) {
            push_diagnostic(diagnostics, format!("duplicate-node-semantic-commit:{}", commit.operation_id))?;
        }
    }
    Ok(())
}

fn collect_equality_class_diagnostics(
    input: &ReconciliationInput,
    equality: &ReconciliationEqualityClass,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    collect_required_text_diagnostic("equality-class", &equality.name, diagnostics)?;
    collect_invalid_ref_diagnostics("equality class", &equality.refs, diagnostics)?;
    collect_invalid_optional_ref_diagnostics("equality variance", equality.variance_ref.as_deref(), diagnostics)?;
    let distinct_refs = equality.refs.iter().map(String::as_str).collect::<OrderedSet<_>>();
    if distinct_refs.len() > 1 {
        match equality.variance_ref.as_deref() {
            Some(variance_ref) if input.allowed_variance_refs.iter().any(|allowed| allowed == variance_ref) => Ok(()),
            _ => push_diagnostic(diagnostics, format!("divergent-ref-class:{}", equality.name)),
        }?;
    }
    Ok(())
}

fn collect_process_plan_collisions(input: &LocalMultiprocessPlanInput, diagnostics: &mut Vec<String>) -> Result<()> {
    let mut nodes = OrderedSet::new();
    let mut state_roots = OrderedMap::<&str, &str>::new();
    let mut transports = OrderedMap::<&str, &str>::new();
    for node in &input.nodes {
        collect_required_text_diagnostic("local node", &node.node_id, diagnostics)?;
        collect_required_text_diagnostic("local state root", &node.state_root_handle, diagnostics)?;
        collect_required_text_diagnostic("local transport", &node.transport_handle, diagnostics)?;
        if !nodes.insert(node.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("local-plan-duplicate-node:{}", node.node_id))?;
        }
        if let Some(existing) = state_roots.insert(node.state_root_handle.as_str(), node.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("local-plan-state-root-collision:{existing}:{}", node.node_id))?;
        }
        if let Some(existing) = transports.insert(node.transport_handle.as_str(), node.node_id.as_str()) {
            push_diagnostic(diagnostics, format!("local-plan-transport-collision:{existing}:{}", node.node_id))?;
        }
    }
    Ok(())
}

fn collect_failure_repro_diagnostics(
    input: &MultinodeFailureReproBundleInput,
    payload_ref: &str,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    let all_root_refs = [
        input.scenario_fixture_ref.clone(),
        input.topology_ref.clone(),
        input.scheduler_ref.clone(),
        input.seed_ref.clone(),
        input.fault_plan_ref.clone(),
        input.redaction_policy_ref.clone(),
    ];
    collect_invalid_ref_diagnostics("failure repro root", &all_root_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro command", &input.command_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro node summary", &input.node_summary_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro receipt", &input.receipt_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro diagnostic", &input.diagnostic_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro log", &input.log_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro private", &input.private_attachment_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("failure repro reveal", &input.reveal_receipt_refs, diagnostics)?;
    collect_invalid_optional_ref_diagnostics(
        "failure repro claimed seal",
        input.claimed_payload_ref.as_deref(),
        diagnostics,
    )?;
    push_if(diagnostics, input.command_refs.is_empty(), "failure-repro-missing-commands")?;
    push_if(diagnostics, input.node_summary_refs.is_empty(), "failure-repro-missing-node-summaries")?;
    push_if(diagnostics, input.receipt_refs.is_empty(), "failure-repro-missing-receipts")?;
    push_if(diagnostics, input.log_refs.is_empty(), "failure-repro-missing-logs")?;
    push_if(diagnostics, input.caveats.is_empty(), "failure-repro-missing-caveats")?;
    push_if(diagnostics, !input.sealed, "failure-repro-unsealed")?;
    push_if(
        diagnostics,
        !input.private_attachment_refs.is_empty() && input.reveal_receipt_refs.is_empty(),
        "failure-repro-private-without-reveal",
    )?;
    push_if(diagnostics, input.redaction_policy_ref.trim().is_empty(), "failure-repro-missing-redaction-policy")?;
    if let Some(claimed) = &input.claimed_payload_ref {
        if claimed != payload_ref {
            push_diagnostic(diagnostics, "failure-repro-seal-mismatch".to_string())?;
        }
    }
    Ok(())
}

fn collect_vm_fault_support_diagnostics(case: &VmFaultSupportCase, diagnostics: &mut Vec<String>) -> Result<()> {
    collect_required_text_diagnostic("fault-kind", &case.fault_kind, diagnostics)?;
    collect_required_text_diagnostic("fault-capability", &case.required_capability, diagnostics)?;
    collect_required_text_diagnostic("fault-target", &case.target, diagnostics)?;
    collect_required_text_diagnostic("fault-command-profile", &case.command_profile, diagnostics)?;
    collect_required_text_diagnostic("fault-expected-outcome", &case.expected_outcome, diagnostics)?;
    collect_required_text_diagnostic("fault-host-support", &case.host_support, diagnostics)?;
    collect_invalid_ref_diagnostics("fault preflight", &case.preflight_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault injection", &case.injection_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault child", &case.child_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault post", &case.post_fault_refs, diagnostics)?;
    collect_invalid_ref_diagnostics("fault diagnostic", &case.diagnostic_refs, diagnostics)?;
    let claims_pass = case.expected_outcome == PASS_DECISION;
    if claims_pass && case.host_support != SUPPORTED {
        push_diagnostic(diagnostics, format!("vm-fault-unsupported-pass:{}", case.fault_kind))?;
    }
    if claims_pass && case.injection_refs.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-missing-injection:{}", case.fault_kind))?;
    }
    if claims_pass && case.child_refs.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-missing-child:{}", case.fault_kind))?;
    }
    if case.host_support == UNAVAILABLE && case.diagnostic_refs.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-unavailable-missing-diagnostic:{}", case.fault_kind))?;
    }
    if case.caveats.is_empty() {
        push_diagnostic(diagnostics, format!("vm-fault-missing-caveat:{}", case.fault_kind))?;
    }
    Ok(())
}

fn scenario_fixture_value(fixture: &MultinodeScenarioFixture) -> Result<IoValue> {
    ensure_count_at_most(fixture.expected_artifact_kinds.len(), MAX_MULTINODE_ITEMS, "fixture artifacts")?;
    ensure_count_at_most(fixture.receipt_refs.len(), MAX_MULTINODE_ITEMS, "fixture receipts")?;
    ensure_count_at_most(fixture.variance_refs.len(), MAX_MULTINODE_ITEMS, "fixture variance")?;
    ensure_count_at_most(fixture.diagnostic_log_refs.len(), MAX_MULTINODE_ITEMS, "fixture logs")?;
    Ok(record("multinode-scenario-fixture-v1", vec![
        string(MULTINODE_SCENARIO_FIXTURE_SCHEMA),
        record("id", vec![string(&fixture.scenario_id)]),
        record("purpose", vec![string(&fixture.purpose)]),
        record("evidence-scope", vec![string(&fixture.evidence_scope)]),
        record("topology-profile", vec![string(&fixture.topology_profile_id)]),
        record("execution-profile", vec![string(&fixture.execution_profile_id)]),
        record("command", vec![string(&fixture.command_surface)]),
        record("artifact-kinds", vec![strings_sequence(&fixture.expected_artifact_kinds)]),
        record("topology", vec![string(&fixture.topology_ref)]),
        record("seed", vec![string(&fixture.seed_ref)]),
        record("fault-plan", vec![string(&fixture.fault_plan_ref)]),
        record("receipts", vec![refs_sequence(&fixture.receipt_refs)]),
        record("variance", vec![refs_sequence(&fixture.variance_refs)]),
        record("diagnostic-logs", vec![refs_sequence(&fixture.diagnostic_log_refs)]),
        record("unavailable-policy", vec![string(&fixture.unavailable_policy)]),
        record("unsupported-claims-pass", vec![bool_value(fixture.unsupported_claims_pass)]),
        record("caveats", vec![strings_sequence(&fixture.caveats)]),
        checks_value(&[
            ("fixture-declarative", PASS_DECISION),
            ("ambient-runtime-state-excluded", PASS_DECISION),
            ("logs-diagnostic-only", PASS_DECISION),
        ]),
    ]))
}

fn scenario_metadata_value(input: &ScenarioMetadataValueInput<'_>) -> Result<IoValue> {
    Ok(record("multinode-scenario-metadata-v1", vec![
        string(MULTINODE_SCENARIO_METADATA_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("fixture", vec![string(input.fixture_ref)]),
        record("topology-profile-ref", vec![string(input.topology_profile_ref)]),
        record("scenario-id", vec![string(&input.fixture.scenario_id)]),
        record("execution-profile", vec![string(&input.fixture.execution_profile_id)]),
        record("command", vec![string(&input.fixture.command_surface)]),
        record("artifact-kinds", vec![strings_sequence(&input.fixture.expected_artifact_kinds)]),
        record("receipts", vec![refs_sequence(&input.fixture.receipt_refs)]),
        record("variance", vec![refs_sequence(&input.fixture.variance_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        checks_value(&[
            ("fixture-ref-bound", status(input.decision == PASS_DECISION)),
            ("metadata-derived-without-ambient-state", PASS_DECISION),
            ("unsupported-execution-not-pass", status(!input.fixture.unsupported_claims_pass)),
        ]),
    ]))
}

fn topology_profile_value(profile: &MultinodeTopologyProfile) -> Result<IoValue> {
    Ok(record("multinode-topology-profile-v1", vec![
        string(MULTINODE_TOPOLOGY_PROFILE_SCHEMA),
        record("id", vec![string(&profile.id)]),
        record("roles", vec![sequence(topology_role_values(&profile.roles))]),
        record("links", vec![sequence(topology_link_values(&profile.allowed_links))]),
        record("evidence-scope", vec![string(&profile.evidence_scope)]),
        record("required-receipts", vec![strings_sequence(&profile.required_receipt_kinds)]),
        record("caveats", vec![strings_sequence(&profile.caveats)]),
        checks_value(&[
            ("roles-explicit", status(!profile.roles.is_empty())),
            ("links-explicit", status(!profile.allowed_links.is_empty())),
            ("transport-not-authority", PASS_DECISION),
        ]),
    ]))
}

fn topology_matrix_value(
    decision: &str,
    profiles: &[MultinodeTopologyProfile],
    profile_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
    let profile_ids = profiles.iter().map(|profile| profile.id.clone()).collect::<Vec<_>>();
    Ok(record("multinode-topology-profile-matrix-v1", vec![
        string(MULTINODE_TOPOLOGY_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile-ids", vec![strings_sequence(&profile_ids)]),
        record("profiles", vec![refs_sequence(profile_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("required-profiles-present", status(decision == PASS_DECISION)),
            ("role-membership-explicit", PASS_DECISION),
        ]),
    ]))
}

fn topology_membership_gate_value(
    decision: &str,
    profile: &MultinodeTopologyProfile,
    claim: &TopologyMembershipClaim,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("multinode-topology-membership-gate-v1", vec![
        string(MULTINODE_TOPOLOGY_MEMBERSHIP_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile", vec![string(&profile.id)]),
        record("topology", vec![string(&claim.topology_ref)]),
        record("scenario-topology", vec![string(&claim.scenario_topology_ref)]),
        record("claimed-roles", vec![sequence(topology_role_values(&claim.node_roles))]),
        record("quorum", vec![optional_ref_value(claim.quorum_ref.as_deref())]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&claim.caveats)]),
        checks_value(&[
            ("subscriber-not-voter", status(!diagnostics.iter().any(|item| item.contains("subscriber")))),
            ("wrong-topology-denies", status(!diagnostics.iter().any(|item| item == "wrong-topology"))),
            ("transport-not-authority", status(!claim.transport_only_authority_claim)),
        ]),
    ]))
}

fn reconciliation_gate_value(input: &ReconciliationInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("multinode-reconciliation-gate-v1", vec![
        string(MULTINODE_RECONCILIATION_SCHEMA),
        record("decision", vec![string(decision)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("scenario-fixture", vec![string(&input.scenario_fixture_ref)]),
        record("required-receipts", vec![refs_sequence(&input.required_receipt_refs)]),
        record("node-summaries", vec![sequence(node_summary_values(&input.node_summaries))]),
        record("equality", vec![sequence(equality_class_values(&input.equality_classes))]),
        record("allowed-variance", vec![refs_sequence(&input.allowed_variance_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("node-summaries-bound", status(!input.node_summaries.is_empty())),
            ("logs-diagnostic-only", PASS_DECISION),
            ("undeclared-drift-denies", status(decision == PASS_DECISION)),
        ]),
    ]))
}

fn local_multiprocess_plan_value(
    input: &LocalMultiprocessPlanInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("local-multiprocess-plan-v1", vec![
        string(LOCAL_MULTIPROCESS_PLAN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("fixture", vec![string(&input.fixture_ref)]),
        record("nodes", vec![sequence(local_process_node_values(&input.nodes))]),
        record("command-plan", vec![string(&input.command_plan_ref)]),
        record("expected-receipts", vec![refs_sequence(&input.expected_receipt_refs)]),
        record("cleanup-policy", vec![string(&input.cleanup_policy)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            (
                "state-roots-isolated",
                status(!diagnostics.iter().any(|item| item.contains("state-root-collision"))),
            ),
            ("transports-isolated", status(!diagnostics.iter().any(|item| item.contains("transport-collision")))),
            ("cleanup-policy-declared", status(input.cleanup_policy == CLEANUP_POLICY_REQUIRED)),
        ]),
    ]))
}

fn local_multiprocess_run_value(
    input: &LocalMultiprocessRunInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("local-multiprocess-run-v1", vec![
        string(LOCAL_MULTIPROCESS_RUN_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plan", vec![string(&input.plan_ref)]),
        record("startup", vec![refs_sequence(&input.startup_refs)]),
        record("workflow", vec![refs_sequence(&input.workflow_refs)]),
        record("shutdown", vec![refs_sequence(&input.shutdown_refs)]),
        record("cleanup", vec![refs_sequence(&input.cleanup_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("process-receipts-bound", status(decision == PASS_DECISION)),
            ("local-evidence-not-vm-evidence", PASS_DECISION),
            ("cleanup-recorded", status(!input.cleanup_refs.is_empty())),
        ]),
    ]))
}

fn generated_case_value(case: &GeneratedDistributedCase) -> Result<IoValue> {
    let topology_ref =
        canonical_hash(&crate::distributed_core::distributed_topology_value(&case.simulation.topology)?)?;
    let scheduler_ref = canonical_hash(&crate::distributed_core::scheduler_profile_value(&case.simulation.scheduler)?)?;
    let seed_ref = canonical_hash(&crate::distributed_core::seed_value(&case.simulation.seed)?)?;
    let fault_plan_ref = canonical_hash(&crate::distributed_core::fault_plan_value(&case.simulation.fault_plan)?)?;
    Ok(record("generated-distributed-case-v1", vec![
        string(GENERATED_DISTRIBUTED_CASE_SCHEMA),
        record("id", vec![string(&case.case_id)]),
        record("invariant", vec![string(&case.invariant_name)]),
        record("topology", vec![string(topology_ref)]),
        record("scheduler", vec![string(scheduler_ref)]),
        record("seed", vec![string(seed_ref)]),
        record("fault-plan", vec![string(fault_plan_ref)]),
        record("commands", vec![strings_sequence(
            &case.simulation.commands.iter().map(|command| command.operation_id.clone()).collect::<Vec<_>>(),
        )]),
        checks_value(&[
            ("seed-bound", PASS_DECISION),
            ("ambient-randomness-excluded", PASS_DECISION),
        ]),
    ]))
}

fn generated_repro_value(
    case: &GeneratedDistributedCase,
    case_ref: &str,
    first: &crate::distributed_core::DistributedSimulationRun,
    replay: &crate::distributed_core::DistributedSimulationRun,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("generated-distributed-repro-v1", vec![
        string(GENERATED_DISTRIBUTED_REPRO_SCHEMA),
        record("decision", vec![string(decision)]),
        record("case", vec![string(case_ref)]),
        record("invariant", vec![string(&case.invariant_name)]),
        record("run", vec![string(&first.receipt_ref)]),
        record("replay-run", vec![string(&replay.receipt_ref)]),
        record("topology", vec![string(&first.topology_ref)]),
        record("scheduler", vec![string(&first.scheduler_ref)]),
        record("seed", vec![string(&first.seed_ref)]),
        record("fault-plan", vec![string(&first.fault_plan_ref)]),
        record("events", vec![refs_sequence(&first.event_refs)]),
        record("final-state", vec![string(&first.final_state_ref)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("evidence-scope", vec![string(DIAGNOSTIC_ONLY)]),
        checks_value(&[
            ("replay-seed-bound", status(first.receipt_ref == replay.receipt_ref)),
            ("diagnostic-only-unless-gated", PASS_DECISION),
        ]),
    ]))
}

fn failure_repro_payload_value(input: &MultinodeFailureReproBundleInput) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-payload-v1", vec![
        string(MULTINODE_FAILURE_REPRO_PAYLOAD_SCHEMA),
        record("scenario-fixture", vec![string(&input.scenario_fixture_ref)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("scheduler", vec![string(&input.scheduler_ref)]),
        record("seed", vec![string(&input.seed_ref)]),
        record("fault-plan", vec![string(&input.fault_plan_ref)]),
        record("commands", vec![refs_sequence(&input.command_refs)]),
        record("node-summaries", vec![refs_sequence(&input.node_summary_refs)]),
        record("receipts", vec![refs_sequence(&input.receipt_refs)]),
        record("diagnostics", vec![refs_sequence(&input.diagnostic_refs)]),
        record("logs", vec![refs_sequence(&input.log_refs)]),
        record("redaction-policy", vec![string(&input.redaction_policy_ref)]),
        record("replay-status", vec![string(&input.replay_status)]),
        record("diagnostic-only", vec![bool_value(input.diagnostic_only)]),
        record("private-attachments", vec![refs_sequence(&input.private_attachment_refs)]),
        record("reveal-receipts", vec![refs_sequence(&input.reveal_receipt_refs)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
    ]))
}

fn failure_repro_bundle_value(
    input: &MultinodeFailureReproBundleInput,
    payload: &IoValue,
    claimed_payload_ref: &str,
) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-bundle-v1", vec![
        string(MULTINODE_FAILURE_REPRO_BUNDLE_SCHEMA),
        record("sealed", vec![bool_value(input.sealed)]),
        record("payload-ref", vec![string(claimed_payload_ref)]),
        record("payload", vec![payload.clone()]),
        checks_value(&[
            ("sealed", status(input.sealed)),
            ("diagnostic-only-unless-gated", PASS_DECISION),
            (
                "private-content-requires-reveal",
                status(input.private_attachment_refs.is_empty() || !input.reveal_receipt_refs.is_empty()),
            ),
        ]),
    ]))
}

fn failure_repro_verify_value(
    input: &MultinodeFailureReproBundleInput,
    payload_ref: &str,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-verify-v1", vec![
        string(MULTINODE_FAILURE_REPRO_VERIFY_SCHEMA),
        record("decision", vec![string(decision)]),
        record("payload", vec![string(payload_ref)]),
        record("claimed-payload", vec![optional_ref_value(input.claimed_payload_ref.as_deref())]),
        record("replay-status", vec![string(&input.replay_status)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("seal-metadata-valid", status(!diagnostics.iter().any(|item| item.contains("seal")))),
            ("redaction-policy-bound", status(!diagnostics.iter().any(|item| item.contains("redaction")))),
            ("diagnostic-only-not-pass", PASS_DECISION),
        ]),
    ]))
}

fn failure_repro_pass_gate_value(
    verification: &MultinodeFailureReproVerification,
    diagnostic_only: bool,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("multinode-failure-repro-pass-gate-v1", vec![
        string(MULTINODE_FAILURE_REPRO_PASS_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("verification", vec![string(&verification.verification_ref)]),
        record("payload", vec![string(&verification.payload_ref)]),
        record("diagnostic-only", vec![bool_value(diagnostic_only)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("verified-before-use", status(verification.decision == PASS_DECISION)),
            ("diagnostic-bundle-not-pass", status(!diagnostic_only)),
        ]),
    ]))
}

fn live_transport_vm_gate_value(
    input: &LiveTransportVmEvidenceInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("nixos-vm-live-transport-gate-v1", vec![
        string(LIVE_TRANSPORT_VM_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("sender", vec![record("node", vec![string(&input.actual_sender_node)])]),
        record("receiver", vec![record("node", vec![string(&input.actual_receiver_node)])]),
        record("peer", vec![string(&input.actual_peer)]),
        record("topic", vec![string(&input.topic)]),
        record("operation", vec![string(&input.operation_id)]),
        record("ticket", vec![string(&input.ticket_ref)]),
        record("peer-admission", vec![string(&input.peer_admission_ref)]),
        record("authority", vec![string(&input.authority_ref)]),
        record("send", vec![string(&input.send_ref)]),
        record("receive", vec![string(&input.receive_ref)]),
        record("ingress", vec![string(&input.ingress_ref)]),
        record("queue", vec![string(&input.queue_ref)]),
        record("dispatch", vec![string(&input.dispatch_ref)]),
        record("reconcile", vec![string(&input.reconcile_ref)]),
        record("ack", vec![string(&input.ack_ref)]),
        record("protocol-gate", vec![string(&input.protocol_gate_ref)]),
        record("logs", vec![refs_sequence(&input.log_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
        checks_value(&[
            ("receive-receipt-bound", status(!input.receive_ref.trim().is_empty())),
            ("protocol-gate-bound", status(!input.protocol_gate_ref.trim().is_empty())),
            ("logs-diagnostic-only", PASS_DECISION),
            ("vm-topology-scoped", PASS_DECISION),
        ]),
    ]))
}

fn vm_fault_support_matrix_value(
    cases: &[VmFaultSupportCase],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("nixos-vm-fault-support-matrix-v1", vec![
        string(VM_FAULT_SUPPORT_MATRIX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("cases", vec![sequence(cases.iter().map(vm_fault_case_value).collect::<Vec<_>>())]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("support-status-explicit", status(!cases.is_empty())),
            ("unsupported-is-not-pass", status(!diagnostics.iter().any(|item| item.contains("unsupported-pass")))),
            ("canonical-diagnostics-required", PASS_DECISION),
        ]),
    ]))
}

fn vm_fault_case_value(case: &VmFaultSupportCase) -> IoValue {
    record("fault", vec![
        record("kind", vec![string(&case.fault_kind)]),
        record("capability", vec![string(&case.required_capability)]),
        record("target", vec![string(&case.target)]),
        record("command-profile", vec![string(&case.command_profile)]),
        record("expected-outcome", vec![string(&case.expected_outcome)]),
        record("host-support", vec![string(&case.host_support)]),
        record("preflight", vec![refs_sequence(&case.preflight_refs)]),
        record("injection", vec![refs_sequence(&case.injection_refs)]),
        record("children", vec![refs_sequence(&case.child_refs)]),
        record("post-fault", vec![refs_sequence(&case.post_fault_refs)]),
        record("diagnostics", vec![refs_sequence(&case.diagnostic_refs)]),
        record("caveats", vec![strings_sequence(&case.caveats)]),
    ])
}

fn topology_role_values(roles: &[MultinodeTopologyRole]) -> Vec<IoValue> {
    roles
        .iter()
        .map(|role_item| {
            record("role", vec![
                record("node", vec![string(&role_item.node_id)]),
                record("role", vec![string(&role_item.role)]),
                record("membership", vec![string(&role_item.membership)]),
            ])
        })
        .collect()
}

fn topology_link_values(links: &[MultinodeTopologyLink]) -> Vec<IoValue> {
    links
        .iter()
        .map(|link_item| {
            record("link", vec![
                record("from", vec![string(&link_item.from)]),
                record("to", vec![string(&link_item.to)]),
                record("topic", vec![string(&link_item.topic)]),
            ])
        })
        .collect()
}

fn node_summary_values(summaries: &[MultinodeNodeSummary]) -> Vec<IoValue> {
    summaries
        .iter()
        .map(|summary| {
            record("node-summary", vec![
                record("node", vec![string(&summary.node_id)]),
                record("topology", vec![string(&summary.topology_ref)]),
                record("scenario-fixture", vec![string(&summary.scenario_fixture_ref)]),
                record("receipts", vec![refs_sequence(&summary.receipt_refs)]),
                record("queue", vec![string(&summary.queue_ref)]),
                record("ledger", vec![string(&summary.ledger_ref)]),
                record("dispatch", vec![string(&summary.dispatch_ref)]),
                record("ack", vec![string(&summary.ack_ref)]),
                record("protocol", vec![string(&summary.protocol_ref)]),
                record("commits", vec![sequence(semantic_commit_values(&summary.semantic_commits))]),
                record("logs", vec![refs_sequence(&summary.diagnostic_log_refs)]),
            ])
        })
        .collect()
}

fn semantic_commit_values(commits: &[SemanticCommitEvidence]) -> Vec<IoValue> {
    commits
        .iter()
        .map(|commit| {
            record("commit", vec![
                record("operation", vec![string(&commit.operation_id)]),
                record("ref", vec![string(&commit.commit_ref)]),
            ])
        })
        .collect()
}

fn equality_class_values(classes: &[ReconciliationEqualityClass]) -> Vec<IoValue> {
    classes
        .iter()
        .map(|class| {
            record("equality", vec![
                record("name", vec![string(&class.name)]),
                record("refs", vec![refs_sequence(&class.refs)]),
                record("variance", vec![optional_ref_value(class.variance_ref.as_deref())]),
            ])
        })
        .collect()
}

fn local_process_node_values(nodes: &[LocalProcessNodePlan]) -> Vec<IoValue> {
    nodes
        .iter()
        .map(|node| {
            record("node", vec![
                record("id", vec![string(&node.node_id)]),
                record("state-root", vec![string(&node.state_root_handle)]),
                record("transport", vec![string(&node.transport_handle)]),
            ])
        })
        .collect()
}

fn role(node_id: &str, role_name: &str, membership: &str) -> MultinodeTopologyRole {
    MultinodeTopologyRole {
        node_id: node_id.to_string(),
        role: role_name.to_string(),
        membership: membership.to_string(),
    }
}

fn link(from: &str, to: &str, topic: &str) -> MultinodeTopologyLink {
    MultinodeTopologyLink {
        from: from.to_string(),
        to: to.to_string(),
        topic: topic.to_string(),
    }
}

fn live_transport_refs(input: &LiveTransportVmEvidenceInput) -> Vec<String> {
    vec![
        input.ticket_ref.clone(),
        input.peer_admission_ref.clone(),
        input.authority_ref.clone(),
        input.send_ref.clone(),
        input.receive_ref.clone(),
        input.ingress_ref.clone(),
        input.queue_ref.clone(),
        input.dispatch_ref.clone(),
        input.reconcile_ref.clone(),
        input.ack_ref.clone(),
        input.protocol_gate_ref.clone(),
    ]
}

fn collect_required_text_diagnostic(label: &str, value: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    if value.trim().is_empty() {
        push_diagnostic(diagnostics, format!("missing-{label}"))?;
    }
    Ok(())
}

fn collect_invalid_ref_diagnostics(label: &str, refs: &[String], diagnostics: &mut Vec<String>) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_MULTINODE_ITEMS, label)?;
    for reference in refs {
        if crate::preserves_rail::validate_content_ref(reference).is_err() {
            push_diagnostic(diagnostics, format!("invalid-{label}-ref"))?;
        }
    }
    Ok(())
}

fn collect_invalid_optional_ref_diagnostics(
    label: &str,
    reference: Option<&str>,
    diagnostics: &mut Vec<String>,
) -> Result<()> {
    if let Some(reference) = reference {
        if crate::preserves_rail::validate_content_ref(reference).is_err() {
            push_diagnostic(diagnostics, format!("invalid-{label}-ref"))?;
        }
    }
    Ok(())
}

fn push_if(diagnostics: &mut Vec<String>, condition: bool, diagnostic: &'static str) -> Result<()> {
    if condition {
        push_diagnostic(diagnostics, diagnostic.to_string())?;
    }
    Ok(())
}

fn push_diagnostic(diagnostics: &mut Vec<String>, diagnostic: String) -> Result<()> {
    if diagnostics.len() >= MAX_MULTINODE_ITEMS {
        return Err(MoltenError::invalid_harness("multinode diagnostics exceeded bound"));
    }
    diagnostics.push(diagnostic);
    Ok(())
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds bound {maximum}")))
    }
}

fn decision_from_diagnostics(diagnostics: &[String]) -> &'static str {
    if diagnostics.is_empty() {
        PASS_DECISION
    } else {
        DENY_DECISION
    }
}

fn status(condition: bool) -> &'static str {
    if condition { PASS_DECISION } else { DENY_DECISION }
}

fn content_ref_from_text(value: &str) -> String {
    crate::preserves_rail::content_ref_from_bytes(value.as_bytes())
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    match reference {
        Some(reference) => record("some", vec![string(reference)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    crate::preserves_rail::checks_value(checks)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMULATION_MAX_TICKS: u64 = 32;
    const FAULT_START_TICK: u64 = 1;
    const FAULT_DURATION_TICKS: u64 = 2;

    fn local_ref(label: &str) -> String {
        content_ref_from_text(label)
    }

    fn execution_profiles() -> Vec<crate::distributed_core::DistributedCiProfile> {
        crate::distributed_core::default_distributed_ci_profiles()
    }

    fn protocol_profile() -> crate::distributed_core::DistributedCiProfile {
        execution_profiles().into_iter().find(|profile| profile.id == "protocol").expect("protocol profile")
    }

    fn valid_fixture() -> MultinodeScenarioFixture {
        let profile = protocol_profile();
        MultinodeScenarioFixture {
            scenario_id: "protocol-pairwise-smoke".to_string(),
            purpose: "review pairwise protocol metadata".to_string(),
            evidence_scope: "simulated distributed protocol evidence".to_string(),
            topology_profile_id: PROFILE_PAIRWISE_TRANSPORT.to_string(),
            execution_profile_id: profile.id,
            command_surface: profile.command,
            expected_artifact_kinds: profile.expected_artifact_kinds,
            topology_ref: local_ref("topology"),
            seed_ref: local_ref("seed"),
            fault_plan_ref: local_ref("fault-plan"),
            receipt_refs: vec![local_ref("receipt")],
            variance_refs: vec![local_ref("variance:none")],
            diagnostic_log_refs: vec![local_ref("log")],
            unavailable_policy: "unavailable-is-deny".to_string(),
            unsupported_claims_pass: false,
            caveats: vec!["fixture evidence is review evidence only".to_string()],
        }
    }

    #[test]
    fn declarative_scenario_fixture_derives_stable_metadata() {
        // r[verify molten.testing.multinode.declarative_scenario_fixtures]
        let fixture = valid_fixture();
        let topology_profiles = default_multinode_topology_profiles();
        let first = derive_multinode_scenario_metadata(&fixture, &execution_profiles(), &topology_profiles)
            .expect("first metadata");
        let second = derive_multinode_scenario_metadata(&fixture, &execution_profiles(), &topology_profiles)
            .expect("second metadata");
        let rendered = crate::preserves_rail::to_text(&first.value).expect("render metadata");

        assert_eq!(first.decision, PASS_DECISION);
        assert_eq!(first.metadata_ref, second.metadata_ref);
        assert_eq!(first.fixture_ref, second.fixture_ref);
        assert!(first.diagnostics.is_empty());
        assert!(rendered.contains("multinode-scenario-metadata-v1"));
        assert!(rendered.contains("profile"));
    }

    #[test]
    fn declarative_scenario_fixture_validation_denies_bad_bindings() {
        // r[verify molten.testing.multinode.scenario_fixture_validation]
        let mut fixture = valid_fixture();
        fixture.command_surface = "cargo test wrong-profile".to_string();
        fixture.receipt_refs = Vec::new();
        fixture.variance_refs = Vec::new();
        fixture.unsupported_claims_pass = true;
        fixture.expected_artifact_kinds = vec!["wrong-kind".to_string()];
        let metadata =
            derive_multinode_scenario_metadata(&fixture, &execution_profiles(), &default_multinode_topology_profiles())
                .expect("denied metadata");

        assert_eq!(metadata.decision, DENY_DECISION);
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-command-profile-mismatch"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-artifact-kind-mismatch"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-missing-receipt-ref"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-missing-variance-ref"));
        assert!(metadata.diagnostics.iter().any(|item| item == "fixture-unsupported-pass-claim"));
    }

    #[test]
    fn topology_profile_matrix_binds_profiles_and_membership_scope() {
        // r[verify molten.testing.multinode.topology_profile_matrix]
        let profiles = default_multinode_topology_profiles();
        let matrix = build_multinode_topology_matrix(&profiles).expect("topology matrix");
        let rendered = crate::preserves_rail::to_text(&matrix.value).expect("render matrix");

        assert_eq!(matrix.decision, PASS_DECISION);
        assert_eq!(matrix.profile_refs.len(), REQUIRED_DEFAULT_TOPOLOGY_PROFILE_COUNT);
        assert!(matrix.diagnostics.is_empty());
        assert!(rendered.contains(PROFILE_CONTROL_QUORUM));
        assert!(rendered.contains("role-membership-explicit"));
    }

    #[test]
    fn topology_membership_negatives_deny_role_confusion() {
        // r[verify molten.testing.multinode.role_membership_negatives]
        let profile = default_multinode_topology_profiles()
            .into_iter()
            .find(|profile| profile.id == PROFILE_SUBSCRIBER_PEER)
            .expect("subscriber profile");
        let claim = TopologyMembershipClaim {
            profile_id: profile.id.clone(),
            topology_ref: local_ref("topology-a"),
            scenario_topology_ref: local_ref("topology-b"),
            node_roles: vec![role("subscriber", ROLE_VOTER, MEMBERSHIP_VOTER)],
            quorum_ref: None,
            transport_only_authority_claim: true,
            caveats: vec!["negative fixture".to_string()],
        };
        let gate = evaluate_topology_membership_claim(&profile, &claim).expect("membership gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "wrong-topology"));
        assert!(gate.diagnostics.iter().any(|item| item == "subscriber-promoted-to-voter:subscriber"));
        assert!(gate.diagnostics.iter().any(|item| item == "transport-only-authority-claim"));
    }

    fn node_summary(node: &str, queue_ref: String, commit_ref: String) -> MultinodeNodeSummary {
        MultinodeNodeSummary {
            node_id: node.to_string(),
            topology_ref: local_ref("topology"),
            scenario_fixture_ref: local_ref("fixture"),
            receipt_refs: vec![local_ref("receipt:workflow"), local_ref(&format!("receipt:{node}"))],
            queue_ref,
            ledger_ref: local_ref("ledger"),
            dispatch_ref: local_ref("dispatch"),
            ack_ref: local_ref("ack"),
            protocol_ref: local_ref("protocol"),
            semantic_commits: vec![SemanticCommitEvidence {
                operation_id: "op-1".to_string(),
                commit_ref,
            }],
            diagnostic_log_refs: vec![local_ref(&format!("log:{node}"))],
        }
    }

    fn reconciliation_input() -> ReconciliationInput {
        let shared_queue = local_ref("queue");
        let shared_commit = local_ref("commit:op-1");
        ReconciliationInput {
            topology_ref: local_ref("topology"),
            scenario_fixture_ref: local_ref("fixture"),
            required_receipt_refs: vec![local_ref("receipt:workflow")],
            node_summaries: vec![
                node_summary("node-a", shared_queue.clone(), shared_commit.clone()),
                node_summary("node-b", shared_queue.clone(), shared_commit),
            ],
            equality_classes: vec![ReconciliationEqualityClass {
                name: "queue".to_string(),
                refs: vec![shared_queue.clone(), shared_queue],
                variance_ref: None,
            }],
            allowed_variance_refs: vec![local_ref("variance:clock")],
            caveats: vec!["reconciliation evidence is scoped".to_string()],
        }
    }

    #[test]
    fn reconciliation_gate_passes_converged_nodes_and_declared_variance() {
        // r[verify molten.testing.multinode.cross_node_reconciliation_gate]
        let mut input = reconciliation_input();
        input.equality_classes.push(ReconciliationEqualityClass {
            name: "runtime-log".to_string(),
            refs: vec![local_ref("log-a"), local_ref("log-b")],
            variance_ref: Some(local_ref("variance:clock")),
        });
        let gate = evaluate_reconciliation(&input).expect("reconciliation gate");
        let rendered = crate::preserves_rail::to_text(&gate.value).expect("render reconciliation");

        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
        assert!(rendered.contains("multinode-reconciliation-gate-v1"));
        assert!(rendered.contains("allowed-variance"));
    }

    #[test]
    fn reconciliation_gate_denies_divergence_and_duplicate_commit() {
        // r[verify molten.testing.multinode.reconciliation_deny_drift]
        let mut input = reconciliation_input();
        input.node_summaries[1].queue_ref = local_ref("queue-divergent");
        input.node_summaries[1].semantic_commits[0].commit_ref = local_ref("commit:op-1-duplicate");
        input.equality_classes[0].refs = vec![local_ref("queue"), local_ref("queue-divergent")];
        let gate = evaluate_reconciliation(&input).expect("reconciliation gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "divergent-ref-class:queue"));
        assert!(gate.diagnostics.iter().any(|item| item == "duplicate-semantic-commit:op-1"));
    }

    fn local_plan_input() -> LocalMultiprocessPlanInput {
        LocalMultiprocessPlanInput {
            fixture_ref: local_ref("fixture"),
            nodes: vec![
                LocalProcessNodePlan {
                    node_id: "node-a".to_string(),
                    state_root_handle: "state-a".to_string(),
                    transport_handle: "transport-a".to_string(),
                },
                LocalProcessNodePlan {
                    node_id: "node-b".to_string(),
                    state_root_handle: "state-b".to_string(),
                    transport_handle: "transport-b".to_string(),
                },
            ],
            command_plan_ref: local_ref("command-plan"),
            expected_receipt_refs: vec![local_ref("startup"), local_ref("workflow"), local_ref("cleanup")],
            cleanup_policy: CLEANUP_POLICY_REQUIRED.to_string(),
            caveats: vec!["local integration evidence only".to_string()],
        }
    }

    #[test]
    fn local_multiprocess_plan_and_run_bind_isolated_process_evidence() {
        // r[verify molten.testing.multinode.local_multiprocess_harness]
        let plan = build_local_multiprocess_plan(&local_plan_input()).expect("local plan");
        let run = build_local_multiprocess_run_receipt(&LocalMultiprocessRunInput {
            plan_ref: plan.plan_ref.clone(),
            startup_refs: vec![local_ref("startup-a"), local_ref("startup-b")],
            workflow_refs: vec![local_ref("workflow")],
            shutdown_refs: vec![local_ref("shutdown-a"), local_ref("shutdown-b")],
            cleanup_refs: vec![local_ref("cleanup")],
            diagnostics: Vec::new(),
            caveats: vec!["local multiprocess evidence is not VM evidence".to_string()],
        })
        .expect("local run");

        assert_eq!(plan.decision, PASS_DECISION);
        assert_eq!(run.decision, PASS_DECISION);
        assert!(run.receipt_ref.starts_with("blake3:"));
    }

    #[test]
    fn local_multiprocess_plan_denies_collisions_and_missing_cleanup() {
        // r[verify molten.testing.multinode.process_isolation_cleanup]
        let mut input = local_plan_input();
        input.nodes[1].state_root_handle = input.nodes[0].state_root_handle.clone();
        input.nodes[1].transport_handle = input.nodes[0].transport_handle.clone();
        input.cleanup_policy = String::new();
        let plan = build_local_multiprocess_plan(&input).expect("denied local plan");

        assert_eq!(plan.decision, DENY_DECISION);
        assert!(plan.diagnostics.iter().any(|item| item.contains("state-root-collision")));
        assert!(plan.diagnostics.iter().any(|item| item.contains("transport-collision")));
        assert!(plan.diagnostics.iter().any(|item| item == "local-plan-missing-cleanup-policy"));
    }

    fn distributed_topology() -> crate::distributed_core::DistributedTopology {
        crate::distributed_core::DistributedTopology {
            peers: vec![
                crate::distributed_core::DistributedPeer {
                    id: "peer-a".to_string(),
                    roles: vec!["sender".to_string()],
                },
                crate::distributed_core::DistributedPeer {
                    id: "peer-b".to_string(),
                    roles: vec!["receiver".to_string()],
                },
            ],
            channels: vec![crate::distributed_core::DistributedChannel {
                id: "a-to-b".to_string(),
                from_peer: "peer-a".to_string(),
                to_peer: "peer-b".to_string(),
                topic: "node-control".to_string(),
            }],
            caveats: vec!["generated simulation evidence only".to_string()],
        }
    }

    fn generated_command(operation_id: &str) -> crate::distributed_core::SimulationCommand {
        crate::distributed_core::SimulationCommand {
            operation_id: operation_id.to_string(),
            from_peer: "peer-a".to_string(),
            to_peer: "peer-b".to_string(),
            payload_ref: local_ref(&format!("payload:{operation_id}")),
            commit_ref: local_ref(&format!("commit:{operation_id}")),
            authority_ref: Some(local_ref("authority")),
            policy_ref: Some(local_ref("policy")),
            resource_ref: Some(local_ref("resource")),
            transport_ref: Some(local_ref("transport")),
            requires_authority: true,
            requires_quorum: false,
        }
    }

    fn generated_case_with_fault(
        case_id: &str,
        invariant: &str,
        fault_kind: &str,
        operation_id: &str,
    ) -> GeneratedDistributedCase {
        GeneratedDistributedCase {
            case_id: case_id.to_string(),
            invariant_name: invariant.to_string(),
            simulation: crate::distributed_core::DistributedSimulationInput {
                topology: distributed_topology(),
                scheduler: crate::distributed_core::SchedulerProfile {
                    id: "generated-round-robin".to_string(),
                    policy: "deterministic-virtual-clock".to_string(),
                    max_ticks: SIMULATION_MAX_TICKS,
                },
                seed: crate::distributed_core::SimulationSeed {
                    id: format!("seed:{case_id}"),
                    entropy_ref: local_ref(&format!("seed:{case_id}")),
                },
                fault_plan: crate::distributed_core::FaultPlan {
                    events: vec![crate::distributed_core::FaultEvent {
                        kind: fault_kind.to_string(),
                        target_kind: "operation".to_string(),
                        target: operation_id.to_string(),
                        operation_id: Some(operation_id.to_string()),
                        start_tick: FAULT_START_TICK,
                        duration_ticks: FAULT_DURATION_TICKS,
                        diagnostic: format!("generated:{fault_kind}"),
                    }],
                    caveats: vec!["generated bounded fault plan".to_string()],
                },
                source_ref: local_ref("source"),
                test_binary_ref: local_ref("test-binary"),
                commands: vec![generated_command(operation_id)],
                child_workflow_refs: vec![local_ref("child")],
                allowed_variance_refs: vec![local_ref("variance:none")],
            },
        }
    }

    #[test]
    fn generated_distributed_cases_replay_benign_interleavings_stably() {
        // r[verify molten.testing.distributed_simulation.generated_fault_interleavings]
        let cases = [
            generated_case_with_fault("delay", "deterministic replay", "delay", "op-delay"),
            generated_case_with_fault("restart", "restart stability", "restart", "op-restart"),
            generated_case_with_fault("duplicate", "idempotent duplicate", "duplicate", "op-duplicate"),
        ];

        for case in cases {
            let repro = run_generated_distributed_case(&case).expect("generated repro");
            assert_eq!(repro.decision, PASS_DECISION);
            assert_eq!(repro.run_ref, repro.replay_run_ref);
            assert!(repro.repro_ref.starts_with("blake3:"));
        }
    }

    #[test]
    fn generated_distributed_cases_preserve_deny_repro_seed() {
        // r[verify molten.testing.distributed_simulation.generated_fault_interleavings]
        // r[verify molten.testing.distributed_simulation.generated_repro_seed]
        let mut case =
            generated_case_with_fault("missing-authority", "missing authority denies", "stale-evidence", "op-deny");
        case.simulation.commands[0].authority_ref = None;
        let repro = run_generated_distributed_case(&case).expect("generated deny repro");
        let rendered = crate::preserves_rail::to_text(&repro.value).expect("render repro");

        assert_eq!(repro.decision, PASS_DECISION);
        assert_eq!(repro.run_ref, repro.replay_run_ref);
        assert!(rendered.contains("generated-distributed-repro-v1"));
        assert!(rendered.contains(DIAGNOSTIC_ONLY));
    }

    fn repro_input() -> MultinodeFailureReproBundleInput {
        MultinodeFailureReproBundleInput {
            scenario_fixture_ref: local_ref("fixture"),
            topology_ref: local_ref("topology"),
            scheduler_ref: local_ref("scheduler"),
            seed_ref: local_ref("seed"),
            fault_plan_ref: local_ref("fault-plan"),
            command_refs: vec![local_ref("command")],
            node_summary_refs: vec![local_ref("node-summary")],
            receipt_refs: vec![local_ref("receipt")],
            diagnostic_refs: vec![local_ref("diagnostic")],
            log_refs: vec![local_ref("redacted-log")],
            redaction_policy_ref: local_ref("redaction-policy"),
            replay_status: REPLAYABLE_SIMULATION.to_string(),
            diagnostic_only: true,
            sealed: true,
            private_attachment_refs: Vec::new(),
            reveal_receipt_refs: Vec::new(),
            claimed_payload_ref: None,
            caveats: vec!["failure repro is not pass evidence".to_string()],
        }
    }

    #[test]
    fn multinode_failure_repro_bundle_verifies_sealed_simulation_payload() {
        // r[verify molten.testing.multinode.failure_repro_bundle]
        let input = repro_input();
        let bundle = build_multinode_failure_repro_bundle(&input).expect("bundle");
        let verification = verify_multinode_failure_repro_bundle(&input).expect("verification");

        assert_eq!(verification.decision, PASS_DECISION);
        assert_eq!(verification.payload_ref, bundle.payload_ref);
        assert!(verification.diagnostics.is_empty());
    }

    #[test]
    fn multinode_failure_repro_bundle_privacy_and_pass_gate_fail_closed() {
        // r[verify molten.testing.multinode.failure_repro_privacy_and_replay]
        let mut input = repro_input();
        let payload_ref = canonical_hash(&failure_repro_payload_value(&input).expect("payload")).expect("payload ref");
        input.claimed_payload_ref = Some(local_ref("tampered-payload"));
        input.private_attachment_refs = vec![local_ref("private-log")];
        input.replay_status = NON_REPLAYABLE_VM.to_string();
        let verification = verify_multinode_failure_repro_bundle(&input).expect("verification");
        let valid_verification = verify_multinode_failure_repro_bundle(&repro_input()).expect("valid verification");
        let pass_gate = gate_multinode_failure_repro_as_pass(&valid_verification, true).expect("pass gate");

        assert_ne!(verification.payload_ref, payload_ref);
        assert_eq!(verification.decision, DENY_DECISION);
        assert!(verification.diagnostics.iter().any(|item| item == "failure-repro-seal-mismatch"));
        assert!(verification.diagnostics.iter().any(|item| item == "failure-repro-private-without-reveal"));
        assert_eq!(pass_gate.decision, DENY_DECISION);
        assert!(pass_gate.diagnostics.iter().any(|item| item == "diagnostic-bundle-cannot-satisfy-pass"));
    }

    fn live_transport_input() -> LiveTransportVmEvidenceInput {
        LiveTransportVmEvidenceInput {
            expected_sender_node: "sender".to_string(),
            actual_sender_node: "sender".to_string(),
            expected_receiver_node: "receiver".to_string(),
            actual_receiver_node: "receiver".to_string(),
            expected_peer: "peer:operator".to_string(),
            actual_peer: "peer:operator".to_string(),
            topic: "node-control".to_string(),
            operation_id: "blake3:operation".to_string(),
            ticket_ref: local_ref("ticket"),
            peer_admission_ref: local_ref("peer-admission"),
            authority_ref: local_ref("authority"),
            send_ref: local_ref("send"),
            receive_ref: local_ref("receive"),
            ingress_ref: local_ref("ingress"),
            queue_ref: local_ref("queue"),
            dispatch_ref: local_ref("dispatch"),
            reconcile_ref: local_ref("reconcile"),
            ack_ref: local_ref("ack"),
            protocol_gate_ref: local_ref("protocol-gate"),
            log_refs: vec![local_ref("vm-log")],
            caveats: vec!["live VM transport evidence is topology-scoped".to_string()],
        }
    }

    #[test]
    fn live_transport_vm_gate_accepts_complete_receipt_chain() {
        // r[verify molten.testing.nixos_vm.cross_node_live_transport]
        let gate = evaluate_live_transport_vm_gate(&live_transport_input()).expect("live transport gate");

        assert_eq!(gate.decision, PASS_DECISION);
        assert!(gate.diagnostics.is_empty());
    }

    #[test]
    fn live_transport_vm_gate_denies_wrong_peer_and_log_only_receive() {
        // r[verify molten.testing.nixos_vm.live_transport_negative_gate]
        let mut input = live_transport_input();
        input.actual_peer = "peer:wrong".to_string();
        input.receive_ref = String::new();
        input.protocol_gate_ref = String::new();
        let gate = evaluate_live_transport_vm_gate(&input).expect("live transport gate");

        assert_eq!(gate.decision, DENY_DECISION);
        assert!(gate.diagnostics.iter().any(|item| item == "live-transport-peer-mismatch"));
        assert!(gate.diagnostics.iter().any(|item| item == "live-transport-missing-receive"));
        assert!(gate.diagnostics.iter().any(|item| item == "live-transport-missing-protocol-gate"));
    }

    fn supported_fault_case(kind: &str) -> VmFaultSupportCase {
        VmFaultSupportCase {
            fault_kind: kind.to_string(),
            required_capability: "test-driver-control".to_string(),
            target: "node-a".to_string(),
            command_profile: "nixos-vm-multinode".to_string(),
            expected_outcome: PASS_DECISION.to_string(),
            host_support: SUPPORTED.to_string(),
            preflight_refs: vec![local_ref("preflight")],
            injection_refs: vec![local_ref("injection")],
            child_refs: vec![local_ref("child")],
            post_fault_refs: vec![local_ref("post")],
            diagnostic_refs: vec![local_ref("diagnostic")],
            caveats: vec!["VM fault evidence is platform-scoped".to_string()],
        }
    }

    #[test]
    fn executable_vm_fault_support_matrix_records_supported_and_unavailable_cases() {
        // r[verify molten.testing.nixos_vm.executable_fault_support_matrix]
        let mut unavailable = supported_fault_case("bounded-disk-pressure");
        unavailable.expected_outcome = UNAVAILABLE.to_string();
        unavailable.host_support = UNAVAILABLE.to_string();
        unavailable.injection_refs = Vec::new();
        unavailable.child_refs = Vec::new();
        let matrix = build_vm_fault_support_matrix(&[
            supported_fault_case("network-partition"),
            supported_fault_case("crash-restart"),
            unavailable,
        ])
        .expect("fault matrix");

        assert_eq!(matrix.decision, PASS_DECISION);
        assert!(matrix.diagnostics.is_empty());
    }

    #[test]
    fn executable_vm_fault_support_matrix_denies_invalid_claims() {
        // r[verify molten.testing.nixos_vm.executable_fault_validation_negatives]
        let mut unsupported_pass = supported_fault_case("unsupported-host-feature");
        unsupported_pass.host_support = UNAVAILABLE.to_string();
        unsupported_pass.injection_refs = Vec::new();
        unsupported_pass.child_refs = Vec::new();
        unsupported_pass.diagnostic_refs = Vec::new();
        let matrix = build_vm_fault_support_matrix(&[unsupported_pass]).expect("fault matrix");

        assert_eq!(matrix.decision, DENY_DECISION);
        assert!(matrix.diagnostics.iter().any(|item| item == "vm-fault-unsupported-pass:unsupported-host-feature"));
        assert!(matrix.diagnostics.iter().any(|item| item == "vm-fault-missing-injection:unsupported-host-feature"));
        assert!(matrix.diagnostics.iter().any(|item| item == "vm-fault-missing-child:unsupported-host-feature"));
        assert!(
            matrix
                .diagnostics
                .iter()
                .any(|item| item == "vm-fault-unavailable-missing-diagnostic:unsupported-host-feature")
        );
    }
}
