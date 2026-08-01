#[derive(Debug, clap::Subcommand)]
pub(crate) enum FabricSimulationCommand {
    /// Validate the canonical reference world without running callbacks.
    Preflight,
    /// Run all reference services through deterministic fabric ports.
    Run {
        #[arg(long, default_value = "artifacts/fabric-simulation-run")]
        out: std::path::PathBuf,
    },
    /// Replay the reference world and require the supplied canonical report.
    Replay { report: std::path::PathBuf },
    /// Shrink the bounded reference failure fixture.
    Shrink {
        #[arg(long, default_value = "artifacts/fabric-simulation-shrink")]
        out: std::path::PathBuf,
    },
    /// Read and validate a canonical whole-system simulation report.
    Inspect { report: std::path::PathBuf },
    /// Export a compact offline-verifiable reference bundle.
    Export {
        #[arg(long, default_value = "artifacts/fabric-simulation-export")]
        out: std::path::PathBuf,
    },
}
