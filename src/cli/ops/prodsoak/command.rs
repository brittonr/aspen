include!("parts/command/p000/body.rs");

pub(super) fn artifact_kind(text: &str) -> &'static str {
    for (needle, kind) in [
        ("prod-soak-evidence-export-v1", "evidence-export"),
        ("prod-soak-durability-v1", "durability"),
        ("prod-soak-fault-case-v1", "fault-case"),
        ("prod-soak-resource-envelope-v1", "resource-envelope"),
        ("prod-soak-fault-matrix-v1", "fault-matrix"),
        ("prod-soak-run-v1", "run"),
        ("prod-ops-deployment-profile-v1", "deployment-profile"),
        ("prod-ops-backup-restore-drill-v1", "backup-restore-drill"),
        ("prod-ops-upgrade-rollback-drill-v1", "upgrade-rollback-drill"),
        ("prod-ops-observability-slo-v1", "observability-slo"),
        ("prod-ops-runbook-check-v1", "runbook-check"),
        ("prod-security-threat-model-v1", "threat-model"),
        ("prod-security-drill-v1", "security-drill"),
        ("prod-security-redaction-audit-v1", "redaction-audit"),
        ("prod-security-supply-chain-review-v1", "supply-chain-review"),
        ("prod-security-boundary-negative-suite-v1", "boundary-negative-suite"),
        ("prod-security-incident-response-drill-v1", "incident-response-drill"),
        ("prod-security-readiness-report-v1", "security-readiness-report"),
        ("prod-release-pilot-decision-v1", "pilot-decision"),
        ("prod-release-candidate-gate-v2", "release-candidate-gate"),
    ] {
        if text.contains(needle) {
            return kind;
        }
    }
    "artifact"
}
