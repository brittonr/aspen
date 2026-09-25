type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Topology(TopologyInput),
    NodeEvidence(NodeInput),
    RunReceipt(ReceiptInput),
    Validate(ValidateInput),
    Manifest(ManifestInput),
    FaultDescriptor(FaultDescriptorInput),
    FaultReceipt(FaultReceiptInput),
    FaultValidate(FaultValidateInput),
    ShardRun(ShardRunInput),
    Aggregate(AggregateInput),
    Show { artifact: FilePath },
}

#[derive(Debug, clap::Args)]
pub(crate) struct TopologyInput {
    #[arg(long = "node")]
    pub(super) nodes: Vec<String>,
    #[arg(long)]
    pub(super) package_ref: String,
    #[arg(long)]
    pub(super) package_path: String,
    #[arg(long, default_value = "nixos-test-private")]
    pub(super) network: String,
    #[arg(long = "nix-input")]
    pub(super) nix_inputs: Vec<String>,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct NodeInput {
    #[arg(long)]
    pub(super) node: String,
    #[arg(long)]
    pub(super) state_root: FilePath,
    #[arg(long)]
    pub(super) identity: Option<FilePath>,
    #[arg(long)]
    pub(super) startup: FilePath,
    #[arg(long)]
    pub(super) health: FilePath,
    #[arg(long)]
    pub(super) control_loop: FilePath,
    #[arg(long)]
    pub(super) heartbeat: FilePath,
    #[arg(long)]
    pub(super) shutdown: Option<FilePath>,
    #[arg(long = "log")]
    pub(super) logs: Vec<FilePath>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ReceiptInput {
    #[arg(long)]
    pub(super) topology: FilePath,
    #[arg(long = "node-evidence")]
    pub(super) node_evidence: Vec<FilePath>,
    #[arg(long)]
    pub(super) scenario: String,
    #[arg(long, default_value = "none")]
    pub(super) fault_profile: String,
    #[arg(long = "child-ref")]
    pub(super) child_refs: Vec<String>,
    #[arg(long = "log")]
    pub(super) logs: Vec<FilePath>,
    #[arg(long, default_value = "pass")]
    pub(super) decision: String,
    #[arg(long, default_value = "non-replayable-vm-observations")]
    pub(super) replay_status: String,
    #[arg(long = "diagnostic")]
    pub(super) diagnostics: Vec<String>,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ValidateInput {
    #[arg(long)]
    pub(super) topology: FilePath,
    #[arg(long = "node-evidence")]
    pub(super) node_evidence: Vec<FilePath>,
    #[arg(long = "test-run")]
    pub(super) test_run: FilePath,
    #[arg(long = "prod-soak")]
    pub(super) prod_soak: Vec<FilePath>,
    #[arg(long = "child-artifact")]
    pub(super) child_artifacts: Vec<FilePath>,
    #[arg(long = "expected-node")]
    pub(super) expected_nodes: Vec<String>,
    #[arg(long = "expected-package-ref")]
    pub(super) expected_package_ref: Option<String>,
    #[arg(long = "expected-child-ref")]
    pub(super) expected_child_refs: Vec<String>,
    #[arg(long = "expected-child-receipt")]
    pub(super) expected_child_receipts: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ManifestInput {
    #[arg(long)]
    pub(super) root: Option<FilePath>,
    #[arg(long = "artifact")]
    pub(super) artifacts: Vec<FilePath>,
    #[arg(long = "log")]
    pub(super) logs: Vec<FilePath>,
    #[arg(long = "required-artifact")]
    pub(super) required_artifacts: Vec<String>,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FaultDescriptorInput {
    #[arg(long = "fault-id")]
    pub(super) fault_id: String,
    #[arg(long)]
    pub(super) topology: FilePath,
    #[arg(long = "target-node")]
    pub(super) target_node: String,
    #[arg(long = "target-link")]
    pub(super) target_link: Option<String>,
    #[arg(long = "fault-kind")]
    pub(super) fault_kind: String,
    #[arg(long = "command-profile")]
    pub(super) command_profile: String,
    #[arg(long = "expected-outcome")]
    pub(super) expected_outcome: String,
    #[arg(long = "duration-millis")]
    pub(super) duration_millis: u64,
    #[arg(long)]
    pub(super) trigger: String,
    #[arg(long = "preflight")]
    pub(super) preflight: Vec<FilePath>,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FaultReceiptInput {
    #[arg(long)]
    pub(super) descriptor: FilePath,
    #[arg(long, default_value = "pass")]
    pub(super) decision: String,
    #[arg(long = "host-support", default_value = "supported")]
    pub(super) host_support: String,
    #[arg(long = "pre-fault")]
    pub(super) pre_fault: Vec<FilePath>,
    #[arg(long = "injection")]
    pub(super) injection: Vec<FilePath>,
    #[arg(long = "child")]
    pub(super) children: Vec<FilePath>,
    #[arg(long = "post-fault")]
    pub(super) post_fault: Vec<FilePath>,
    #[arg(long = "replay-status", default_value = "non-replayable-vm-observations")]
    pub(super) replay_status: String,
    #[arg(long = "diagnostic")]
    pub(super) diagnostics: Vec<String>,
    #[arg(long = "log")]
    pub(super) logs: Vec<FilePath>,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct FaultValidateInput {
    #[arg(long)]
    pub(super) topology: FilePath,
    #[arg(long = "descriptor")]
    pub(super) descriptors: Vec<FilePath>,
    #[arg(long = "receipt")]
    pub(super) receipts: Vec<FilePath>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ShardRunInput {
    #[arg(long = "shard-id")]
    pub(super) shard_id: String,
    #[arg(long = "scenario-fixture-ref")]
    pub(super) scenario_fixture_ref: String,
    #[arg(long = "topology-ref")]
    pub(super) topology_ref: String,
    #[arg(long = "package-ref")]
    pub(super) package_ref: String,
    #[arg(long = "evidence-scope", default_value = "executable-vm")]
    pub(super) evidence_scope: String,
    #[arg(long = "node-evidence-ref")]
    pub(super) node_evidence_refs: Vec<String>,
    #[arg(long = "child-receipt-ref")]
    pub(super) child_receipt_refs: Vec<String>,
    #[arg(long = "diagnostic-log-ref")]
    pub(super) diagnostic_log_refs: Vec<String>,
    #[arg(long)]
    pub(super) unavailable: bool,
    #[arg(long = "claimed-decision", default_value = "pass")]
    pub(super) claimed_decision: String,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct AggregateInput {
    #[arg(long = "topology-ref")]
    pub(super) topology_ref: String,
    #[arg(long = "package-ref")]
    pub(super) package_ref: String,
    #[arg(long = "manifest-ref")]
    pub(super) manifest_ref: String,
    #[arg(long = "required-shard-id")]
    pub(super) required_shard_ids: Vec<String>,
    #[arg(long = "shard-ref")]
    pub(super) shard_refs: Vec<String>,
    #[arg(long = "shard-scope")]
    pub(super) shard_scopes: Vec<String>,
    #[arg(long = "denied-shard-id")]
    pub(super) denied_shard_ids: Vec<String>,
    #[arg(long = "unavailable-as-pass-shard-id")]
    pub(super) unavailable_as_pass_shard_ids: Vec<String>,
    #[arg(long = "stale-child-ref")]
    pub(super) stale_child_refs: Vec<String>,
    #[arg(long = "log-only-child-ref")]
    pub(super) log_only_child_refs: Vec<String>,
    #[arg(long = "caveat")]
    pub(super) caveats: Vec<String>,
    #[arg(long)]
    pub(super) out: Option<FilePath>,
}
