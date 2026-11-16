
/// Actual node-status health-file open failure, serialized only after the syscall
/// reports permission denied. No caller-supplied decision or failure classification.
pub struct NodeStateAttemptInput<'a> {
    pub state_root: &'a str,
    pub target: &'a str,
    pub startup_ref: &'a str,
    pub config_ref: &'a str,
    pub binding_ref: &'a str,
    pub executable_path: &'a str,
    pub executable_ref: &'a str,
    pub effective_uid: u32,
    pub effective_gid: u32,
    pub groups: &'a str,
    pub cap_eff: &'a str,
    pub errno: i32,
}

pub fn node_state_permission_attempt_value(input: &NodeStateAttemptInput<'_>) -> Result<IoValue> {
    for reference in [input.startup_ref, input.config_ref, input.binding_ref, input.executable_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    for value in [input.state_root, input.target, input.executable_path, input.groups, input.cap_eff] {
        validate_text("node state attempt", value)?;
    }
    if input.target != "health-receipt.preserves" || input.errno != 13 {
        return Err(MoltenError::invalid_harness("only the actual status health open EACCES is an observed storage denial"));
    }
    Ok(record("node-state-operation-attempt-v1", vec![
        string("molten.node.state-operation-attempt.v1"),
        record("operation", vec![string("node-status-health-write")]),
        record("decision", vec![string("deny")]),
        record("state-root", vec![string(input.state_root)]),
        record("target", vec![string(input.target)]),
        record("startup-ref", vec![string(input.startup_ref)]),
        record("config-ref", vec![string(input.config_ref)]),
        record("binding-ref", vec![string(input.binding_ref)]),
        record("executable-path", vec![string(input.executable_path)]),
        record("executable-ref", vec![string(input.executable_ref)]),
        record("effective-uid", vec![string(input.effective_uid.to_string())]),
        record("effective-gid", vec![string(input.effective_gid.to_string())]),
        record("groups", vec![string(input.groups)]),
        record("cap-eff", vec![string(input.cap_eff)]),
        record("errno", vec![string(input.errno.to_string())]),
        record("phase", vec![string("open-before-write")]),
    ]))
}

pub struct VmStateObservationInput<'a> {
    pub phase: &'a str,
    pub target_node: &'a str,
    pub state_root: &'a str,
    pub target: &'a str,
    pub tree_ref: &'a str,
    pub root_uid: u32,
    pub root_gid: u32,
    pub root_mode: u32,
    pub target_absent: bool,
    pub package_path: &'a str,
    pub executable_ref: &'a str,
    pub startup_ref: &'a str,
    pub config_ref: &'a str,
    pub binding_ref: &'a str,
}

