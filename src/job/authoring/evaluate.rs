fn job_evaluator_identity(path: &FilePath) -> Result<String> {
    if !path.is_absolute() { return Err(MoltenError::invalid_harness("captured evaluator/limiter path must be absolute")); }
    let metadata = std::fs::metadata(path).map_err(MoltenError::from)?;
    if metadata.len() > 134_217_728 { return Err(MoltenError::invalid_harness("captured evaluator executable exceeds byte bound")); }
    let bytes = std::fs::read(path).map_err(MoltenError::from)?;
    Ok(crate::preserves_rail::content_ref_from_bytes(&bytes))
}

fn run_captured_nickel(input: &CapturedNickelJobInput<'_>, source: Vec<u8>, version: bool) -> Result<Vec<u8>> {
    if !input.workspace.is_absolute() { return Err(MoltenError::invalid_harness("captured workspace must be absolute")); }
    let mut args: Vec<std::ffi::OsString> = vec!["--as=536870912".into(), "--cpu=2".into(), "--".into(), input.nickel.as_os_str().into()];
    if version { args.push("--version".into()); }
    else { args.extend(["export".into(), "--format".into(), "json".into()]); }
    let request = bounded_exec::RunRequest {
        command: bounded_exec::CommandSpec {
            program: input.prlimit.to_path_buf(), args, current_dir: input.workspace.to_path_buf(),
            environment_mode: bounded_exec::EnvironmentMode::Clear, environment: Vec::new(),
            input: if version { bounded_exec::Input::Null } else { bounded_exec::Input::Bytes(source) },
        },
        limits: bounded_exec::ExecutionLimits {
            timeout_ms: 3_000, stdin_max_bytes: MAX_JOB_AUTHORING_BYTES,
            stdout_max_bytes: MAX_JOB_AUTHORING_BYTES, stderr_max_bytes: 4_096,
            poll_interval_ms: 10, teardown_timeout_ms: 1_000,
        },
        termination_scope: bounded_exec::TerminationScope::ProcessGroup,
        outcome_policy: bounded_exec::OutcomePolicy::new(vec![0], true, true)
            .map_err(|error| MoltenError::invalid_harness(format!("captured evaluator policy: {error:?}")))?,
    };
    let output = bounded_exec::run(request).map_err(|_| MoltenError::invalid_harness(format!("captured evaluator execution failed at {}; source locations unavailable", input.source_name)))?;
    if output.disposition != bounded_exec::Disposition::Succeeded {
        // Evaluator text may contain secrets. Logical source attribution is retained, text is not disclosed.
        return Err(MoltenError::invalid_harness(format!("captured evaluation/export rejected at {} ({:?}); source locations unavailable", input.source_name, output.disposition)));
    }
    Ok(output.stdout.bytes)
}

/// Evaluates only captured source, then admits the closed export through the normal parser.
/// Inspection/lowering never executes a workload stage.
// r[impl molten.job_dag.captured_authoring]
pub fn lower_captured_nickel_job(input: CapturedNickelJobInput<'_>) -> Result<LoweredNickelJob> {
    ensure_count_at_most(input.source_name.len(), 128, "captured logical source name")?;
    validate_node_id(input.source_name)?;
    captured_source_check(input.source)?;
    ensure_count_at_most(input.modules.len(), 64, "captured modules")?;
    let mut modules = OrderedMap::new();
    let mut source_bytes = input.source.len();
    for module in input.modules {
        ensure_count_at_most(module.key.len(), 128, "captured module key")?;
        validate_node_id(&module.key)?;
        captured_source_check(&module.source)?;
        source_bytes = checked_count_sum(source_bytes, module.source.len(), MAX_JOB_AUTHORING_BYTES, "captured source closure")?;
        if modules.insert(module.key.clone(), module.source.clone()).is_some() {
            return Err(MoltenError::invalid_harness("duplicate captured module key"));
        }
    }
    let parameters = job_nickel_literal(input.parameters, 0)?;
    ensure_count_at_most(parameters.len(), MAX_JOB_AUTHORING_BYTES, "captured parameter bytes")?;
    let mut definitions = Vec::with_capacity(modules.len());
    for (key, source) in &modules {
        let key = serde_json::to_string(key).map_err(|error| MoltenError::invalid_harness(error.to_string()))?;
        definitions.push(format!("{key} = ({source})"));
    }
    let source = format!("({}) {{{}}} ({parameters})", input.source, definitions.join(","));
    ensure_count_at_most(source.len(), MAX_JOB_AUTHORING_BYTES, "captured expanded evaluator input")?;
    let evaluator_ref = job_evaluator_identity(input.nickel)?;
    let limiter_ref = job_evaluator_identity(input.prlimit)?;
    if evaluator_ref != input.expected_evaluator_ref || limiter_ref != input.expected_limiter_ref {
        return Err(MoltenError::invalid_harness("captured evaluator/limiter differs from selected executable identity"));
    }
    let version = run_captured_nickel(&input, Vec::new(), true)?;
    let version = String::from_utf8(version).map_err(|_| MoltenError::invalid_harness("Nickel version is not UTF-8"))?;
    if version.trim() != "nickel-lang-cli nickel 1.17.0 (rev 1320a98)" {
        return Err(MoltenError::invalid_harness("captured authoring requires Nickel 1.17.0"));
    }
    let export = run_captured_nickel(&input, source.as_bytes().to_vec(), false)?;
    let export_text = String::from_utf8(export).map_err(|_| MoltenError::invalid_harness("Nickel export is not UTF-8"))?;
    if evaluator_ref != job_evaluator_identity(input.nickel)? || limiter_ref != job_evaluator_identity(input.prlimit)? {
        return Err(MoltenError::invalid_harness("captured evaluator/limiter changed during export"));
    }
    let mut lowered = lower_job_graph_export(&export_text, &source, &evaluator_ref, &limiter_ref, &modules)
        .map_err(|_| MoltenError::invalid_harness(format!("captured graph admission rejected at {}; inspect bounded instance/port origins; source locations unavailable", input.source_name)))?;
    lowered.provenance = crate::preserves_rail::record("job-nickel-capture-v1", vec![
        crate::preserves_rail::string(input.source_name), lowered.provenance,
    ]);
    Ok(lowered)
}
