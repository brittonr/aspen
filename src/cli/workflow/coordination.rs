use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::coordination;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;
use molten::raft_control_plane;

const COORDINATION_CLI_BATCH_REF_LIMIT: usize = 4096;
const COORDINATION_CLI_BATCH_EVIDENCE_LIMIT: usize = 16384;
const _: () = assert!(COORDINATION_CLI_BATCH_REF_LIMIT <= 100_000);
const _: () = assert!(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT <= 100_000);

#[derive(Debug, Subcommand)]
pub(crate) enum CoordinationCommand {
    Manifest {
        #[arg(long, default_value = "coordination:local")]
        service_id: String,
        #[arg(long = "service")]
        services: Vec<String>,
        #[arg(long)]
        control_group_ref: Option<String>,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_QUEUE_CAPACITY)]
        queue_capacity: u64,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_SEMAPHORE_CAPACITY)]
        semaphore_capacity: u64,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_RATE_LIMIT)]
        rate_limit: u64,
        #[arg(long, default_value_t = coordination::DEFAULT_COORDINATION_BARRIER_PARTIES)]
        barrier_parties: u64,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Request {
        #[arg(long)]
        service: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        client_session: String,
        #[arg(long)]
        operation_id_ref: String,
        #[arg(long)]
        payload: Option<PathBuf>,
        #[arg(long = "authority-ref")]
        authority_refs: Vec<String>,
        #[arg(long = "resource-ref")]
        resource_refs: Vec<String>,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Apply {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long = "request")]
        requests: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    RunFixture {
        #[arg(long)]
        out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

struct CoordinationCliBoundedItems<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> CoordinationCliBoundedItems<T> {
    fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    fn push(&mut self, value: T) -> Result<()> {
        if self.values.len() >= self.maximum {
            return Err(MoltenError::invalid_harness(format!("{} count exceeds {}", self.label, self.maximum)));
        }
        self.values.push(value);
        Ok(())
    }

    fn into_vec(self) -> Vec<T> {
        self.values
    }
}

pub(crate) fn run_coordination_command(command: CoordinationCommand) -> Result<()> {
    match command {
        CoordinationCommand::Manifest {
            service_id,
            services,
            control_group_ref,
            queue_capacity,
            semaphore_capacity,
            rate_limit,
            barrier_parties,
            policy_refs,
            resource_refs,
            out,
        } => {
            let control_group_ref = match control_group_ref {
                Some(reference) => reference,
                None => canonical_hash(&raft_control_plane::control_registry_fixture_manifest_value()?)?,
            };
            let services = if services.is_empty() {
                coordination::coordination_supported_services()
            } else {
                services
            };
            let value =
                coordination::coordination_service_manifest_value(&coordination::CoordinationServiceManifestInput {
                    service_id,
                    services,
                    control_group_ref,
                    queue_capacity,
                    semaphore_capacity,
                    rate_limit,
                    barrier_parties,
                    policy_refs,
                    resource_refs,
                })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(is_written_to_file, &format!("coordination manifest ref={reference}"));
            Ok(())
        }
        CoordinationCommand::Request {
            service,
            operation,
            key,
            client_session,
            operation_id_ref,
            payload,
            authority_refs,
            resource_refs,
            policy_refs,
            out,
        } => {
            let payload = payload.as_ref().map(|path| read_preserves_file(path)).transpose()?;
            let value = coordination::coordination_request_value(&coordination::CoordinationRequestInput {
                service,
                operation,
                key,
                client_session,
                operation_id_ref,
                payload,
                authority_refs,
                resource_refs,
                policy_refs,
            })?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(is_written_to_file, &format!("coordination request ref={reference}"));
            Ok(())
        }
        CoordinationCommand::Apply {
            manifest,
            requests,
            out,
        } => {
            if requests.is_empty() {
                return Err(MoltenError::invalid_harness("coordination apply requires at least one --request file"));
            }
            let manifest_value = read_preserves_file(&manifest)?;
            let mut runtime = coordination::new_coordination_runtime(&manifest_value)?;
            let manifest_ref = runtime.manifest.manifest_ref.clone();
            let mut decision = "pass";
            let mut evidence_values =
                CoordinationCliBoundedItems::new(COORDINATION_CLI_BATCH_EVIDENCE_LIMIT, "coordination apply evidence");
            evidence_values.push(manifest_value)?;
            let mut receipt_refs =
                CoordinationCliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "coordination apply receipts");
            let mut assertion_refs =
                CoordinationCliBoundedItems::new(COORDINATION_CLI_BATCH_REF_LIMIT, "coordination apply assertions");
            for request in requests {
                let request_value = read_preserves_file(&request)?;
                let result = coordination::apply_coordination_request(&mut runtime, &request_value)?;
                if result.receipt.decision != "pass" {
                    decision = "deny";
                }
                receipt_refs.push(result.receipt.receipt_ref.clone())?;
                for assertion in &result.assertions {
                    assertion_refs.push(assertion.assertion_ref.clone())?;
                }
                for value in &result.evidence_values {
                    evidence_values.push(value.clone())?;
                }
            }
            let final_state_value = coordination::coordination_state_snapshot_value(&runtime.state)?;
            let final_state_ref = canonical_hash(&final_state_value)?;
            evidence_values.push(final_state_value)?;
            let evidence_values = evidence_values.into_vec();
            let receipt_refs = receipt_refs.into_vec();
            let assertion_refs = assertion_refs.into_vec();
            let evidence_refs = evidence_values.iter().map(canonical_hash).collect::<Result<Vec<_>>>()?;
            let report_value = coordination::coordination_apply_report_value(coordination::ApplyReportValueInput {
                decision,
                manifest_ref: &manifest_ref,
                final_state_ref: &final_state_ref,
                receipt_refs: &receipt_refs,
                assertion_refs: &assertion_refs,
                evidence_refs: &evidence_refs,
            })?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&report_value)?)?;
            write_indexed_values(&out, "evidence", &evidence_values)?;
            println!(
                "coordination apply decision={} manifest={} state={} receipts={} assertions={} evidence={} out={}",
                decision,
                manifest_ref,
                final_state_ref,
                receipt_refs.len(),
                assertion_refs.len(),
                evidence_refs.len(),
                out.display()
            );
            Ok(())
        }
        CoordinationCommand::RunFixture { out } => {
            let run = coordination::run_coordination_fixture()?;
            fs::create_dir_all(&out).map_err(MoltenError::from)?;
            write_file(&out.join("report.preserves"), &to_text(&run.report_value)?)?;
            write_indexed_values(&out, "evidence", &run.evidence_values)?;
            println!(
                "coordination fixture decision={} manifest={} state={} receipts={} assertions={} out={}",
                run.decision,
                run.manifest_ref,
                run.final_state_ref,
                run.receipt_refs.len(),
                run.assertion_refs.len(),
                out.display()
            );
            Ok(())
        }
        CoordinationCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", coordination::coordination_summary(&value)?);
            Ok(())
        }
    }
}

fn write_indexed_values(out: &Path, prefix: &str, values: &[preserves::IOValue]) -> Result<()> {
    for (index, value) in values.iter().enumerate() {
        write_file(&out.join(format!("{prefix}-{index}.preserves")), &to_text(value)?)?;
    }
    Ok(())
}

fn write_optional_preserves(out: Option<&PathBuf>, value: &preserves::IOValue) -> Result<bool> {
    if let Some(path) = out {
        write_file(path, &to_text(value)?)?;
        Ok(true)
    } else {
        println!("{}", to_text(value)?);
        Ok(false)
    }
}

fn print_or_log_summary(is_written_to_file: bool, summary: &str) {
    if is_written_to_file {
        println!("{summary}");
    } else {
        eprintln!("{summary}");
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
