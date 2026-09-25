const CONSENSUS_PLACEMENT_REPORT_SCHEMA: &str = "molten.consensus.placement-report.v1";
const CONSENSUS_NON_CLAIM_RECEIPT_SCHEMA: &str = "molten.consensus.non-claim-receipt.v1";
const CONSENSUS_SIMULATION_RECEIPT_SCHEMA: &str = "molten.consensus.simulation-receipt.v1";
const CONSENSUS_PLACEMENT_FIELD_COUNT: usize = 13;
const CONSENSUS_NON_CLAIM_FIELD_COUNT: usize = 8;
const CONSENSUS_SIMULATION_FIELD_COUNT: usize = 13;
const MAJORITY_QUORUM_DIVISOR: usize = 2;
const MAJORITY_QUORUM_OFFSET: usize = 1;
const SCENARIO_MAJORITY_PROGRESS: &str = "majority-progress";
const SCENARIO_MINORITY_DENIAL: &str = "minority-denial";
const SCENARIO_STALE_READ_CLASSIFICATION: &str = "stale-read-classification";
const SCENARIO_LEADERLESS_NON_LEADER_PROGRESS: &str = "leaderless-non-leader-progress";
const SCENARIO_LEADERLESS_MISSING_EVIDENCE: &str = "leaderless-missing-evidence";
const SCENARIO_CONCURRENT_PROPOSAL_RESOLUTION: &str = "concurrent-proposal-resolution";
const SCENARIO_UNSAFE_PLACEMENT: &str = "unsafe-placement";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusPlacementInput {
    pub group_id: String,
    pub candidate_members: Vec<String>,
    pub admitted_members: Vec<String>,
    pub fault_domain_refs: Vec<String>,
    pub fault_domain_policy_ref: String,
    pub membership_refs: Vec<String>,
    pub placement_policy_refs: Vec<String>,
    pub majority_reachable: bool,
    pub latency_diagnostics: Vec<String>,
    pub denied_candidates: Vec<String>,
    pub refresh_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusPlacementReport {
    pub report_ref: String,
    pub decision: String,
    pub group_id: String,
    pub admitted_members: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusClaimBoundaryInput {
    pub group_ref: String,
    pub claim: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusClaimBoundaryReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub claim: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusSimulationInput {
    pub scenario: String,
    pub algorithm_profile: String,
    pub topology_ref: String,
    pub membership_refs: Vec<String>,
    pub fault_plan_ref: String,
    pub operation_ids: Vec<String>,
    pub connected_replicas: usize,
    pub proposer_ref: Option<String>,
    pub required_evidence_refs: Vec<String>,
    pub placement_ref: Option<String>,
    pub local_state_fresh: bool,
    pub requested_read_consistency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusSimulationReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub scenario: String,
    pub final_state_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

// r[impl molten.consensus.replica_placement_evidence]
pub fn consensus_placement_report(input: &ConsensusPlacementInput) -> Result<ConsensusPlacementReport> {
    validate_group_id(&input.group_id)?;
    validate_refs(&input.candidate_members, "consensus placement candidate member ref")?;
    validate_refs(&input.admitted_members, "consensus placement admitted member ref")?;
    validate_refs(&input.fault_domain_refs, "consensus placement fault domain ref")?;
    require_ref(&input.fault_domain_policy_ref, "consensus placement fault-domain policy ref")?;
    validate_refs(&input.membership_refs, "consensus placement membership ref")?;
    validate_refs(&input.placement_policy_refs, "consensus placement policy ref")?;
    validate_refs(&input.denied_candidates, "consensus placement denied candidate ref")?;
    validate_refs(&input.refresh_refs, "consensus placement refresh ref")?;
    validate_diagnostic_strings(&input.latency_diagnostics, "consensus placement latency diagnostics")?;
    let diagnostics = placement_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = consensus_placement_report_value(input, decision, &diagnostics)?;
    Ok(ConsensusPlacementReport {
        report_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        group_id: input.group_id.clone(),
        admitted_members: input.admitted_members.clone(),
        diagnostics,
        value,
    })
}

pub fn parse_consensus_placement_report(value: &IoValue) -> Result<ConsensusPlacementReport> {
    let fields = value
        .collect_simple_record("consensus-placement-report-v1", Some(CONSENSUS_PLACEMENT_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-placement-report-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_PLACEMENT_REPORT_SCHEMA, "consensus placement schema")?;
    let decision = record_string(&fields[1], "decision")?;
    let group_id = record_string(&fields[2], "group")?;
    let admitted_members = parse_ref_sequence(&fields[4], "admitted-members")?;
    let diagnostics = parse_string_sequence(&fields[10], "diagnostics")?;
    require_check(&parse_checks(&fields[12])?, "placement-evidence-bound", "consensus placement report")?;
    Ok(ConsensusPlacementReport {
        report_ref: canonical_hash(value)?,
        decision,
        group_id,
        admitted_members,
        diagnostics,
        value: value.clone(),
    })
}

// r[impl molten.consensus.non_claim_boundaries]
pub fn consensus_claim_boundary_receipt(
    input: &ConsensusClaimBoundaryInput,
) -> Result<ConsensusClaimBoundaryReceipt> {
    require_ref(&input.group_ref, "consensus claim group ref")?;
    validate_non_empty(&input.claim, "consensus claim")?;
    validate_refs(&input.evidence_refs, "consensus claim evidence ref")?;
    let diagnostics = claim_boundary_diagnostics(&input.claim);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = record("consensus-non-claim-receipt-v1", vec![
        string(CONSENSUS_NON_CLAIM_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("group", vec![string(&input.group_ref)]),
        record("claim", vec![string(&input.claim)]),
        record("evidence", vec![strings_sequence(&input.evidence_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        record("boundary", vec![string("consensus-evidence-non-claim")]),
        checks_value(&[
            ("claim-boundary-evaluated", "pass"),
            ("unsupported-claim-denied", decision),
        ]),
    ]);
    Ok(ConsensusClaimBoundaryReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        claim: input.claim.clone(),
        diagnostics,
        value,
    })
}

pub fn parse_consensus_claim_boundary_receipt(value: &IoValue) -> Result<ConsensusClaimBoundaryReceipt> {
    let fields = value
        .collect_simple_record("consensus-non-claim-receipt-v1", Some(CONSENSUS_NON_CLAIM_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-non-claim-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_NON_CLAIM_RECEIPT_SCHEMA, "consensus non-claim schema")?;
    require_check(&parse_checks(&fields[7])?, "claim-boundary-evaluated", "consensus non-claim receipt")?;
    Ok(ConsensusClaimBoundaryReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        claim: record_string(&fields[3], "claim")?,
        diagnostics: parse_string_sequence(&fields[5], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.testing.consensus_fault_matrix]
// r[impl molten.testing.leaderless_experimental_fixtures]
// r[impl molten.testing.consensus_placement_fixtures]
pub fn run_consensus_simulation(input: &ConsensusSimulationInput) -> Result<ConsensusSimulationReceipt> {
    validate_simulation_input(input)?;
    let diagnostics = simulation_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let final_state_ref = (decision == "pass").then(|| simulation_final_state_ref(input)).transpose()?;
    let value = consensus_simulation_receipt_value(input, decision, final_state_ref.as_deref(), &diagnostics)?;
    Ok(ConsensusSimulationReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        scenario: input.scenario.clone(),
        final_state_ref,
        diagnostics,
        value,
    })
}

pub fn parse_consensus_simulation_receipt(value: &IoValue) -> Result<ConsensusSimulationReceipt> {
    let fields = value
        .collect_simple_record("consensus-simulation-receipt-v1", Some(CONSENSUS_SIMULATION_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-simulation-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_SIMULATION_RECEIPT_SCHEMA, "consensus simulation schema")?;
    require_check(&parse_checks(&fields[12])?, "deterministic-scheduler", "consensus simulation receipt")?;
    Ok(ConsensusSimulationReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        scenario: record_string(&fields[2], "scenario")?,
        final_state_ref: record_optional_ref(&fields[10], "final-state")?,
        diagnostics: parse_string_sequence(&fields[11], "diagnostics")?,
        value: value.clone(),
    })
}

fn consensus_placement_report_value(
    input: &ConsensusPlacementInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("consensus-placement-report-v1", vec![
        string(CONSENSUS_PLACEMENT_REPORT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("group", vec![string(&input.group_id)]),
        record("candidate-members", vec![strings_sequence(&input.candidate_members)]),
        record("admitted-members", vec![strings_sequence(&input.admitted_members)]),
        record("fault-domains", vec![strings_sequence(&input.fault_domain_refs)]),
        record("fault-domain-policy", vec![string(&input.fault_domain_policy_ref)]),
        record("membership", vec![strings_sequence(&input.membership_refs)]),
        record("placement-policy", vec![strings_sequence(&input.placement_policy_refs)]),
        record("majority-reachable", vec![bool_value(input.majority_reachable)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        record("refresh", vec![strings_sequence(&input.refresh_refs)]),
        checks_value(&[
            ("placement-evidence-bound", "pass"),
            ("fault-domain-policy-bound", if input.fault_domain_policy_ref.is_empty() { "fail" } else { "pass" }),
            ("majority-reachability", decision),
        ]),
    ]))
}

fn placement_diagnostics(input: &ConsensusPlacementInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.candidate_members.is_empty() {
        diagnostics.push("consensus placement requires candidate members".to_string());
    }
    if input.admitted_members.is_empty() {
        diagnostics.push("consensus placement requires admitted members".to_string());
    }
    diagnostics.extend(
        input
            .admitted_members
            .iter()
            .filter(|member| !input.candidate_members.iter().any(|candidate| candidate == *member))
            .map(|member| format!("admitted member {member} is not a placement candidate")),
    );
    if input.membership_refs.is_empty() {
        diagnostics.push("consensus placement requires membership evidence".to_string());
    }
    if input.placement_policy_refs.is_empty() {
        diagnostics.push("consensus placement requires placement policy evidence".to_string());
    }
    if !input.majority_reachable {
        diagnostics.push("consensus placement lacks majority reachability".to_string());
    }
    if input.fault_domain_refs.len() < input.admitted_members.len() {
        diagnostics.push("consensus placement lacks a fault-domain ref for every admitted member".to_string());
    }
    let unique_fault_domains = input.fault_domain_refs.iter().collect::<std::collections::BTreeSet<_>>().len();
    let quorum = majority_quorum_count(input.admitted_members.len())?;
    if unique_fault_domains < quorum {
        diagnostics.push(format!(
            "unsafe fault-domain concentration: {unique_fault_domains} domains for quorum {quorum}"
        ));
    }
    Ok(diagnostics)
}

fn claim_boundary_diagnostics(claim: &str) -> Vec<String> {
    match claim {
        "control-plane-consensus" => Vec::new(),
        "byzantine-tolerance" => vec!["consensus evidence does not claim Byzantine tolerance".to_string()],
        "general-purpose-database" => vec!["consensus evidence is not a general database guarantee".to_string()],
        "ordinary-actor-ordering" => vec!["consensus evidence does not order ordinary actor traffic".to_string()],
        "global-dataspace-consistency" => vec!["consensus evidence does not prove global dataspace consistency".to_string()],
        "transport-delivery-correctness" => vec!["consensus evidence does not prove transport delivery correctness".to_string()],
        "lease-read-without-timing-policy" => {
            vec!["lease reads require accepted timing assumptions and policy evidence".to_string()]
        }
        "production-leaderless-without-evidence" => {
            vec!["leaderless production profile requires accepted proof policy simulation placement and membership evidence".to_string()]
        }
        value => vec![format!("unsupported consensus claim {value}")],
    }
}
