type CachePutArgs = super::super::command::Put;
type CacheResult<T> = molten::error::Result<T>;
type EvalCacheKeyInput = molten::eval_cache::KeyInput;
type EvalCachePut = molten::eval_cache::Put;
type EvalCacheValueInput = molten::eval_cache::ValueInput;
type IoValue = preserves::IOValue;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;

pub(super) fn put(args: CachePutArgs) -> CacheResult<()> {
    let prepared = prepare_put(args)?;
    let put = molten::eval_cache::put(&prepared.cache, &prepared.key_input, &prepared.value_input)?;
    emit_put_result(
        &prepared.cache,
        prepared.key_out.as_ref(),
        prepared.value_out.as_ref(),
        prepared.receipt_out.as_ref(),
        &put,
    )
}

struct PreparedPut {
    cache: PathBuf,
    key_out: Option<PathBuf>,
    value_out: Option<PathBuf>,
    receipt_out: Option<PathBuf>,
    key_input: EvalCacheKeyInput,
    value_input: EvalCacheValueInput,
}

struct PutKeyParts {
    operation: String,
    version: String,
    input_ref: String,
    dependency_closure_hash: String,
    dependency_refs: Vec<String>,
    handler_profile_ref: Option<String>,
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    revocation_refs: Vec<String>,
    evidence_refs: Vec<String>,
    tool_ref: String,
    tool_version: String,
    assumption_refs: Vec<String>,
}

struct PutValueParts {
    tier: String,
    status: String,
    output: Option<IoValue>,
    dependency_refs: Vec<String>,
    policy_refs: Vec<String>,
    evidence_refs: Vec<String>,
    diagnostics: Vec<String>,
}

fn prepare_put(args: CachePutArgs) -> CacheResult<PreparedPut> {
    let input_value = super::super::io::read_preserves_file(&args.input)?;
    let output_value = args.output.as_ref().map(|path| super::super::io::read_preserves_file(path)).transpose()?;
    let tool_ref = resolve_tool_ref(args.tool_ref, &args.operation)?;
    let assumption_refs = extend_denial_assumptions(args.assumption_refs, &args.status, GuardRefs {
        evidence: &args.evidence_refs,
        policy: &args.policy_refs,
        capability: &args.capability_refs,
        revocation: &args.revocation_refs,
    });
    let dependency_closure_hash =
        resolve_closure_hash(args.dependency_closure_hash, &args.operation, &args.dependencies)?;
    let key_input = put_key_input(PutKeyParts {
        operation: args.operation,
        version: args.version,
        input_ref: molten::preserves_rail::canonical_hash(&input_value)?,
        dependency_closure_hash,
        dependency_refs: args.dependencies,
        handler_profile_ref: args.handler_profile_ref,
        policy_refs: args.policy_refs.clone(),
        capability_refs: args.capability_refs,
        revocation_refs: args.revocation_refs,
        evidence_refs: args.evidence_refs.clone(),
        tool_ref,
        tool_version: args.tool_version,
        assumption_refs,
    });
    let value_input = put_value_input(PutValueParts {
        tier: args.tier,
        status: args.status,
        output: output_value,
        dependency_refs: key_input.dependency_refs.clone(),
        policy_refs: args.policy_refs,
        evidence_refs: args.evidence_refs,
        diagnostics: args.diagnostics,
    });
    Ok(PreparedPut {
        cache: args.cache,
        key_out: args.key_out,
        value_out: args.value_out,
        receipt_out: args.receipt_out,
        key_input,
        value_input,
    })
}

fn put_key_input(parts: PutKeyParts) -> EvalCacheKeyInput {
    EvalCacheKeyInput {
        operation: parts.operation,
        version: parts.version,
        input_ref: parts.input_ref,
        artifact_refs: Vec::new(),
        input_refs: Vec::new(),
        dependency_closure_hash: parts.dependency_closure_hash,
        dependency_refs: parts.dependency_refs,
        schema_refs: Vec::new(),
        handler_profile_ref: parts.handler_profile_ref,
        policy_refs: parts.policy_refs,
        policy_export_refs: Vec::new(),
        capability_refs: parts.capability_refs,
        revocation_refs: parts.revocation_refs,
        resource_refs: Vec::new(),
        effect_manifest_refs: Vec::new(),
        provenance_refs: Vec::new(),
        source_gate_refs: Vec::new(),
        evidence_refs: parts.evidence_refs,
        retention_refs: Vec::new(),
        compatibility_refs: Vec::new(),
        tool_ref: parts.tool_ref,
        tool_version: parts.tool_version,
        assumption_refs: parts.assumption_refs,
    }
}

fn put_value_input(parts: PutValueParts) -> EvalCacheValueInput {
    EvalCacheValueInput {
        tier: parts.tier,
        status: parts.status,
        output: parts.output,
        dependency_refs: parts.dependency_refs,
        policy_refs: parts.policy_refs,
        evidence_refs: parts.evidence_refs,
        diagnostics: parts.diagnostics,
    }
}

struct GuardRefs<'a> {
    evidence: &'a [String],
    policy: &'a [String],
    capability: &'a [String],
    revocation: &'a [String],
}

fn resolve_tool_ref(tool_ref: Option<String>, operation: &str) -> CacheResult<String> {
    match tool_ref {
        Some(tool_ref) => Ok(tool_ref),
        None => super::super::io::local_ref("tool", operation),
    }
}

fn extend_denial_assumptions(mut assumption_refs: Vec<String>, status: &str, refs: GuardRefs<'_>) -> Vec<String> {
    if !matches!(status, molten::eval_cache::STATUS_DENY | molten::eval_cache::STATUS_ERROR) {
        return assumption_refs;
    }
    for evidence_ref in refs.evidence {
        if !assumption_refs.contains(evidence_ref)
            && !refs.policy.contains(evidence_ref)
            && !refs.capability.contains(evidence_ref)
            && !refs.revocation.contains(evidence_ref)
        {
            assumption_refs.push(evidence_ref.clone());
        }
    }
    assumption_refs
}

fn resolve_closure_hash(
    dependency_closure_hash: Option<String>,
    operation: &str,
    dependencies: &[String],
) -> CacheResult<String> {
    match dependency_closure_hash {
        Some(hash) => Ok(hash),
        None => {
            molten::preserves_rail::canonical_hash(&molten::preserves_rail::record("eval-cache-cli-closure", vec![
                molten::preserves_rail::string(operation),
                super::super::io::preserves_sequence_strings(dependencies),
            ]))
        }
    }
}

fn emit_put_result(
    cache: &Path,
    key_out: Option<&PathBuf>,
    value_out: Option<&PathBuf>,
    receipt_out: Option<&PathBuf>,
    put: &EvalCachePut,
) -> CacheResult<()> {
    if let Some(path) = key_out {
        super::super::io::write_file(path, &molten::preserves_rail::to_text(&put.key.value)?)?;
    }
    if let Some(path) = value_out {
        super::super::io::write_file(path, &molten::preserves_rail::to_text(&put.value.value)?)?;
    }
    super::super::io::emit_named_receipt(receipt_out, "eval cache receipt", &put.receipt_value)?;
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
