type Chain = super::command::Chain;
type Command = super::command::Command;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run_ledger(command: Command) -> Outcome<()> {
    match command {
        Command::Import {
            artifact,
            ledger,
            receipt_out,
        } => {
            let artifact_value = super::io::read_preserves_file(&artifact)?;
            let imported = molten::ledger::import_artifact(&ledger, &artifact_value)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "ledger import receipt", &imported.receipt_value)?;
            println!(
                "ledger import ok artifact={} kind={} ledger={}",
                imported.artifact_ref,
                imported.artifact_kind,
                ledger.display()
            );
            Ok(())
        }
        Command::Export {
            artifact_ref,
            ledger,
            out,
            receipt_out,
        } => {
            let exported = molten::ledger::export_artifact(&ledger, &artifact_ref, &out)?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "ledger export receipt", &exported.receipt_value)?;
            println!(
                "ledger export ok artifact={} kind={} out={}",
                exported.artifact_ref,
                exported.artifact_kind,
                out.display()
            );
            Ok(())
        }
        Command::List { ledger } => {
            for entry in molten::ledger::list_artifacts(&ledger)? {
                println!("{} {}", entry.artifact_ref, entry.artifact_kind);
            }
            Ok(())
        }
        Command::Pin { artifact_ref, ledger } => {
            molten::ledger::pin_artifact(&ledger, &artifact_ref)?;
            println!("ledger pin ok artifact={} ledger={}", artifact_ref, ledger.display());
            Ok(())
        }
        Command::Gc {
            ledger,
            dry_run,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let retention_evidence = retention.into_retention_evidence();
            let gc = molten::ledger::gc(&ledger, molten::ledger::LedgerGcInput {
                dry_run,
                retention_evidence: &retention_evidence,
                apply_refs: &apply_refs,
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "ledger gc receipt", &gc.receipt_value)?;
            println!(
                "ledger gc ok decision={} dry_run={} removed={} retention_receipts={}",
                gc.decision,
                gc.dry_run,
                gc.removed_refs.len(),
                gc.retention_receipt_refs.len()
            );
            Ok(())
        }
    }
}

pub(super) fn run_chain(command: Chain) -> Outcome<()> {
    match command {
        Chain::Publish {
            ledger,
            iroh_store,
            scope,
            id,
            epoch,
            anchor,
            head,
            node,
            fork_policy,
            receipt_out,
        } => {
            let chain = molten::evidence_chain::ChainScope::new(scope, id, epoch);
            let policy = parse_policy(&fork_policy)?;
            let published =
                molten::iroh_exchange::publish_chain_segment(&molten::iroh_exchange::PublishChainSegmentInput {
                    iroh_root: &iroh_store,
                    ledger_root: &ledger,
                    chain: &chain,
                    anchor_ref: anchor.as_deref(),
                    expected_head: head.as_deref(),
                    node: &node,
                    fork_policy: policy,
                })?;
            super::io::emit_named_receipt(
                receipt_out.as_ref(),
                "iroh chain exchange receipt",
                &published.receipt_value,
            )?;
            println!(
                "chain publish ok ticket={} bundle={} chain={}/{}/{}",
                published.ticket,
                published.bundle_ref,
                published.chain.scope,
                published.chain.id,
                published.chain.epoch
            );
            Ok(())
        }
        Chain::Fetch {
            ticket,
            ledger,
            iroh_store,
            expected_bundle_ref,
            peer,
            fork_policy,
            receipt_out,
        } => {
            let policy = parse_policy(&fork_policy)?;
            let fetched = molten::iroh_exchange::fetch_chain_segment(&molten::iroh_exchange::FetchChainSegmentInput {
                iroh_root: &iroh_store,
                ticket: &ticket,
                expected_bundle_ref: expected_bundle_ref.as_deref(),
                peer: &peer,
                ledger_root: &ledger,
                fork_policy: policy,
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "iroh chain exchange receipt", &fetched.receipt_value)?;
            println!(
                "chain fetch ok ticket={} bundle={} chain={}/{}/{}",
                fetched.ticket, fetched.bundle_ref, fetched.chain.scope, fetched.chain.id, fetched.chain.epoch
            );
            Ok(())
        }
    }
}

fn parse_policy(value: &str) -> Outcome<molten::evidence_chain::ChainForkPolicy> {
    match value {
        "reject-unexpected-forks" | "production" | "reject" => {
            Ok(molten::evidence_chain::ChainForkPolicy::RejectUnexpectedForks)
        }
        "retain-fork-evidence" | "diagnostic" | "retain" => {
            Ok(molten::evidence_chain::ChainForkPolicy::RetainForkEvidence)
        }
        other => Err(molten::error::MoltenError::invalid_harness(format!(
            "unsupported chain fork policy {other}; expected reject-unexpected-forks or retain-fork-evidence"
        ))),
    }
}
