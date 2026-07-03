type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;

const EXTERNAL_LIVE_PILOT_SCHEMA: &str = "molten.external-live-pilot-decision-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotScope {
    pub allowed_workloads: Vec<String>,
    pub denied_workloads: Vec<String>,
    pub host_identity_refs: Vec<String>,
    pub rollback_triggers: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotDecisionInput {
    pub scope: PilotScope,
    pub child_evidence_refs: Vec<String>,
    pub replay_refs: Vec<String>,
    pub network_diagnostic_refs: Vec<String>,
    pub resource_envelope_refs: Vec<String>,
    pub retention_review_refs: Vec<String>,
    pub freshness_tick: u64,
    pub max_freshness_tick: u64,
    pub diagnostics_within_threshold: bool,
    pub resource_within_threshold: bool,
    pub claims_broad_production: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotDecisionReceipt {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
    pub receipt_ref: String,
}

pub fn validate_pilot_decision(input: &PilotDecisionInput) -> Result<PilotDecisionReceipt> {
    let mut diagnostics = Vec::new();
    require_non_empty(&mut diagnostics, "allowed workload", &input.scope.allowed_workloads);
    require_non_empty(&mut diagnostics, "host identity", &input.scope.host_identity_refs);
    require_non_empty(&mut diagnostics, "rollback trigger", &input.scope.rollback_triggers);
    require_non_empty(&mut diagnostics, "stop condition", &input.scope.stop_conditions);
    require_non_empty(&mut diagnostics, "child evidence", &input.child_evidence_refs);
    require_non_empty(&mut diagnostics, "replay evidence", &input.replay_refs);
    require_non_empty(&mut diagnostics, "network diagnostics", &input.network_diagnostic_refs);
    require_non_empty(&mut diagnostics, "resource envelope", &input.resource_envelope_refs);
    require_non_empty(&mut diagnostics, "retention readback", &input.retention_review_refs);
    if input.freshness_tick > input.max_freshness_tick {
        diagnostics.push("pilot evidence is stale".to_string());
    }
    if !input.diagnostics_within_threshold {
        diagnostics.push("network diagnostics outside threshold".to_string());
    }
    if !input.resource_within_threshold {
        diagnostics.push("resource envelope breach".to_string());
    }
    if input.claims_broad_production {
        diagnostics.push("constrained pilot evidence cannot claim broad production readiness".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = receipt_value(decision, &diagnostics, input);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(PilotDecisionReceipt {
        decision: decision.to_string(),
        diagnostics,
        value,
        receipt_ref,
    })
}

pub fn pilot_evidence_bundle_members() -> &'static [&'static str] {
    &[
        "node-control-workflow",
        "service-exchange",
        "blob-ref-job",
        "coordination",
        "retention-readback",
        "replay",
        "network-diagnostics",
        "resource-envelope",
        "rollback-readiness",
    ]
}

fn require_non_empty(diagnostics: &mut Vec<String>, label: &str, values: &[String]) {
    if values.is_empty() {
        diagnostics.push(format!("missing {label}"));
    }
}

fn receipt_value(decision: &str, diagnostics: &[String], input: &PilotDecisionInput) -> IoValue {
    crate::preserves_rail::record("external-live-pilot-decision-v1", vec![
        string(EXTERNAL_LIVE_PILOT_SCHEMA),
        field("decision", decision),
        list_field("allowed-workloads", &input.scope.allowed_workloads),
        list_field("denied-workloads", &input.scope.denied_workloads),
        list_field("host-identity-refs", &input.scope.host_identity_refs),
        list_field("child-evidence-refs", &input.child_evidence_refs),
        list_field("retention-review-refs", &input.retention_review_refs),
        list_field("diagnostics", diagnostics),
        field("evidence-only", "pilot-readback-does-not-grant-deployment-trust"),
    ])
}

fn field(label: &'static str, value: &str) -> IoValue {
    crate::preserves_rail::record(label, vec![string(value)])
}

fn list_field(label: &'static str, values: &[String]) -> IoValue {
    crate::preserves_rail::record(label, vec![crate::preserves_rail::sequence(values.iter().map(string).collect())])
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT_TICK: u64 = 3;
    const MAX_TICK: u64 = 4;
    const STALE_TICK: u64 = 5;

    #[test]
    fn constrained_pilot_decision_passes_with_child_evidence() {
        let receipt = validate_pilot_decision(&input(CURRENT_TICK, false, true, true)).expect("pilot");
        assert_eq!(receipt.decision, "pass");
        assert!(pilot_evidence_bundle_members().contains(&"retention-readback"));
    }

    #[test]
    fn pilot_denies_stale_overbroad_or_resource_breached_claims() {
        let receipt = validate_pilot_decision(&input(STALE_TICK, true, false, false)).expect("pilot");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource")));
        assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("broad production")));
    }

    fn input(
        freshness_tick: u64,
        claims_broad_production: bool,
        diagnostics_within_threshold: bool,
        resource_within_threshold: bool,
    ) -> PilotDecisionInput {
        PilotDecisionInput {
            scope: PilotScope {
                allowed_workloads: vec!["node-control-status".to_string()],
                denied_workloads: vec!["destructive-retention".to_string()],
                host_identity_refs: vec![test_ref("host")],
                rollback_triggers: vec!["operator-stop".to_string()],
                stop_conditions: vec!["threshold-breach".to_string()],
                caveats: vec!["pilot-only".to_string()],
            },
            child_evidence_refs: vec![test_ref("child")],
            replay_refs: vec![test_ref("replay")],
            network_diagnostic_refs: vec![test_ref("network")],
            resource_envelope_refs: vec![test_ref("resource")],
            retention_review_refs: vec![test_ref("retention")],
            freshness_tick,
            max_freshness_tick: MAX_TICK,
            diagnostics_within_threshold,
            resource_within_threshold,
            claims_broad_production,
        }
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("pilot-test-ref", vec![string(label)]))
            .expect("test ref")
    }
}
