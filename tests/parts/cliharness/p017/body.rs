#[test]
fn cli_nixos_vm_shard_and_aggregate_receipts_work() -> CliResult<()> {
    let dir = temp_dir("cli-nixos-vm-shards")?;
    let shard = dir.join("shard.preserves");
    let aggregate = dir.join("aggregate.preserves");
    let topology_ref = cli_vm_ref("topology");
    let package_ref = cli_vm_ref("package");
    let manifest_ref = cli_vm_ref("manifest");
    let node_ref = cli_vm_ref("node-evidence");
    let child_ref = cli_vm_ref("child-receipt");
    let log_ref = cli_vm_ref("diagnostic-log");

    let shard_output = molten_cmd()
        .args([
            "test",
            "nixos-vm",
            "shard-run",
            "--shard-id",
            "nixos-vm-live-control",
            "--scenario-fixture-ref",
            &cli_vm_ref("scenario-fixture"),
            "--topology-ref",
            &topology_ref,
            "--package-ref",
            &package_ref,
            "--node-evidence-ref",
            &node_ref,
            "--child-receipt-ref",
            &child_ref,
            "--diagnostic-log-ref",
            &log_ref,
            "--claimed-decision",
            "pass",
            "--caveat",
            "CLI shard receipt is fixture evidence only",
            "--out",
        ])
        .arg(&shard)
        .output()?;
    assert_success(&shard_output, "nixos-vm shard-run CLI");
    let shard_value = read_preserves(&shard)?;
    let shard_ref = molten::preserves_rail::canonical_hash(&shard_value)?;
    let shard_text = molten::preserves_rail::to_text(&shard_value)?;
    assert!(shard_text.contains("nixos-vm-shard-run-v1"));
    assert!(shard_text.contains("decision \"pass\""));

    let aggregate_output = molten_cmd()
        .args([
            "test",
            "nixos-vm",
            "aggregate",
            "--topology-ref",
            &topology_ref,
            "--package-ref",
            &package_ref,
            "--manifest-ref",
            &manifest_ref,
            "--required-shard-id",
            "nixos-vm-live-control",
            "--shard-ref",
            &shard_ref,
            "--caveat",
            "CLI aggregate receipt indexes child shard evidence only",
            "--out",
        ])
        .arg(&aggregate)
        .output()?;
    assert_success(&aggregate_output, "nixos-vm aggregate CLI");
    let aggregate_text = molten::preserves_rail::to_text(&read_preserves(&aggregate)?)?;
    assert!(aggregate_text.contains("nixos-vm-multinode-aggregate-v1"));
    assert!(aggregate_text.contains("decision \"pass\""));
    Ok(())
}

fn cli_vm_ref(label: &str) -> String {
    molten::preserves_rail::content_ref_from_bytes(label.as_bytes())
}
