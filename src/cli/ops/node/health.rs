pub(crate) fn shutdown(input: super::command::health::Shutdown) -> molten::error::Result<()> {
    let super::command::health::Shutdown {
        startup,
        adapters,
        drained_jobs,
        index_receipt_refs,
        receipt_out,
    } = input;
    let adapter_receipts = parse_adapter_receipt_args(&adapters)?;
    let receipt =
        molten::node_runtime::node_shutdown_receipt_value(&molten::node_runtime::ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &startup,
            adapter_receipts: &adapter_receipts,
            drained_job_refs: &drained_jobs,
            index_receipt_refs: &index_receipt_refs,
            diagnostics: &[],
        })?;
    super::core::emit_named_receipt(receipt_out.as_ref(), "node shutdown receipt", &receipt)?;
    println!("node shutdown receipt={}", molten::preserves_rail::canonical_hash(&receipt)?);
    Ok(())
}

pub(crate) fn restart(input: super::command::health::Restart) -> molten::error::Result<()> {
    let super::command::health::Restart {
        startup_receipt,
        shutdown,
        index_receipt_refs,
        head_refs,
        open_job_refs,
        receipt_out,
    } = input;
    let startup_value = super::core::read_preserves_file(&startup_receipt)?;
    let startup = molten::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let receipt = molten::node_runtime::node_restart_health_receipt_value(
        &molten::node_runtime::RestartHealthReceiptValueInput {
            startup_receipt: &startup,
            shutdown_receipt_ref: shutdown.as_deref(),
            index_receipt_refs: &index_receipt_refs,
            head_refs: &head_refs,
            open_job_refs: &open_job_refs,
            diagnostics: &[],
        },
    )?;
    super::core::emit_named_receipt(receipt_out.as_ref(), "node health receipt", &receipt)?;
    println!("node health receipt={}", molten::preserves_rail::canonical_hash(&receipt)?);
    Ok(())
}

fn parse_adapter_receipt_args(
    args: &[String],
) -> molten::error::Result<Vec<molten::node_runtime::NodeAdapterReceiptRef>> {
    args.iter()
        .map(|arg| {
            let (name, receipt_ref) = arg.split_once('=').ok_or_else(|| {
                molten::error::MoltenError::invalid_harness(format!(
                    "node adapter receipt arg `{arg}` must be name=blake3:ref"
                ))
            })?;
            Ok(molten::node_runtime::NodeAdapterReceiptRef {
                name: name.to_string(),
                receipt_ref: receipt_ref.to_string(),
            })
        })
        .collect()
}
