pub struct ReadbackSource {
    pub(super) binding_ref: String,
    pub(super) validation_ref: String,
    pub(super) binding: crate::core_api::startup_binding::Binding,
    pub(super) observed_subject: super::Subject,
    pub(super) observed_receipts: std::collections::BTreeMap<String, String>,
}

pub(super) struct Current {
    pub source: ReadbackSource,
    pub validation_value: preserves::IOValue,
}

// The report shell reuses signed-output verification, then asks that exact
// admitted executable for its compiled facts. The report tool's own Cargo
// features, caller JSON and receipt metadata cannot substitute for those facts.
// r[impl molten.testing.capability_qualification.exact_evidence]
pub fn readback(
    root: &crate::node_state::DirectoryRoot,
    root_path: &std::path::Path,
) -> crate::error::Result<ReadbackSource> {
    let validated = crate::node_daemon::verify_selected_node_output_with_root(root)?;
    let (binding, output) = selected(root, &validated.binding_ref)?;
    let observed = describe(&output, root_path)?;
    if observed.binding_ref != validated.binding_ref
        || observed.validation_ref != validated.expected_startup_validation_ref
        || observed.subject.source_ref != binding.source_ref
        || observed.subject.build_ref != binding.executable_ref
        || observed.subject.dependency_closure_ref != binding.dependencies_ref
    {
        return Err(super::invalid("admitted executable readback differs from selected source/build/closure"));
    }
    Ok(ReadbackSource {
        binding_ref: observed.binding_ref,
        validation_ref: observed.validation_ref,
        binding,
        observed_subject: observed.subject,
        observed_receipts: observed.receipt_refs,
    })
}

pub(super) fn current(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Current> {
    let validated = crate::node_daemon::gate_validation_with_root(root)?;
    let (binding, _) = selected(root, &validated.binding_ref)?;
    let observed_subject = super::model::subject(&binding)?;
    Ok(Current {
        source: ReadbackSource {
            binding_ref: validated.binding_ref,
            validation_ref: validated.validation_ref,
            binding,
            observed_subject,
            observed_receipts: std::collections::BTreeMap::new(),
        },
        validation_value: validated.validation_value,
    })
}

fn selected(
    root: &crate::node_state::DirectoryRoot,
    binding_ref: &str,
) -> crate::error::Result<(crate::core_api::startup_binding::Binding, std::path::PathBuf)> {
    let trust: serde_json::Value =
        serde_json::from_slice(&root.read(&crate::node_state::RelativePath::parse("source-gate-trust.json")?, 16_384)?)
            .map_err(|_| super::invalid("invalid independently provisioned source trust"))?;
    let path = trust["output"].as_str().ok_or_else(|| super::invalid("source trust has no selected output"))?;
    let output = std::path::PathBuf::from(path);
    let selected = crate::node_state::DirectoryRoot::open_existing(&output)?;
    let bytes = selected.read(&crate::node_state::RelativePath::parse("binding.json")?, 65_536)?;
    if crate::preserves_rail::content_ref_from_bytes(&bytes) != binding_ref {
        return Err(super::invalid("signed binding changed after admission"));
    }
    let binding = serde_json::from_slice(&bytes).map_err(|_| super::invalid("invalid admitted binding"))?;
    Ok((binding, output))
}

fn describe(
    output: &std::path::Path,
    root_path: &std::path::Path,
) -> crate::error::Result<super::model::ObservedReadback> {
    let root_path = std::fs::canonicalize(root_path).map_err(crate::error::Failure::from)?;
    let policy = bounded_exec::OutcomePolicy::new(vec![0], true, true)
        .map_err(|_| super::invalid("invalid subject readback outcome policy"))?;
    let result = bounded_exec::run(bounded_exec::RunRequest {
        command: bounded_exec::CommandSpec {
            program: output.join("bin/molten"),
            args: vec![
                "evidence".into(),
                "qualify".into(),
                "--source-root".into(),
                root_path.into_os_string(),
                "--scenario".into(),
                "readback".into(),
                "--out".into(),
                "-".into(),
            ],
            current_dir: std::path::PathBuf::from("/"),
            environment_mode: bounded_exec::EnvironmentMode::Clear,
            environment: Vec::new(),
            input: bounded_exec::Input::Null,
        },
        limits: bounded_exec::ExecutionLimits {
            timeout_ms: 600_000,
            stdin_max_bytes: 1,
            stdout_max_bytes: 65_536,
            stderr_max_bytes: 65_536,
            poll_interval_ms: 10,
            teardown_timeout_ms: 1000,
        },
        termination_scope: bounded_exec::TerminationScope::ProcessGroup,
        outcome_policy: policy,
    })
    .map_err(|_| super::invalid("admitted subject readback process failed"))?;
    if result.disposition != bounded_exec::Disposition::Succeeded {
        return Err(super::invalid("admitted subject readback denied or unavailable"));
    }
    serde_json::from_slice(&result.stdout.bytes).map_err(|_| super::invalid("invalid native subject readback"))
}