pub fn vm_state_observation_value(input: &VmStateObservationInput<'_>) -> Result<IoValue> {
    if !matches!(input.phase, "pre" | "post") {
        return Err(MoltenError::invalid_harness("VM state observation phase must be pre or post"));
    }
    for reference in [input.tree_ref, input.executable_ref, input.startup_ref, input.config_ref, input.binding_ref] {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    for value in [input.target_node, input.state_root, input.target, input.package_path] {
        validate_text("VM state observation", value)?;
    }
    Ok(record("nixos-vm-state-observation-v1", vec![
        string("molten.testing.nixos-vm.state-observation.v1"),
        record("phase", vec![string(input.phase)]),
        record("target-node", vec![string(input.target_node)]),
        record("state-root", vec![string(input.state_root)]),
        record("target", vec![string(input.target)]),
        record("tree-ref", vec![string(input.tree_ref)]),
        record("root-uid", vec![string(input.root_uid.to_string())]),
        record("root-gid", vec![string(input.root_gid.to_string())]),
        record("root-mode", vec![string(input.root_mode.to_string())]),
        record("target-absent", vec![string(input.target_absent.to_string())]),
        record("package-path", vec![string(input.package_path)]),
        record("executable-ref", vec![string(input.executable_ref)]),
        record("startup-ref", vec![string(input.startup_ref)]),
        record("config-ref", vec![string(input.config_ref)]),
        record("binding-ref", vec![string(input.binding_ref)]),
    ]))
}

#[derive(Debug)]
struct ParsedStateAttempt {
    state_root: String,
    target: String,
    startup_ref: String,
    config_ref: String,
    binding_ref: String,
    executable_path: String,
    executable_ref: String,
    uid: u32,
    gid: u32,
    groups: Vec<u32>,
    cap_eff: u64,
    errno: i32,
}

fn parse_state_attempt(value: &IoValue) -> Result<ParsedStateAttempt> {
    let fields = simple_record(value, "node-state-operation-attempt-v1", 16)?;
    require_schema(&fields[0], "molten.node.state-operation-attempt.v1", "node state attempt")?;
    if required_record_string(&fields[1], "operation", "attempt operation")? != "node-status-health-write"
        || required_record_string(&fields[2], "decision", "attempt decision")? != "deny"
        || required_record_string(&fields[15], "phase", "attempt phase")? != "open-before-write"
    {
        return Err(MoltenError::invalid_harness("node state attempt is not a denied status health open"));
    }
    let number = |index, label, context| -> Result<String> { required_record_string(&fields[index], label, context) };
    let groups_text = number(12, "groups", "attempt groups")?;
    let groups = if groups_text == "-" { Vec::new() } else {
        groups_text.split(',').take(MAX_VM_EFFECTIVE_GROUPS + 1)
            .map(|part| part.parse::<u32>().map_err(|_| MoltenError::invalid_harness("invalid effective group")))
            .collect::<Result<Vec<_>>>()?
    };
    if groups.len() > MAX_VM_EFFECTIVE_GROUPS {
        return Err(MoltenError::invalid_harness("effective group count exceeds VM bound"));
    }
    let cap_eff = u64::from_str_radix(&number(13, "cap-eff", "attempt capabilities")?, 16)
        .map_err(|_| MoltenError::invalid_harness("invalid effective capabilities"))?;
    Ok(ParsedStateAttempt {
        state_root: number(3, "state-root", "attempt state root")?,
        target: number(4, "target", "attempt target")?,
        startup_ref: required_record_ref(&fields[5], "startup-ref", "attempt startup")?,
        config_ref: required_record_ref(&fields[6], "config-ref", "attempt config")?,
        binding_ref: required_record_ref(&fields[7], "binding-ref", "attempt binding")?,
        executable_path: number(8, "executable-path", "attempt executable path")?,
        executable_ref: required_record_ref(&fields[9], "executable-ref", "attempt executable")?,
        uid: number(10, "effective-uid", "attempt uid")?
            .parse().map_err(|_| MoltenError::invalid_harness("invalid effective uid"))?,
        gid: number(11, "effective-gid", "attempt gid")?
            .parse().map_err(|_| MoltenError::invalid_harness("invalid effective gid"))?,
        groups,
        cap_eff,
        errno: number(14, "errno", "attempt errno")?
            .parse().map_err(|_| MoltenError::invalid_harness("invalid attempt errno"))?,
    })
}

#[derive(Debug)]
struct ParsedStateObservation {
    phase: String,
    target_node: String,
    state_root: String,
    target: String,
    tree_ref: String,
    root_uid: u32,
    root_gid: u32,
    root_mode: u32,
    target_absent: bool,
    package_path: String,
    executable_ref: String,
    startup_ref: String,
    config_ref: String,
    binding_ref: String,
}

fn parse_state_observation(value: &IoValue) -> Result<ParsedStateObservation> {
    let fields = simple_record(value, "nixos-vm-state-observation-v1", 15)?;
    require_schema(&fields[0], "molten.testing.nixos-vm.state-observation.v1", "VM state observation")?;
    let field = |index, label, context| -> Result<String> { required_record_string(&fields[index], label, context) };
    Ok(ParsedStateObservation {
        phase: field(1, "phase", "observation phase")?,
        target_node: field(2, "target-node", "observation target node")?,
        state_root: field(3, "state-root", "observation root")?,
        target: field(4, "target", "observation target")?,
        tree_ref: required_record_ref(&fields[5], "tree-ref", "observation tree")?,
        root_uid: field(6, "root-uid", "observation root uid")?
            .parse().map_err(|_| MoltenError::invalid_harness("invalid observation uid"))?,
        root_gid: field(7, "root-gid", "observation root gid")?
            .parse().map_err(|_| MoltenError::invalid_harness("invalid observation gid"))?,
        root_mode: field(8, "root-mode", "observation root mode")?
            .parse().map_err(|_| MoltenError::invalid_harness("invalid observation mode"))?,
        target_absent: match field(9, "target-absent", "observation target absence")?.as_str() {
            "true" => true,
            "false" => false,
            _ => return Err(MoltenError::invalid_harness("invalid observation target absence")),
        },
        package_path: field(10, "package-path", "observation package")?,
        executable_ref: required_record_ref(&fields[11], "executable-ref", "observation executable")?,
        startup_ref: required_record_ref(&fields[12], "startup-ref", "observation startup")?,
        config_ref: required_record_ref(&fields[13], "config-ref", "observation config")?,
        binding_ref: required_record_ref(&fields[14], "binding-ref", "observation binding")?,
    })
}

pub const MAX_VM_EFFECTIVE_GROUPS: usize = 64;

pub struct VmProcessIdentity {
    pub uid: u32,
    pub gid: u32,
    pub groups: String,
    pub cap_eff: String,
}



