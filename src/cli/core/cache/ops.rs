pub(super) fn put(args: super::command::Put) -> molten::error::Result<()> {
    let super::command::Put {
        input,
        cache,
        output,
        operation,
        version,
        dependencies,
        dependency_closure_hash,
        handler_profile_ref,
        policy_refs,
        capability_refs,
        revocation_refs,
        tool_ref,
        tool_version,
        mut assumption_refs,
        tier,
        status,
        evidence_refs,
        diagnostics,
        key_out,
        value_out,
        receipt_out,
    } = args;
    let input_value = super::io::read_preserves_file(&input)?;
    let output_value = output.as_ref().map(|path| super::io::read_preserves_file(path)).transpose()?;
    let tool_ref = match tool_ref {
        Some(tool_ref) => tool_ref,
        None => super::io::cli_cache_ref("tool", &operation)?,
    };
    if matches!(status.as_str(), molten::eval_cache::STATUS_DENY | molten::eval_cache::STATUS_ERROR) {
        for evidence_ref in &evidence_refs {
            if !assumption_refs.contains(evidence_ref)
                && !policy_refs.contains(evidence_ref)
                && !capability_refs.contains(evidence_ref)
                && !revocation_refs.contains(evidence_ref)
            {
                assumption_refs.push(evidence_ref.clone());
            }
        }
    }
    let closure_hash = match dependency_closure_hash {
        Some(hash) => hash,
        None => {
            molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("eval-cache-cli-closure", vec![
                molten::preserves_rail::string(&operation),
                super::io::preserves_sequence_strings(&dependencies),
            ]))?
        }
    };
    let key_input = molten::eval_cache::EvalCacheKeyInput {
        operation: operation.clone(),
        version,
        input_ref: molten::preserves_rail::canonical_hash(&input_value)?,
        dependency_closure_hash: closure_hash,
        dependency_refs: dependencies,
        handler_profile_ref,
        policy_refs: policy_refs.clone(),
        capability_refs,
        revocation_refs,
        tool_ref,
        tool_version,
        assumption_refs,
    };
    let value_input = molten::eval_cache::EvalCacheValueInput {
        tier,
        status,
        output: output_value,
        dependency_refs: key_input.dependency_refs.clone(),
        policy_refs,
        evidence_refs,
        diagnostics,
    };
    let put = molten::eval_cache::put(&cache, &key_input, &value_input)?;
    emit_put_result(&cache, key_out.as_ref(), value_out.as_ref(), receipt_out.as_ref(), &put)
}

fn emit_put_result(
    cache: &std::path::Path,
    key_out: Option<&std::path::PathBuf>,
    value_out: Option<&std::path::PathBuf>,
    receipt_out: Option<&std::path::PathBuf>,
    put: &molten::eval_cache::EvalCachePut,
) -> molten::error::Result<()> {
    if let Some(path) = key_out {
        super::io::write_file(path, &molten::preserves_rail::to_text(&put.key.value)?)?;
    }
    if let Some(path) = value_out {
        super::io::write_file(path, &molten::preserves_rail::to_text(&put.value.value)?)?;
    }
    super::io::emit_named_receipt(receipt_out, "eval cache receipt", &put.receipt_value)?;
    println!(
        "cache put ok key={} value={} operation={} tier={} status={} cache={}",
        put.key.key_ref,
        put.value.value_ref,
        put.key.operation,
        put.value.tier,
        put.value.status,
        cache.display()
    );
    Ok(())
}

pub(super) fn get(args: super::command::Get) -> molten::error::Result<()> {
    let get = molten::eval_cache::get(&args.cache, &args.key_ref, &molten::eval_cache::EvalCacheGetInput {
        current_policy_refs: args.current_policy_refs,
        current_capability_refs: args.current_capability_refs,
        current_revocation_refs: args.current_revocation_refs,
        semantic: args.semantic_enabled,
    })?;
    if let Some(output) = get.output.as_ref() {
        let text = molten::preserves_rail::to_text(output)?;
        if let Some(path) = args.out.as_ref() {
            super::io::write_file(path, &text)?;
        } else {
            println!("{text}");
        }
    } else if args.out.is_none() {
        println!("<none>");
    }
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "eval cache receipt", &get.receipt_value)?;
    eprintln!(
        "cache get ok key={} value={} status={} tier={} cache={}",
        get.key.key_ref,
        get.value.value_ref,
        get.value.status,
        get.value.tier,
        args.cache.display()
    );
    Ok(())
}

