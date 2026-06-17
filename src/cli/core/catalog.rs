use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten::catalog;
use molten::catalog_mcp;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

#[path = "catalog/command.rs"]
mod command;
#[path = "catalog/filter.rs"]
mod filter;

pub(crate) type CatalogCommand = command::Command;

pub(crate) fn run_catalog_command(command: CatalogCommand) -> Result<()> {
    match command {
        CatalogCommand::List {
            registry,
            ledger,
            kind,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::list(&registry, ledger.as_deref(), &catalog::CatalogListInput {
                kind,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            eprintln!("catalog list items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        CatalogCommand::View {
            reference,
            registry,
            ledger,
            payload_inclusion_enabled,
            redaction_enabled,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::view(&registry, ledger.as_deref(), &catalog::CatalogViewInput {
                reference,
                include_payload: payload_inclusion_enabled,
                redacted: redaction_enabled,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            Ok(())
        }
        CatalogCommand::Search {
            registry,
            ledger,
            artifact_kind,
            ledger_kind,
            schema_ref,
            structural_fingerprint,
            effect_ref,
            policy_ref,
            capability_ref,
            evidence_ref,
            dependency_ref,
            dependent_ref,
            receipt_operation,
            receipt_decision,
            transcript_status,
            upgrade_status,
            text,
            root_refs,
            dependency_inclusion_enabled,
            dependent_inclusion_enabled,
            hidden_refs,
            receipt_out,
        } => {
            let filters = filter::filters(filter::Input {
                artifact_kind,
                ledger_kind,
                schema_ref,
                structural_fingerprint,
                effect_ref,
                policy_ref,
                capability_ref,
                evidence_ref,
                dependency_ref,
                dependent_ref,
                receipt_operation,
                receipt_decision,
                transcript_status,
                upgrade_status,
                text,
            });
            let result = catalog::search(&registry, ledger.as_deref(), &catalog::CatalogSearchInput {
                root_refs,
                include_dependencies: dependency_inclusion_enabled,
                include_dependents: dependent_inclusion_enabled,
                filters,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            eprintln!("catalog search items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        CatalogCommand::Deps {
            reference,
            registry,
            ledger,
            transitive,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::dependencies(&registry, ledger.as_deref(), &catalog::CatalogGraphInput {
                reference,
                transitive,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            Ok(())
        }
        CatalogCommand::Dependents {
            reference,
            registry,
            ledger,
            transitive,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::dependents(&registry, ledger.as_deref(), &catalog::CatalogGraphInput {
                reference,
                transitive,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            Ok(())
        }
        CatalogCommand::ShortId {
            prefix,
            registry,
            ledger,
            min_length,
            hidden_refs,
            receipt_out,
        } => {
            let resolution = catalog::resolve_short_id(&registry, ledger.as_deref(), &catalog::CatalogShortIdInput {
                prefix,
                min_length,
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &resolution.receipt_value)?;
            println!("{}", to_text(&resolution.value)?);
            if let Some(full_ref) = resolution.full_ref.as_ref() {
                eprintln!("catalog short-id {} -> {}", resolution.prefix, full_ref);
            } else {
                eprintln!("catalog short-id {} decision={}", resolution.prefix, resolution.decision);
            }
            Ok(())
        }
        CatalogCommand::Chunks {
            chunks,
            hidden_refs,
            receipt_out,
        } => {
            let result = catalog::chunk_store(&chunks, &catalog::CatalogChunkStoreInput {
                visibility: catalog_visibility(hidden_refs),
            })?;
            emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            print_catalog_items(&result.items)?;
            eprintln!("catalog chunks items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        CatalogCommand::McpCall {
            request,
            registry,
            ledger,
            chunks,
            out,
            receipt_out,
        } => {
            let request_value = read_preserves_file(&request)?;
            let call =
                catalog_mcp::call_with_chunk_store(&registry, ledger.as_deref(), chunks.as_deref(), &request_value)?;
            if let Some(path) = out.as_ref() {
                write_file(path, &to_text(&call.response_value)?)?;
            } else {
                println!("{}", to_text(&call.response_value)?);
            }
            emit_named_receipt(receipt_out.as_ref(), "catalog MCP receipt", &call.receipt_value)?;
            eprintln!("catalog MCP call decision={} response={}", call.decision, call.response_ref);
            Ok(())
        }
        CatalogCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            match catalog::catalog_summary(&value) {
                Ok(summary) => println!("{summary}"),
                Err(_) => println!("{}", catalog_mcp::catalog_mcp_summary(&value)?),
            }
            Ok(())
        }
    }
}

fn catalog_visibility(hidden_refs: Vec<String>) -> catalog::CatalogVisibilityInput {
    catalog::CatalogVisibilityInput {
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        hidden_refs,
        redaction_profile_ref: None,
    }
}

fn print_catalog_items(items: &[preserves::IOValue]) -> Result<()> {
    for item in items {
        println!("{}", to_text(item)?);
    }
    Ok(())
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
