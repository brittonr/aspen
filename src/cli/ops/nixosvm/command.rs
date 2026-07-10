type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Topology {
        #[arg(long = "node")]
        nodes: Vec<String>,
        #[arg(long)]
        package_ref: String,
        #[arg(long)]
        package_path: String,
        #[arg(long, default_value = "nixos-test-private")]
        network: String,
        #[arg(long = "nix-input")]
        nix_inputs: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    NodeEvidence {
        #[arg(long)]
        node: String,
        #[arg(long)]
        state_root: FilePath,
        #[arg(long)]
        identity: Option<FilePath>,
        #[arg(long)]
        startup: FilePath,
        #[arg(long)]
        health: FilePath,
        #[arg(long)]
        control_loop: FilePath,
        #[arg(long)]
        heartbeat: FilePath,
        #[arg(long)]
        shutdown: Option<FilePath>,
        #[arg(long = "log")]
        logs: Vec<FilePath>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    RunReceipt {
        #[arg(long)]
        topology: FilePath,
        #[arg(long = "node-evidence")]
        node_evidence: Vec<FilePath>,
        #[arg(long)]
        scenario: String,
        #[arg(long, default_value = "none")]
        fault_profile: String,
        #[arg(long = "child-ref")]
        child_refs: Vec<String>,
        #[arg(long = "log")]
        logs: Vec<FilePath>,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long, default_value = "non-replayable-vm-observations")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Validate {
        #[arg(long)]
        topology: FilePath,
        #[arg(long = "node-evidence")]
        node_evidence: Vec<FilePath>,
        #[arg(long = "test-run")]
        test_run: FilePath,
        #[arg(long = "prod-soak")]
        prod_soak: Vec<FilePath>,
        #[arg(long = "child-artifact")]
        child_artifacts: Vec<FilePath>,
        #[arg(long = "expected-node")]
        expected_nodes: Vec<String>,
        #[arg(long = "expected-package-ref")]
        expected_package_ref: Option<String>,
        #[arg(long = "expected-child-ref")]
        expected_child_refs: Vec<String>,
        #[arg(long = "expected-child-receipt")]
        expected_child_receipts: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Manifest {
        #[arg(long)]
        root: Option<FilePath>,
        #[arg(long = "artifact")]
        artifacts: Vec<FilePath>,
        #[arg(long = "log")]
        logs: Vec<FilePath>,
        #[arg(long = "required-artifact")]
        required_artifacts: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    FaultDescriptor {
        #[arg(long = "fault-id")]
        fault_id: String,
        #[arg(long)]
        topology: FilePath,
        #[arg(long = "target-node")]
        target_node: String,
        #[arg(long = "target-link")]
        target_link: Option<String>,
        #[arg(long = "fault-kind")]
        fault_kind: String,
        #[arg(long = "command-profile")]
        command_profile: String,
        #[arg(long = "expected-outcome")]
        expected_outcome: String,
        #[arg(long = "duration-millis")]
        duration_millis: u64,
        #[arg(long)]
        trigger: String,
        #[arg(long = "preflight")]
        preflight: Vec<FilePath>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    FaultReceipt {
        #[arg(long)]
        descriptor: FilePath,
        #[arg(long, default_value = "pass")]
        decision: String,
        #[arg(long = "host-support", default_value = "supported")]
        host_support: String,
        #[arg(long = "pre-fault")]
        pre_fault: Vec<FilePath>,
        #[arg(long = "injection")]
        injection: Vec<FilePath>,
        #[arg(long = "child")]
        children: Vec<FilePath>,
        #[arg(long = "post-fault")]
        post_fault: Vec<FilePath>,
        #[arg(long = "replay-status", default_value = "non-replayable-vm-observations")]
        replay_status: String,
        #[arg(long = "diagnostic")]
        diagnostics: Vec<String>,
        #[arg(long = "log")]
        logs: Vec<FilePath>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    FaultValidate {
        #[arg(long)]
        topology: FilePath,
        #[arg(long = "descriptor")]
        descriptors: Vec<FilePath>,
        #[arg(long = "receipt")]
        receipts: Vec<FilePath>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    ShardRun {
        #[arg(long = "shard-id")]
        shard_id: String,
        #[arg(long = "scenario-fixture-ref")]
        scenario_fixture_ref: String,
        #[arg(long = "topology-ref")]
        topology_ref: String,
        #[arg(long = "package-ref")]
        package_ref: String,
        #[arg(long = "evidence-scope", default_value = "executable-vm")]
        evidence_scope: String,
        #[arg(long = "node-evidence-ref")]
        node_evidence_refs: Vec<String>,
        #[arg(long = "child-receipt-ref")]
        child_receipt_refs: Vec<String>,
        #[arg(long = "diagnostic-log-ref")]
        diagnostic_log_refs: Vec<String>,
        #[arg(long, default_value_t = false)]
        unavailable: bool,
        #[arg(long = "claimed-decision", default_value = "pass")]
        claimed_decision: String,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Aggregate {
        #[arg(long = "topology-ref")]
        topology_ref: String,
        #[arg(long = "package-ref")]
        package_ref: String,
        #[arg(long = "manifest-ref")]
        manifest_ref: String,
        #[arg(long = "required-shard-id")]
        required_shard_ids: Vec<String>,
        #[arg(long = "shard-ref")]
        shard_refs: Vec<String>,
        #[arg(long = "shard-scope")]
        shard_scopes: Vec<String>,
        #[arg(long = "denied-shard-id")]
        denied_shard_ids: Vec<String>,
        #[arg(long = "unavailable-as-pass-shard-id")]
        unavailable_as_pass_shard_ids: Vec<String>,
        #[arg(long = "stale-child-ref")]
        stale_child_refs: Vec<String>,
        #[arg(long = "log-only-child-ref")]
        log_only_child_refs: Vec<String>,
        #[arg(long = "caveat")]
        caveats: Vec<String>,
        #[arg(long)]
        out: Option<FilePath>,
    },
    Show {
        artifact: FilePath,
    },
}
