use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::PROD_SOAK_EVIDENCE_EXPORT_SCHEMA;
use crate::preserves_rail::PROD_SOAK_RUN_SCHEMA;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;

const MAX_SOAK_REFS: usize = 512;
const MAX_SOAK_TEXT_FIELDS: usize = 128;
const _: () = assert!(MAX_SOAK_REFS <= 100_000);
const _: () = assert!(MAX_SOAK_TEXT_FIELDS <= 100_000);

pub struct ProdSoakEvidenceExportInput<'a> {
    pub node: &'a str,
    pub node_evidence_ref: &'a str,
    pub artifact_refs: &'a [String],
    pub log_refs: &'a [String],
}

pub struct ProdSoakRunInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub topology_ref: &'a str,
    pub fault_profile: &'a str,
    pub node_evidence_refs: &'a [String],
    pub peer_ticket_refs: &'a [String],
    pub node_control_refs: &'a [String],
    pub remote_service_refs: &'a [String],
    pub job_refs: &'a [String],
    pub coordination_refs: &'a [String],
    pub evidence_export_refs: &'a [String],
    pub replay_status: &'a str,
    pub diagnostics: &'a [String],
    pub log_refs: &'a [String],
    pub caveats: &'a [String],
}

pub fn evidence_export_value(input: &ProdSoakEvidenceExportInput<'_>) -> Result<IOValue> {
    validate_text_field("node", input.node)?;
    validate_content_ref(input.node_evidence_ref)?;
    validate_ref_slice("evidence export artifact", input.artifact_refs)?;
    validate_ref_slice("evidence export log", input.log_refs)?;
    Ok(record("prod-soak-evidence-export-v1", vec![
        string(PROD_SOAK_EVIDENCE_EXPORT_SCHEMA),
        record("node", vec![string(input.node)]),
        record("node-evidence", vec![string(input.node_evidence_ref)]),
        record("artifacts", vec![sequence(ref_values(input.artifact_refs)?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("checks", vec![sequence(vec![
            check_value("node-evidence-bound", "pass"),
            check_value("artifacts-exported", status(input.artifact_refs.is_empty())),
            check_value("logs-diagnostic-only", "pass"),
            check_value("export-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn run_value(input: &ProdSoakRunInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_content_ref(input.topology_ref)?;
    validate_text_field("fault profile", input.fault_profile)?;
    validate_ref_slice("node evidence", input.node_evidence_refs)?;
    validate_ref_slice("peer ticket", input.peer_ticket_refs)?;
    validate_ref_slice("node control", input.node_control_refs)?;
    validate_ref_slice("remote service", input.remote_service_refs)?;
    validate_ref_slice("job", input.job_refs)?;
    validate_ref_slice("coordination", input.coordination_refs)?;
    validate_ref_slice("evidence export", input.evidence_export_refs)?;
    validate_text_field("replay status", input.replay_status)?;
    validate_ref_slice("log", input.log_refs)?;
    validate_pass_category("node evidence", input.node_evidence_refs, input.decision)?;
    validate_pass_category("peer ticket", input.peer_ticket_refs, input.decision)?;
    validate_pass_category("node control", input.node_control_refs, input.decision)?;
    validate_pass_category("remote service", input.remote_service_refs, input.decision)?;
    validate_pass_category("job", input.job_refs, input.decision)?;
    validate_pass_category("coordination", input.coordination_refs, input.decision)?;
    validate_pass_category("evidence export", input.evidence_export_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-run-v1", vec![
        string(PROD_SOAK_RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("topology", vec![string(input.topology_ref)]),
        record("fault-profile", vec![string(input.fault_profile)]),
        record("node-evidence", vec![sequence(ref_values(input.node_evidence_refs)?)]),
        record("peer-tickets", vec![sequence(ref_values(input.peer_ticket_refs)?)]),
        record("node-control-workflows", vec![sequence(ref_values(input.node_control_refs)?)]),
        record("remote-service", vec![sequence(ref_values(input.remote_service_refs)?)]),
        record("job-workers", vec![sequence(ref_values(input.job_refs)?)]),
        record("coordination", vec![sequence(ref_values(input.coordination_refs)?)]),
        record("evidence-exports", vec![sequence(ref_values(input.evidence_export_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "diagnostic",
            input.diagnostics,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("caveats", vec![sequence(string_values(
            "soak caveat",
            input.caveats,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("production-shaped-child-evidence", "pass"),
            check_value("replay-boundary-explicit", "pass"),
            check_value("live-caveats-explicit", status(input.caveats.is_empty())),
            check_value("soak-evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

fn validate_pass_category(label: &str, refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && refs.is_empty() {
        Err(MoltenError::invalid_harness(format!("passing prod soak run requires at least one {label} ref")))
    } else {
        Ok(())
    }
}

fn validate_pass_caveats(caveats: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && caveats.is_empty() {
        Err(MoltenError::invalid_harness("passing prod soak run requires explicit evidence-only caveats"))
    } else {
        Ok(())
    }
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("prod soak {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_SOAK_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak {label} ref count {} exceeds bound {MAX_SOAK_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid prod soak {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported prod soak decision {other}; expected pass, deny, unavailable, or skipped"
        ))),
    }
}

fn ref_values(refs: &[String]) -> Result<Vec<IOValue>> {
    validate_ref_slice("artifact", refs)?;
    Ok(refs.iter().map(string).collect())
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IOValue>> {
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak {label} count {} exceeds bound {maximum}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text_field(label, value)?;
        output.push(string(value));
    }
    Ok(output)
}

fn check_value(name: &'static str, status: &'static str) -> IOValue {
    record("check", vec![string(name), string(status)])
}

fn status(is_problem: bool) -> &'static str {
    if is_problem { "deny" } else { "pass" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preserves_rail::canonical_hash;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::preserves_rail::to_text;

    fn local_ref(name: &str) -> String {
        content_ref_from_bytes(name.as_bytes())
    }

    #[test]
    fn evidence_export_binds_node_and_artifacts() {
        let artifact_ref = local_ref("artifact");
        let value = evidence_export_value(&ProdSoakEvidenceExportInput {
            node: "node-a",
            node_evidence_ref: &local_ref("node-evidence"),
            artifact_refs: std::slice::from_ref(&artifact_ref),
            log_refs: &[local_ref("log")],
        })
        .expect("evidence export");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-evidence-export-v1"));
        assert!(text.contains(&artifact_ref));
    }

    #[test]
    fn run_receipt_binds_phase_one_child_categories() {
        let node_evidence = vec![local_ref("node-a"), local_ref("node-b")];
        let peer_ticket = vec![local_ref("ticket")];
        let node_control = vec![local_ref("protocol-gate")];
        let remote_service = vec![local_ref("remote-deliver")];
        let job = vec![local_ref("job-worker")];
        let coordination = vec![local_ref("coordination")];
        let evidence_export = vec![local_ref("export-a"), local_ref("export-b")];
        let value = run_value(&ProdSoakRunInput {
            decision: "pass",
            scenario: "phase1-soak",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &node_evidence,
            peer_ticket_refs: &peer_ticket,
            node_control_refs: &node_control,
            remote_service_refs: &remote_service,
            job_refs: &job,
            coordination_refs: &coordination,
            evidence_export_refs: &evidence_export,
            replay_status: "non-replayable-live-observations",
            diagnostics: &[],
            log_refs: &[local_ref("log")],
            caveats: &["soak evidence is pilot-scoped".to_string()],
        })
        .expect("soak run");
        let reference = canonical_hash(&value).expect("ref");
        let text = to_text(&value).expect("text");
        assert!(reference.starts_with("blake3:"));
        assert!(text.contains("prod-soak-run-v1"));
        assert!(text.contains("phase1-soak"));
        assert!(text.contains(&remote_service[0]));
    }

    #[test]
    fn passing_run_requires_all_phase_one_categories() {
        let error = run_value(&ProdSoakRunInput {
            decision: "pass",
            scenario: "missing-remote",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &[local_ref("node")],
            peer_ticket_refs: &[local_ref("ticket")],
            node_control_refs: &[local_ref("control")],
            remote_service_refs: &[],
            job_refs: &[local_ref("job")],
            coordination_refs: &[local_ref("coordination")],
            evidence_export_refs: &[local_ref("export")],
            replay_status: "non-replayable-live-observations",
            diagnostics: &[],
            log_refs: &[],
            caveats: &["diagnostic only".to_string()],
        })
        .expect_err("missing remote should deny pass");
        assert!(error.to_string().contains("remote service"));
    }
}
