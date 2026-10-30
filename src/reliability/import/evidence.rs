//! Pure admission of external finite live observations. Never an authority grant.

#[derive(Debug, Clone)]
pub struct Expected {
    pub admitted: crate::live_reliability::profile::Admitted,
    pub plan: crate::live_reliability::generator::OperationPlan,
    pub identities: std::collections::BTreeMap<String, String>,
    pub endpoints: std::collections::BTreeSet<String>,
    pub max_events: usize,
    pub max_witness_bytes: usize,
    pub max_recovery_ms: u64,
    pub checked: crate::live_reliability::checker::CheckedReport,
    pub reference_checked: Option<crate::live_reliability::checker::CheckedReport>,
    pub readiness: std::collections::BTreeMap<String, crate::live_reliability::evidenceproof::ExpectedReadiness>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observed {
    pub identities: std::collections::BTreeMap<String, String>,
    pub profile_ref: String,
    pub plan_ref: String,
    pub history: serde_json::Value,
    pub report: serde_json::Value,
    pub reference_report: Option<serde_json::Value>,
    pub faults: Vec<FaultObservation>,
    pub recovery: Vec<FinalRead>,
    pub readiness: std::collections::BTreeMap<String, crate::live_reliability::evidenceproof::ReadinessEvidence>,
    pub teardown: bool,
    pub non_claims: std::collections::BTreeSet<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultObservation {
    pub id: String,
    pub mechanism: String,
    pub selected: u64,
    pub applied: u64,
    pub observed: u64,
    pub healed: u64,
    pub recovered: u64,
    pub effect_ref: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalRead {
    pub endpoint: String,
    pub attempt_id: String,
    pub value: serde_json::Value,
    pub ready: bool,
    pub ready_event: u64,
    pub stable_membership_event: u64,
    pub stable_membership_observations: u32,
    pub stable_membership: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Issue {
    Bounds,
    Identity,
    Plan,
    History,
    Fault,
    Recovery,
    Checker,
    Witness,
    Teardown,
    Claim,
}

#[derive(Debug, Clone)]
pub struct ExternalReceipt {
    pub receipt_ref: String,
    pub value: preserves::IOValue,
    pub verdict: String,
    pub status: String,
}

pub const NON_CLAIMS: [&str; 10] = [
    "authority",
    "policy",
    "resources",
    "provenance",
    "transport",
    "retention",
    "deployment",
    "release",
    "universal-correctness",
    "deterministic-replay",
];
const IDENTITIES: [&str; 15] = [
    "semantic-history",
    "checker",
    "model",
    "onixos-live-reliability",
    "onixos-native-service",
    "molten-build",
    "service-profile",
    "generator",
    "deployment",
    "adapter",
    "cluster",
    "artifact",
    "source",
    "profile",
    "plan",
];

// r[impl molten.testing.live_reliability_evidence]
// r[impl molten.testing.live_reliability_boundary]
pub fn import(expected: &Expected, evidence: &Observed) -> Result<ExternalReceipt, Issue> {
    checks::validate_identity(expected, evidence)?;
    checks::validate_checked(expected, evidence)?;
    crate::live_reliability::generator::validate_plan(&expected.admitted, &expected.plan).map_err(|_| Issue::Plan)?;
    let history = serde_json::from_value(evidence.history.clone()).map_err(|_| Issue::History)?;
    crate::live_reliability::history::validate(&expected.admitted, &expected.plan, &history)
        .map_err(|_| Issue::History)?;
    let history_id = crate::live_reliability::checker::semantic_history_id(&history).map_err(|_| Issue::Checker)?;
    if evidence.report["history_id"].as_str() != Some(history_id.as_str()) {
        return Err(Issue::Checker);
    }
    let events = evidence.history["events"].as_array().ok_or(Issue::History)?;
    if events.len() > expected.max_events
        || evidence.history["source_artifact"].as_str() != expected.identities.get("source").map(String::as_str)
    {
        return Err(Issue::History);
    }
    checks::validate_faults(evidence, events)?;
    checks::validate_recovery(expected, evidence, events)?;
    crate::live_reliability::evidenceproof::validate(&expected.endpoints, &expected.readiness, &evidence.readiness)?;
    let verdict = checks::validate_report(expected, evidence)?;
    if !evidence.teardown || !events.last().is_some_and(|e| e["kind"] == "lifecycle" && e["phase"] == "teardown") {
        return Err(Issue::Teardown);
    }
    let required: std::collections::BTreeSet<_> = NON_CLAIMS.iter().map(|v| (*v).to_owned()).collect();
    if evidence.non_claims != required {
        return Err(Issue::Claim);
    }
    let is_disagreement =
        evidence.reference_report.as_ref().is_some_and(|r| r["verdict"] != evidence.report["verdict"])
            || evidence.report["reference_status"] == "disagreement";
    let status = if is_disagreement {
        "checker-disagreement"
    } else {
        match verdict {
            "valid" => "bounded-valid",
            "invalid" => "external-failure",
            _ => "inconclusive",
        }
    };
    let receipt = serde_json::json!({"schema":"molten.external-live-reliability.v1",
        "environment":"onixos-live", "status":status, "verdict":verdict,
        "authority":false, "scope":"exact finite admitted history only", "evidence":evidence});
    let value = crate::preserves_rail::record("external-live-reliability-receipt-v1", vec![
        crate::live_reliability::checker::canonical_value(&receipt).map_err(|_| Issue::Bounds)?,
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value).map_err(|_| Issue::Bounds)?;
    Ok(ExternalReceipt {
        receipt_ref,
        value,
        verdict: verdict.into(),
        status: status.into(),
    })
}

// r[impl molten.testing.live_reliability_evidence]
pub fn persist(
    root: &crate::ledger::CapabilityLedgerRoot,
    receipt: &ExternalReceipt,
) -> crate::error::Result<crate::ledger::Import> {
    if crate::preserves_rail::canonical_hash(&receipt.value)? != receipt.receipt_ref {
        return Err(crate::error::Failure::invalid_harness("live evidence receipt identity mismatch"));
    }
    crate::ledger::import_artifact_with_root(root, &receipt.value)
}

#[path = "evidence/checks.rs"]
mod checks;

#[cfg(test)]
#[path = "evidencetests.rs"]
mod tests;
