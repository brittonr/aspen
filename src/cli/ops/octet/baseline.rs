#[derive(Debug, clap::Subcommand)]
pub(crate) enum Command {
    Write {
        #[arg(long, default_value = "target/octet")]
        artifacts: std::path::PathBuf,
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long, default_value = "manual")]
        created_at: String,
        #[arg(long)]
        expires_at: String,
        #[arg(long)]
        target_next: Option<u64>,
    },
    Check {
        #[arg(long, default_value = "target/octet")]
        artifacts: std::path::PathBuf,
        #[arg(long)]
        baseline: std::path::PathBuf,
        #[arg(long, default_value = "quarantine-ci")]
        profile: String,
        #[arg(long)]
        as_of: String,
        #[arg(long)]
        receipt_out: Option<std::path::PathBuf>,
        #[arg(long = "review")]
        reviews: Vec<std::path::PathBuf>,
    },
}

pub(super) fn run(command: Command) -> molten::error::Result<()> {
    match command {
        Command::Write {
            artifacts,
            out,
            created_at,
            expires_at,
            target_next,
        } => {
            let baseline =
                molten::octet_gate::build_octet_warning_baseline(&molten::octet_gate::OctetWarningBaselineInput {
                    artifacts_dir: artifacts,
                    created_at,
                    expires_at,
                    target_next,
                })?;
            super::io::write_file(&out, &molten::preserves_rail::to_text(&baseline.baseline_value)?)?;
            println!(
                "octet warning baseline {} written to {} findings={} critical={}",
                baseline.baseline_ref,
                out.display(),
                baseline.finding_count,
                baseline.critical_count
            );
            Ok(())
        }
        Command::Check {
            artifacts,
            baseline,
            profile,
            as_of,
            receipt_out,
            reviews,
        } => {
            let baseline_value = super::io::read_preserves_file(&baseline)?;
            let review_values = reviews
                .iter()
                .map(|path| super::io::read_preserves_file(path))
                .collect::<molten::error::Result<Vec<_>>>()?;
            let evaluation =
                molten::octet_gate::check_octet_warning_baseline(&molten::octet_gate::OctetBaselineCheckInput {
                    artifacts_dir: artifacts.clone(),
                    baseline_value,
                    profile,
                    as_of,
                    review_values,
                })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "octet baseline receipt", &evaluation.receipt_value)?;
            if evaluation.decision != "pass" {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "octet baseline denied receipt={} artifacts={}",
                    evaluation.receipt_ref,
                    artifacts.display()
                )));
            }
            println!("octet baseline pass receipt={}", evaluation.receipt_ref);
            Ok(())
        }
    }
}