pub(super) fn status(args: super::command::Status) -> molten::error::Result<()> {
    let status = molten::eval_cache::status(&args.cache)?;
    println!(
        "keys={} values={} tombstones={} receipts={} tiers[pure={},simulated={},policy-current={},trace-only={}] statuses[pass={},deny={},error={},trace-only={}]",
        status.keys,
        status.values,
        status.tombstones,
        status.receipts,
        status.pure,
        status.simulated,
        status.policy_current,
        status.trace_only_tier,
        status.pass,
        status.deny,
        status.error,
        status.trace_only_status
    );
    Ok(())
}

pub(super) fn list(args: super::command::List) -> molten::error::Result<()> {
    for entry in molten::eval_cache::list(&args.cache, &molten::eval_cache::EvalCacheListFilter {
        operation: args.operation,
        tier: args.tier,
        status: args.status,
        dependency_ref: args.dependency_ref,
        policy_ref: args.policy_ref,
        capability_ref: args.capability_ref,
        revocation_ref: args.revocation_ref,
        evidence_ref: args.evidence_ref,
    })? {
        println!(
            "{} {} {} {} tombstoned={}",
            entry.key_ref, entry.value_ref, entry.operation, entry.status, entry.tombstoned
        );
    }
    Ok(())
}

pub(super) fn show(args: super::command::Show) -> molten::error::Result<()> {
    if let Ok(key) = molten::eval_cache::read_key(&args.cache, &args.reference) {
        println!("{}", molten::preserves_rail::to_text(&key.value)?);
        return Ok(());
    }
    for entry in molten::eval_cache::list(&args.cache, &molten::eval_cache::EvalCacheListFilter {
        operation: None,
        tier: None,
        status: None,
        dependency_ref: None,
        policy_ref: None,
        capability_ref: None,
        revocation_ref: None,
        evidence_ref: None,
    })? {
        if entry.value_ref == args.reference {
            let value = molten::eval_cache::read_value(&args.cache, &entry.key_ref)?;
            println!("{}", molten::preserves_rail::to_text(&value.value)?);
            return Ok(());
        }
    }
    let receipt = molten::eval_cache::read_receipt(&args.cache, &args.reference)?;
    println!("{}", molten::preserves_rail::to_text(&receipt.value)?);
    Ok(())
}

pub(super) fn invalidate(args: super::command::Invalidate) -> molten::error::Result<()> {
    let invalidated = molten::eval_cache::invalidate(&args.cache, &molten::eval_cache::EvalCacheInvalidateInput {
        key_ref: args.key_ref,
        dependency_ref: args.dependency_ref,
        policy_ref: args.policy_ref,
        capability_ref: args.capability_ref,
        revocation_ref: args.revocation_ref,
        operation: args.operation,
        reason: args.reason,
        retention_evidence: args.retention.into_retention_evidence(),
        apply_refs: args.apply_refs,
    })?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "eval cache receipt", &invalidated.receipt_value)?;
    for key_ref in &invalidated.invalidated_key_refs {
        println!("{key_ref}");
    }
    eprintln!(
        "cache invalidate ok decision={} keys={} retention_receipts={} cache={}",
        invalidated.decision,
        invalidated.invalidated_key_refs.len(),
        invalidated.retention_receipt_refs.len(),
        args.cache.display()
    );
    Ok(())
}

pub(super) fn index_rebuild(args: super::command::IndexRebuild) -> molten::error::Result<()> {
    let receipt = molten::eval_cache::rebuild_index(&args.cache)?;
    super::io::emit_named_receipt(args.receipt_out.as_ref(), "eval cache receipt", &receipt)?;
    println!("cache index-rebuild ok cache={}", args.cache.display());
    Ok(())
}
