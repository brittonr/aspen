#[allow(clippy::large_enum_variant)]
#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    EvidenceExport {
        #[arg(long)]
        node: String,
        #[arg(long)]
        node_evidence: std::path::PathBuf,
        #[arg(long = "artifact")]
        artifacts: Vec<std::path::PathBuf>,
        #[arg(long = "log")]
        logs: Vec<std::path::PathBuf>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    Durability {
        #[arg(long)]
        scenario: String,
        #[arg(long = "queued-control-ref")]
        queued_control_refs: Vec<String>,
        #[arg(long = "recovery-ref")]
        recovery_refs: Vec<String>,
        #[arg(long = "ledger-ref")]
        ledger_refs: Vec<String>,
        #[arg(long = "chunk-ref")]
        chunk_refs: Vec<String>,
        #[arg(long = "retention-ref")]
        retention_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    FaultCase {
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        fault_kind: String,
        #[arg(long, default_value = "simulated")]
        injection: String,
        #[arg(long, default_value = "deny-before-side-effects")]
        expected_outcome: String,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long = "denial-ref")]
        denial_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long, default_value = "simulated-fault")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    ResourceEnvelope {
        #[arg(long)]
        scenario: String,
        #[arg(long)]
        queue_depth: u64,
        #[arg(long)]
        max_queue_depth: u64,
        #[arg(long)]
        receipt_bytes: u64,
        #[arg(long)]
        max_receipt_bytes: u64,
        #[arg(long)]
        store_bytes: u64,
        #[arg(long)]
        max_store_bytes: u64,
        #[arg(long)]
        delivery_latency_ms: u64,
        #[arg(long)]
        max_delivery_latency_ms: u64,
        #[arg(long)]
        recovery_time_ms: u64,
        #[arg(long)]
        max_recovery_time_ms: u64,
        #[arg(long = "pressure-ref")]
        pressure_refs: Vec<String>,
        #[arg(long = "denial-ref")]
        denial_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    FaultMatrix {
        #[arg(long)]
        scenario: String,
        #[arg(long = "fault-case")]
        fault_cases: Vec<std::path::PathBuf>,
        #[arg(long = "fault-kind")]
        fault_kinds: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    RunReceipt {
        #[arg(long)]
        topology: std::path::PathBuf,
        #[arg(long = "node-evidence")]
        node_evidence: Vec<std::path::PathBuf>,
        #[arg(long)]
        scenario: String,
        #[arg(long, default_value = "none")]
        fault_profile: String,
        #[arg(long = "peer-ticket-ref")]
        peer_ticket_refs: Vec<String>,
        #[arg(long = "node-control-ref")]
        node_control_refs: Vec<String>,
        #[arg(long = "remote-service-ref")]
        remote_service_refs: Vec<String>,
        #[arg(long = "job-ref")]
        job_refs: Vec<String>,
        #[arg(long = "coordination-ref")]
        coordination_refs: Vec<String>,
        #[arg(long = "evidence-export")]
        evidence_exports: Vec<std::path::PathBuf>,
        #[arg(long = "fault-ref")]
        fault_refs: Vec<String>,
        #[arg(long = "durability-ref")]
        durability_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long, default_value = "non-replayable-live-observations")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "log")]
        logs: Vec<std::path::PathBuf>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    Show {
        artifact: std::path::PathBuf,
    },
}

pub(super) fn artifact_kind(text: &str) -> &'static str {
    for (needle, kind) in [
        ("prod-soak-evidence-export-v1", "evidence-export"),
        ("prod-soak-durability-v1", "durability"),
        ("prod-soak-fault-case-v1", "fault-case"),
        ("prod-soak-resource-envelope-v1", "resource-envelope"),
        ("prod-soak-fault-matrix-v1", "fault-matrix"),
        ("prod-soak-run-v1", "run"),
    ] {
        if text.contains(needle) {
            return kind;
        }
    }
    "artifact"
}
