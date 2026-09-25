
#[test]
fn cli_node_profile_backed_init_binds_startup_metadata() -> CliResult<()> {
    let dir = temp_dir("cli-node-profile-backed")?;
    let state_root = dir.join("state");
    let config = dir.join("node-config.preserves");
    let startup = dir.join("node-startup.preserves");
    let profile_resolution = dir.join("node-profile-resolution.preserves");
    let profile_ref = test_ref("checked-node-profile")?;
    let profile_state_root_ref = test_ref("profile-state-root")?;
    let policy_ref = test_ref("profile-policy")?;
    let capability_ref = test_ref("profile-capability")?;
    let resource_ref = test_ref("profile-resource")?;
    let effect_ref = test_ref("profile-effect")?;
    let mut init = molten_cmd();
    init.args(["node", "init", "--state-root"])
        .arg(&state_root)
        .args(["--node-id", "node:profile", "--config-out"])
        .arg(&config)
        .args(["--profile-resolution-out"])
        .arg(&profile_resolution)
        .args(["--profile-ref", &profile_ref, "--actual-profile-ref", &profile_ref])
        .args(["--profile-tier", "pilot", "--profile-identity", "pilot-node"])
        .args(["--profile-state-root-ref", &profile_state_root_ref])
        .args(["--policy-ref", &policy_ref, "--capability-ref", &capability_ref])
        .args(["--resource-ref", &resource_ref, "--effect-profile-ref", &effect_ref]);
    for adapter in molten::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        let adapter_ref = test_ref(&format!("adapter-{adapter}"))?;
        init.arg("--adapter-profile").arg(format!("{adapter}={adapter_ref}"));
    }
    let output = init.output()?;
    assert_success(&output, "node profile-backed init");
    assert!(stdout(&output).contains("profile_resolution="));
    assert_eq!(molten::ledger::artifact_kind(&read_preserves(&config)?), "node-config");
    let resolution_value = read_preserves(&profile_resolution)?;
    let resolution_ref = molten::preserves_rail::canonical_hash(&resolution_value)?;
    assert!(molten::preserves_rail::to_text(&resolution_value)?.contains("node-profile-config-resolution-v1"));

    let run = molten_cmd()
        .args(["node", "run", "--state-root"])
        .arg(&state_root)
        .args(["--startup-out"])
        .arg(&startup)
        .output()?;
    assert_success(&run, "node profile-backed run");
    let startup_receipt = molten::node_runtime::parse_node_startup_receipt(&read_preserves(&startup)?)?;
    assert!(startup_receipt.profile_metadata_refs.contains(&profile_ref));
    assert!(startup_receipt.profile_metadata_refs.contains(&resolution_ref));
    Ok(())
}
