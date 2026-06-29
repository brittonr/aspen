#[path = "readiness/operations.rs"]
mod operations;
#[path = "readiness/security.rs"]
mod security;

pub(super) struct Emission {
    pub value: preserves::IOValue,
    pub out: Option<super::FilePath>,
    pub kind: &'static str,
    pub subject: String,
    pub decision: String,
}

pub(super) fn run(command: super::Command) -> super::Outcome<()> {
    let emission = match command {
        command @ super::Command::DeploymentProfile { .. } => operations::deployment_profile(command),
        command @ super::Command::BackupRestoreDrill { .. } => operations::backup_restore_drill(command),
        command @ super::Command::UpgradeRollbackDrill { .. } => operations::upgrade_rollback_drill(command),
        command @ super::Command::ObservabilitySlo { .. } => operations::observability_slo(command),
        command @ super::Command::RunbookCheck { .. } => operations::runbook_check(command),
        command @ super::Command::PilotDecision { .. } => operations::pilot_decision(command),
        command @ super::Command::ReleaseCandidateGate { .. } => operations::release_candidate_gate(command),
        command @ super::Command::ThreatModel { .. } => security::threat_model(command),
        command @ super::Command::SecurityDrill { .. } => security::drill(command),
        command @ super::Command::RedactionAudit { .. } => security::redaction_audit(command),
        command @ super::Command::SupplyChainReview { .. } => security::supply_chain_review(command),
        command @ super::Command::BoundaryNegativeSuite { .. } => security::boundary_negative_suite(command),
        command @ super::Command::IncidentResponseDrill { .. } => security::incident_response_drill(command),
        command @ super::Command::SecurityReadinessReport { .. } => security::report(command),
        _ => return Err(super::wrong_handler("readiness")),
    }?;
    let reference = molten::preserves_rail::canonical_hash(&emission.value)?;
    super::emit_value(
        emission.out.as_ref(),
        &emission.value,
        &format!(
            "prod-readiness {} ref={} decision={} subject={}",
            emission.kind, reference, emission.decision, emission.subject
        ),
    )
}
