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
    Show {
        artifact: FilePath,
    },
}
