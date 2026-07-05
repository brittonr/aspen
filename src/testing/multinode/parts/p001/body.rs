#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeNodeQuorumEvidenceInput {
    pub topology_ref: String,
    pub scenario_fixture_ref: String,
    pub membership_gate_ref: String,
    pub reconciliation_gate_ref: String,
    pub node_summary_refs: Vec<String>,
    pub quorum_refs: Vec<String>,
    pub restarting_member: String,
    pub duplicate_semantic_commit: bool,
    pub log_only_quorum: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeNodeQuorumGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmScenarioGateInput {
    pub scenario_metadata_ref: String,
    pub topology_membership_gate_ref: String,
    pub reconciliation_gate_ref: String,
    pub live_transport_gate_ref: Option<String>,
    pub expected_artifact_kinds: Vec<String>,
    pub observed_artifact_kinds: Vec<String>,
    pub unsupported_pass_claim: bool,
    pub log_only_reconciliation: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmScenarioGate {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub gate_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmFailureReproExportInput {
    pub scenario_fixture_ref: String,
    pub topology_ref: String,
    pub scheduler_ref: String,
    pub seed_ref: String,
    pub fault_plan_ref: String,
    pub command_refs: Vec<String>,
    pub node_summary_refs: Vec<String>,
    pub child_receipt_refs: Vec<String>,
    pub validation_refs: Vec<String>,
    pub diagnostic_log_refs: Vec<String>,
    pub redaction_policy_ref: String,
    pub private_attachment_refs: Vec<String>,
    pub reveal_receipt_refs: Vec<String>,
    pub unavailable_host_support: bool,
    pub denied_or_failed_validation: bool,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmFailureReproExport {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub bundle_ref: String,
    pub verification_ref: String,
    pub export_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedDistributedCase {
    pub case_id: String,
    pub invariant_name: String,
    pub simulation: crate::distributed_core::SimulationInput,
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
pub struct FailureReproBundleInput {
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
pub struct FailureReproBundle {
    pub payload_ref: String,
    pub bundle_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureReproVerification {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub payload_ref: String,
    pub verification_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureReproPassGate {
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

pub fn default_topology_profiles() -> Vec<TopologyProfile> {
    vec![
        pairwise_transport_profile(),
        control_quorum_profile(),
        restart_rejoin_profile(),
        subscriber_peer_profile(),
        three_node_quorum_profile(),
        wrong_topology_profile(),
    ]
}

fn pairwise_transport_profile() -> TopologyProfile {
    TopologyProfile {
        id: PROFILE_PAIRWISE_TRANSPORT.to_string(),
        roles: vec![
            role("sender", ROLE_SENDER, MEMBERSHIP_TRANSPORT_ONLY),
            role("receiver", ROLE_RECEIVER, MEMBERSHIP_TRANSPORT_ONLY),
        ],
        allowed_links: vec![link("sender", "receiver", "node-control")],
        evidence_scope: "live or simulated sender-to-receiver transport handoff".to_string(),
        required_receipt_kinds: vec!["send".to_string(), "receive".to_string(), "ingress".to_string()],
        caveats: vec!["transport evidence does not grant authority".to_string()],
    }
}

fn control_quorum_profile() -> TopologyProfile {
    TopologyProfile {
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
    }
}

fn restart_rejoin_profile() -> TopologyProfile {
    TopologyProfile {
        id: PROFILE_RESTART_REJOIN.to_string(),
        roles: vec![
            role("stable", ROLE_VOTER, MEMBERSHIP_VOTER),
            role("restarting", ROLE_RESTARTING_MEMBER, MEMBERSHIP_VOTER),
        ],
        allowed_links: vec![link("stable", "restarting", "restart-rejoin")],
        evidence_scope: "state-root restart and rejoin evidence".to_string(),
        required_receipt_kinds: vec!["startup".to_string(), "shutdown".to_string(), "rejoin".to_string()],
        caveats: vec!["restart evidence does not imply WAN durability".to_string()],
    }
}

