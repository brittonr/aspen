type FilePath = std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Run {
        suite: FilePath,
        #[arg(long)]
        out: FilePath,
    },
    RunTwoService {
        #[arg(long)]
        out: FilePath,
    },
    Supervise {
        suite: FilePath,
        #[arg(long)]
        out: FilePath,
    },
    RunSupervisionFixture {
        #[arg(long)]
        out: FilePath,
    },
    Show {
        report: FilePath,
    },
    ShowSupervision {
        report: FilePath,
    },
    GateSupervision {
        report: FilePath,
        #[arg(long)]
        receipt_out: Option<FilePath>,
    },
    Replay {
        report: FilePath,
    },
    ReplaySupervision {
        report: FilePath,
    },
}
