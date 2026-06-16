use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::delivery_idempotency;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

#[derive(Debug, Subcommand)]
pub(crate) enum DeliveryCommand {
    Scope {
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: String,
        #[arg(long = "retention-ref")]
        retention_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    OperationId {
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: Option<String>,
        #[arg(long)]
        scope_ref: Option<String>,
        #[arg(long)]
        producer: String,
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        payload_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Check {
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        scope_profile: String,
        #[arg(long)]
        scope_name: Option<String>,
        #[arg(long)]
        scope_ref: Option<String>,
        #[arg(long)]
        producer: String,
        #[arg(long)]
        consumer: String,
        #[arg(long)]
        sequence: u64,
        #[arg(long)]
        intent: String,
        #[arg(long)]
        payload_ref: String,
        #[arg(long = "policy-ref")]
        policy_refs: Vec<String>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
        #[arg(long)]
        semantic_result_ref: Option<String>,
        #[arg(long, default_value = "deny")]
        gap_policy: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        root: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

pub(crate) fn run_delivery_command(command: DeliveryCommand) -> Result<()> {
    match command {
        DeliveryCommand::Scope {
            scope_profile,
            scope_name,
            retention_refs,
            out,
        } => {
            let value = delivery_idempotency::scope_profile_value(&scope_profile, &scope_name, &retention_refs)?;
            let reference = canonical_hash(&value)?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!("delivery scope ref={reference} profile={scope_profile} name={scope_name}"),
            );
            Ok(())
        }
        DeliveryCommand::OperationId {
            scope_profile,
            scope_name,
            scope_ref,
            producer,
            consumer,
            sequence,
            intent,
            payload_ref,
            policy_refs,
            out,
        } => {
            let resolved_scope_ref =
                resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
            let operation = delivery_idempotency::derive_operation_id(delivery_idempotency::OperationIdInput {
                scope_ref: resolved_scope_ref,
                producer,
                consumer,
                sequence,
                intent,
                payload_ref,
                policy_refs,
            })?;
            let is_written_to_file = write_optional_preserves(out.as_ref(), &operation.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "delivery operation ref={} scope={} sequence={} intent={}",
                    operation.operation_ref, operation.scope_ref, operation.sequence, operation.intent
                ),
            );
            Ok(())
        }
        DeliveryCommand::Check {
            root,
            scope_profile,
            scope_name,
            scope_ref,
            producer,
            consumer,
            sequence,
            intent,
            payload_ref,
            policy_refs,
            evidence_refs,
            semantic_result_ref,
            gap_policy,
            receipt_out,
        } => {
            let resolved_scope_ref =
                resolve_delivery_scope_ref(&scope_profile, scope_name.as_deref(), scope_ref.as_deref())?;
            let delivery = delivery_idempotency::check_delivery(delivery_idempotency::DeliveryCheckInput {
                root: &root,
                scope_profile: &scope_profile,
                scope_ref: &resolved_scope_ref,
                producer: &producer,
                consumer: &consumer,
                sequence,
                intent: &intent,
                payload_ref: &payload_ref,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
                semantic_result_ref: semantic_result_ref.as_deref(),
                gap_policy: parse_delivery_gap_policy(&gap_policy)?,
            })?;
            let is_written_to_file = write_optional_preserves(receipt_out.as_ref(), &delivery.receipt.value)?;
            print_or_log_summary(
                is_written_to_file,
                &format!(
                    "delivery idempotency decision={} operation={} receipt={} side_effect={} prior={}",
                    delivery.receipt.decision,
                    delivery.operation.operation_ref,
                    delivery.receipt.receipt_ref,
                    delivery.receipt.side_effect,
                    delivery.prior_semantic_result_ref.as_deref().unwrap_or("none")
                ),
            );
            Ok(())
        }
        DeliveryCommand::ReceiptShow { receipt_ref, root } => {
            let value = delivery_idempotency::read_idempotency_receipt(&root, &receipt_ref)?;
            println!("{}", delivery_idempotency::delivery_summary(&value)?);
            Ok(())
        }
        DeliveryCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", delivery_idempotency::delivery_summary(&value)?);
            Ok(())
        }
    }
}

fn resolve_delivery_scope_ref(
    scope_profile: &str,
    scope_name: Option<&str>,
    scope_ref: Option<&str>,
) -> Result<String> {
    match (scope_name, scope_ref) {
        (_, Some(reference)) => Ok(reference.to_string()),
        (Some(name), None) => delivery_idempotency::scope_ref(scope_profile, name),
        (None, None) => Err(MoltenError::invalid_harness("delivery command requires --scope-ref or --scope-name")),
    }
}

fn parse_delivery_gap_policy(value: &str) -> Result<delivery_idempotency::GapPolicy> {
    match value {
        "deny" => Ok(delivery_idempotency::GapPolicy::Deny),
        "retry" => Ok(delivery_idempotency::GapPolicy::Retry),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported delivery gap policy {other}; expected deny or retry"
        ))),
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
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

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
