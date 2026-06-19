type Command = super::CatalogCommand;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::List {
            registry,
            ledger,
            kind,
            hidden_refs,
            receipt_out,
        } => {
            let result = molten::catalog::list(&registry, ledger.as_deref(), &molten::catalog::CatalogListInput {
                kind,
                visibility: catalog_visibility(hidden_refs),
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            super::io::print_catalog_items(&result.items)?;
            eprintln!("catalog list items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        Command::View {
            reference,
            registry,
            ledger,
            payload_inclusion_enabled,
            redaction_enabled,
            hidden_refs,
            receipt_out,
        } => {
            let result = molten::catalog::view(&registry, ledger.as_deref(), &molten::catalog::CatalogViewInput {
                reference,
                include_payload: payload_inclusion_enabled,
                redacted: redaction_enabled,
                visibility: catalog_visibility(hidden_refs),
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            super::io::print_catalog_items(&result.items)?;
            Ok(())
        }
        Command::Search {
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
            let filters = super::filter::filters(super::filter::Input {
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
            let result = molten::catalog::search(&registry, ledger.as_deref(), &molten::catalog::CatalogSearchInput {
                root_refs,
                include_dependencies: dependency_inclusion_enabled,
                include_dependents: dependent_inclusion_enabled,
                filters,
                visibility: catalog_visibility(hidden_refs),
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            super::io::print_catalog_items(&result.items)?;
            eprintln!("catalog search items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        Command::Deps {
            reference,
            registry,
            ledger,
            transitive,
            hidden_refs,
            receipt_out,
        } => {
            let result =
                molten::catalog::dependencies(&registry, ledger.as_deref(), &molten::catalog::CatalogGraphInput {
                    reference,
                    transitive,
                    visibility: catalog_visibility(hidden_refs),
                })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            super::io::print_catalog_items(&result.items)?;
            Ok(())
        }
        Command::Dependents {
            reference,
            registry,
            ledger,
            transitive,
            hidden_refs,
            receipt_out,
        } => {
            let result =
                molten::catalog::dependents(&registry, ledger.as_deref(), &molten::catalog::CatalogGraphInput {
                    reference,
                    transitive,
                    visibility: catalog_visibility(hidden_refs),
                })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            super::io::print_catalog_items(&result.items)?;
            Ok(())
        }
        Command::ShortId {
            prefix,
            registry,
            ledger,
            min_length,
            hidden_refs,
            receipt_out,
        } => {
            let resolution = molten::catalog::resolve_short_id(
                &registry,
                ledger.as_deref(),
                &molten::catalog::CatalogShortIdInput {
                    prefix,
                    min_length,
                    visibility: catalog_visibility(hidden_refs),
                },
            )?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &resolution.receipt_value)?;
            println!("{}", molten::preserves_rail::to_text(&resolution.value)?);
            if let Some(full_ref) = resolution.full_ref.as_ref() {
                eprintln!("catalog short-id {} -> {}", resolution.prefix, full_ref);
            } else {
                eprintln!("catalog short-id {} decision={}", resolution.prefix, resolution.decision);
            }
            Ok(())
        }
        Command::Chunks {
            chunks,
            hidden_refs,
            receipt_out,
        } => {
            let result = molten::catalog::chunk_store(&chunks, &molten::catalog::CatalogChunkStoreInput {
                visibility: catalog_visibility(hidden_refs),
            })?;
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
            super::io::print_catalog_items(&result.items)?;
            eprintln!("catalog chunks items={} result={}", result.items.len(), result.result_ref);
            Ok(())
        }
        Command::McpCall {
            request,
            registry,
            ledger,
            chunks,
            out,
            receipt_out,
        } => {
            let request_value = super::io::read_preserves_file(&request)?;
            let call = molten::catalog_mcp::call_with_chunk_store(
                &registry,
                ledger.as_deref(),
                chunks.as_deref(),
                &request_value,
            )?;
            if let Some(path) = out.as_ref() {
                super::io::write_file(path, &molten::preserves_rail::to_text(&call.response_value)?)?;
            } else {
                println!("{}", molten::preserves_rail::to_text(&call.response_value)?);
            }
            super::io::emit_named_receipt(receipt_out.as_ref(), "catalog MCP receipt", &call.receipt_value)?;
            eprintln!("catalog MCP call decision={} response={}", call.decision, call.response_ref);
            Ok(())
        }
        Command::Show { artifact } => {
            let value = super::io::read_preserves_file(&artifact)?;
            match molten::catalog::catalog_summary(&value) {
                Ok(summary) => println!("{summary}"),
                Err(_) => println!("{}", molten::catalog_mcp::catalog_mcp_summary(&value)?),
            }
            Ok(())
        }
    }
}

fn catalog_visibility(hidden_refs: Vec<String>) -> molten::catalog::CatalogVisibilityInput {
    molten::catalog::CatalogVisibilityInput {
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        hidden_refs,
        redaction_profile_ref: None,
    }
}
