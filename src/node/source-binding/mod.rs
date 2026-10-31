//! Capability-rooted normal-startup evidence. The trust file is operator authority, not evidence.
// r[impl molten.audit_f02.real_evidence]
// r[impl molten.startup_binding.refresh]

mod closure;
mod measure;
mod process;
#[cfg(test)]
mod tests;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Trust {
    schema: String,
    nix: std::path::PathBuf,
    output: std::path::PathBuf,
    public_key: String,
    selection: molten_core::startup_binding::Selection,
}

/// Evidence from actual source admission, not an activation or authority credential.
pub struct GateValidation {
    pub binding_ref: String,
    pub validation_ref: String,
    pub validation_value: preserves::IOValue,
}

pub fn gate_validation_with_root(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<GateValidation> {
    let source = load(root)?;
    Ok(GateValidation {
        binding_ref: source.binding_ref,
        validation_ref: source.validation_ref,
        validation_value: source.validation_value,
    })
}

/// Authoring-only verification. This cannot construct a current-process startup token.
pub struct SelectedNodeOutputValidation {
    pub binding_ref: String,
    pub expected_startup_validation_ref: String,
    pub selected_output_validation_ref: String,
    pub selected_output_validation_value: preserves::IOValue,
}

pub fn verify_selected_node_output_with_root(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<SelectedNodeOutputValidation> {
    let selected = load_measured(root, Measurement::SelectedOutput)?;
    let value = crate::preserves_rail::record("node-selected-source-binding-validation-v1", vec![
        crate::preserves_rail::string("selected-output-only-no-current-process-admission"),
        crate::preserves_rail::string(&selected.binding_ref),
        crate::preserves_rail::string(&selected.validation_ref),
        crate::preserves_rail::string(&selected.gate_ref),
    ]);
    Ok(SelectedNodeOutputValidation {
        binding_ref: selected.binding_ref,
        expected_startup_validation_ref: selected.validation_ref,
        selected_output_validation_ref: crate::preserves_rail::canonical_hash(&value)?,
        selected_output_validation_value: value,
    })
}

enum Measurement {
    CurrentProcess,
    SelectedOutput,
}

pub(super) struct Accepted {
    pub(super) binding_ref: String,
    pub(super) gate_ref: String,
    pub(super) gate_value: preserves::IOValue,
    pub(super) validation_ref: String,
    pub(super) validation_value: preserves::IOValue,
}

/// Composes a pure runtime start only after the same authenticated source gate as the daemon.
pub fn start_runtime_with_root(
    root: &crate::node_state::DirectoryRoot,
    mut input: crate::node_runtime::NodeRuntimeStartInput,
) -> crate::error::Result<crate::node_runtime::NodeRuntimeStart> {
    let source = load(root)?;
    let gate = crate::kernel::ValidatedNodeSourceGate {
        binding_ref: source.binding_ref.clone(),
        gate_ref: source.gate_ref.clone(),
        validation_ref: source.validation_ref,
    };
    input.source_gate_receipt_refs = vec![source.gate_ref];
    input.source_gate_receipt_values = vec![source.gate_value];
    input.version_refs = vec![source.binding_ref];
    crate::node_runtime::start_node_runtime_with_source_gate(&input, &gate)
}

pub(super) fn load(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<Accepted> {
    load_measured(root, Measurement::CurrentProcess)
}

fn load_measured(root: &crate::node_state::DirectoryRoot, measurement: Measurement) -> crate::error::Result<Accepted> {
    let path = crate::node_state::RelativePath::parse("source-gate-trust.json")?;
    let trust: Trust = serde_json::from_slice(&root.read(&path, 16_384)?)
        .map_err(|error| invalid(format!("source trust selection: {error}")))?;
    validate_trust(&trust)?;
    process::authenticate(&trust)?;
    let output = cap_std::fs::Dir::open_ambient_dir(&trust.output, cap_std::ambient_authority())
        .map_err(crate::error::Failure::from)?;
    let binding_bytes = measure::read(&output, "binding.json", 65_536)?;
    let binding: molten_core::startup_binding::Binding =
        serde_json::from_slice(&binding_bytes).map_err(|error| invalid(format!("source binding: {error}")))?;
    validate_retained_inputs(&output, &binding)?;
    let binding_ref = crate::preserves_rail::content_ref_from_bytes(&binding_bytes);
    let executing_ref = match measurement {
        Measurement::CurrentProcess => measure::executing_ref()?,
        Measurement::SelectedOutput => measure::selected_executable_ref(&output)?,
    };
    let (artifacts, gate_bytes) = measure_artifacts(&output)?;
    molten_core::startup_binding::admit(&binding, &trust.selection, &molten_core::startup_binding::Observation {
        binding_ref: &binding_ref,
        executing_ref: &executing_ref,
        authenticated_output: true,
        artifacts: &artifacts,
    })
    .map_err(|issue| invalid(format!("source binding admission denied: {issue:?}")))?;
    closure::validate(&output, &trust)?;
    let gate_text = std::str::from_utf8(&gate_bytes)
        .map_err(|error| invalid(format!("produced gate receipt is not UTF-8: {error}")))?;
    let gate_value = crate::preserves_rail::parse_text(gate_text)?;
    let gate_ref = crate::octet_gate::validate_produced_octet_gate_receipt(&gate_value)?;
    accepted(binding_ref, gate_ref, gate_value, &executing_ref)
}

fn validate_retained_inputs(
    root: &cap_std::fs::Dir,
    binding: &molten_core::startup_binding::Binding,
) -> crate::error::Result<()> {
    for (path, expected) in [
        ("policy", binding.policy_ref.as_str()),
        ("profile", binding.profile_ref.as_str()),
        ("build-closure", binding.dependencies_ref.as_str()),
    ] {
        let bytes = measure::read(root, path, 16_777_216)?;
        if crate::preserves_rail::content_ref_from_bytes(&bytes) != expected {
            return Err(invalid(format!("source binding retained {path} identity mismatch")));
        }
    }
    Ok(())
}

fn validate_trust(trust: &Trust) -> crate::error::Result<()> {
    if trust.schema != "molten-node-source-trust-v1" || !trust.nix.is_absolute() {
        return Err(invalid("unsupported source trust or non-absolute verifier capability"));
    }
    if trust.public_key.split_whitespace().count() != 1 || !trust.public_key.contains(':') {
        return Err(invalid("source trust requires one independently selected full public key"));
    }
    if trust.output.parent() != Some(std::path::Path::new("/nix/store")) {
        return Err(invalid("source binding must name one exact immutable Nix store output"));
    }
    Ok(())
}

fn measure_artifacts(
    root: &cap_std::fs::Dir,
) -> crate::error::Result<(Vec<molten_core::startup_binding::Artifact>, Vec<u8>)> {
    let mut artifacts = Vec::with_capacity(molten_core::startup_binding::REQUIRED_ROLES.len());
    for (role, directory) in [("primary-cohort", "primary"), ("library-cohort", "library")] {
        artifacts.push(molten_core::startup_binding::Artifact {
            role: role.to_string(),
            content_ref: measure::cohort_ref(root, directory)?,
        });
    }
    let gate = measure::read(root, "gate-receipt.preserves", 1_048_576)?;
    let remediation = measure::read(root, "remediation-plan.preserves", 1_048_576)?;
    for (role, bytes) in [("gate-receipt", &gate), ("remediation-receipt", &remediation)] {
        artifacts.push(molten_core::startup_binding::Artifact {
            role: role.to_string(),
            content_ref: crate::preserves_rail::content_ref_from_bytes(bytes),
        });
    }
    Ok((artifacts, gate))
}

fn accepted(
    binding_ref: String,
    gate_ref: String,
    gate_value: preserves::IOValue,
    executing_ref: &str,
) -> crate::error::Result<Accepted> {
    let validation_value = crate::preserves_rail::record("node-source-binding-validation-v1", vec![
        crate::preserves_rail::string("molten-node-source-binding-v1"),
        crate::preserves_rail::string(&binding_ref),
        crate::preserves_rail::string(executing_ref),
        crate::preserves_rail::string(&gate_ref),
    ]);
    let validation_ref = crate::preserves_rail::canonical_hash(&validation_value)?;
    Ok(Accepted {
        binding_ref,
        gate_ref,
        gate_value,
        validation_ref,
        validation_value,
    })
}

#[cfg(test)]
pub(super) fn fixture() -> crate::error::Result<Accepted> {
    let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let gate_ref = crate::preserves_rail::canonical_hash(&gate_value)?;
    let validation_value = crate::preserves_rail::record("node-source-fixture-validation-v1", vec![
        crate::preserves_rail::string("explicit-test-only-no-producer-authentication"),
        crate::preserves_rail::string(&gate_ref),
    ]);
    let validation_ref = crate::preserves_rail::canonical_hash(&validation_value)?;
    Ok(Accepted {
        binding_ref: validation_ref.clone(),
        gate_ref,
        gate_value,
        validation_ref,
        validation_value,
    })
}

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
