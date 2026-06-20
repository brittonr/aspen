type Command = super::CatalogCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        command @ Command::List { .. } => list(command),
        command @ Command::View { .. } => view(command),
        command @ Command::Search { .. } => search(command),
        command @ Command::Deps { .. } => deps(command),
        command @ Command::Dependents { .. } => dependents(command),
        command @ Command::ShortId { .. } => short_id(command),
        command @ Command::Chunks { .. } => chunks(command),
        command @ Command::McpCall { .. } => mcp_call(command),
        Command::Show { artifact } => show(artifact),
    }
}

fn list(command: Command) -> Outcome<()> {
    let Command::List {
        registry,
        ledger,
        kind,
        hidden_refs,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("list"));
    };
    let result = molten::catalog::list(&registry, ledger.as_deref(), &molten::catalog::CatalogListInput {
        kind,
        visibility: catalog_visibility(hidden_refs),
    })?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
    super::io::print_catalog_items(&result.items)?;
    eprintln!("catalog list items={} result={}", result.items.len(), result.result_ref);
    Ok(())
}

fn view(command: Command) -> Outcome<()> {
    let Command::View {
        reference,
        registry,
        ledger,
        payload_inclusion_enabled,
        redaction_enabled,
        hidden_refs,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("view"));
    };
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

fn search(command: Command) -> Outcome<()> {
    let Command::Search {
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
    } = command
    else {
        return Err(wrong_handler("search"));
    };
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

fn deps(command: Command) -> Outcome<()> {
    let Command::Deps {
        reference,
        registry,
        ledger,
        transitive,
        hidden_refs,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("deps"));
    };
    let result = molten::catalog::dependencies(&registry, ledger.as_deref(), &molten::catalog::CatalogGraphInput {
        reference,
        transitive,
        visibility: catalog_visibility(hidden_refs),
    })?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
    super::io::print_catalog_items(&result.items)?;
    Ok(())
}

fn dependents(command: Command) -> Outcome<()> {
    let Command::Dependents {
        reference,
        registry,
        ledger,
        transitive,
        hidden_refs,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("dependents"));
    };
    let result = molten::catalog::dependents(&registry, ledger.as_deref(), &molten::catalog::CatalogGraphInput {
        reference,
        transitive,
        visibility: catalog_visibility(hidden_refs),
    })?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
    super::io::print_catalog_items(&result.items)?;
    Ok(())
}

fn short_id(command: Command) -> Outcome<()> {
    let Command::ShortId {
        prefix,
        registry,
        ledger,
        min_length,
        hidden_refs,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("short-id"));
    };
    let resolution =
        molten::catalog::resolve_short_id(&registry, ledger.as_deref(), &molten::catalog::CatalogShortIdInput {
            prefix,
            min_length,
            visibility: catalog_visibility(hidden_refs),
        })?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &resolution.receipt_value)?;
    println!("{}", molten::preserves_rail::to_text(&resolution.value)?);
    if let Some(full_ref) = resolution.full_ref.as_ref() {
        eprintln!("catalog short-id {} -> {}", resolution.prefix, full_ref);
    } else {
        eprintln!("catalog short-id {} decision={}", resolution.prefix, resolution.decision);
    }
    Ok(())
}

fn chunks(command: Command) -> Outcome<()> {
    let Command::Chunks {
        chunks,
        hidden_refs,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("chunks"));
    };
    let result = molten::catalog::chunk_store(&chunks, &molten::catalog::CatalogChunkStoreInput {
        visibility: catalog_visibility(hidden_refs),
    })?;
    super::io::emit_named_receipt(receipt_out.as_ref(), "catalog receipt", &result.receipt_value)?;
    super::io::print_catalog_items(&result.items)?;
    eprintln!("catalog chunks items={} result={}", result.items.len(), result.result_ref);
    Ok(())
}

fn mcp_call(command: Command) -> Outcome<()> {
    let Command::McpCall {
        request,
        registry,
        ledger,
        chunks,
        out,
        receipt_out,
    } = command
    else {
        return Err(wrong_handler("mcp-call"));
    };
    let request_value = super::io::read_preserves_file(&request)?;
    let call =
        molten::catalog_mcp::call_with_chunk_store(&registry, ledger.as_deref(), chunks.as_deref(), &request_value)?;
    if let Some(path) = out.as_ref() {
        super::io::write_file(path, &molten::preserves_rail::to_text(&call.response_value)?)?;
    } else {
        println!("{}", molten::preserves_rail::to_text(&call.response_value)?);
    }
    super::io::emit_named_receipt(receipt_out.as_ref(), "catalog MCP receipt", &call.receipt_value)?;
    eprintln!("catalog MCP call decision={} response={}", call.decision, call.response_ref);
    Ok(())
}

fn show(artifact: FilePath) -> Outcome<()> {
    let value = super::io::read_preserves_file(&artifact)?;
    match molten::catalog::catalog_summary(&value) {
        Ok(summary) => println!("{summary}"),
        Err(_) => println!("{}", molten::catalog_mcp::catalog_mcp_summary(&value)?),
    }
    Ok(())
}

fn catalog_visibility(hidden_refs: Vec<String>) -> molten::catalog::CatalogVisibilityInput {
    molten::catalog::CatalogVisibilityInput {
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        hidden_refs,
        redaction_profile_ref: None,
    }
}

fn wrong_handler(name: &str) -> molten::error::MoltenError {
    molten::error::MoltenError::invalid_harness(format!("catalog {name} handler called with another command"))
}
