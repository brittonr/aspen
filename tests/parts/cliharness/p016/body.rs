const CLUSTER_TEST_NODE: &str = "node-a";
const CLUSTER_TEST_NODE_B: &str = "node-b";
const CLUSTER_TEST_SENTINEL: &str = "stale-node-root-sentinel";
const CLUSTER_CONFIG_FILE: &str = "config.preserves";
const CLUSTER_IDENTITY_RECEIPT_FILE: &str = "identity-receipt.preserves";
const CLUSTER_STARTUP_FILE: &str = "startup-receipt.preserves";
const CLUSTER_HEALTH_FILE: &str = "health-receipt.preserves";
const CLUSTER_STATUS_CONTROL_FILE: &str = "status-control-receipt.preserves";
const CLUSTER_SHUTDOWN_FILE: &str = "shutdown-receipt.preserves";
const CLUSTER_STOP_CONTROL_FILE: &str = "stop-control-receipt.preserves";

#[test]
fn cli_cluster_init_denies_lifecycle_collision_and_force_resets_planned_node() -> CliResult<()> {
    let root = temp_dir("cli-cluster-reinit")?;
    let node_root = root.join(CLUSTER_TEST_NODE);

    let first = molten_cmd()
        .args(["cluster", "init", "--state-root"])
        .arg(&root)
        .args(["--node", CLUSTER_TEST_NODE])
        .output()?;
    assert_success(&first, "cluster init first run");
    assert!(node_root.join("config.preserves").exists());

    std::fs::remove_file(molten::cluster::cluster_manifest_path(&root))?;
    let denied = molten_cmd()
        .args(["cluster", "init", "--state-root"])
        .arg(&root)
        .args(["--node", CLUSTER_TEST_NODE])
        .output()?;
    assert_failure(&denied, "cluster init lifecycle collision");
    assert!(stderr(&denied).contains("Initialized lifecycle state"));

    let sentinel = node_root.join(CLUSTER_TEST_SENTINEL);
    std::fs::write(&sentinel, CLUSTER_TEST_SENTINEL)?;
    let forced = molten_cmd()
        .args(["cluster", "init", "--state-root"])
        .arg(&root)
        .args(["--node", CLUSTER_TEST_NODE, "--force"])
        .output()?;
    assert_success(&forced, "cluster init force reset");
    assert!(!sentinel.exists());
    assert!(node_root.join("config.preserves").exists());
    assert!(molten::cluster::cluster_manifest_path(&root).exists());
    Ok(())
}

#[test]
fn cli_cluster_init_denies_existing_manifest_without_force() -> CliResult<()> {
    let root = temp_dir("cli-cluster-manifest-collision")?;
    let nodes = vec![CLUSTER_TEST_NODE.to_string()];
    let plan = molten::cluster::plan_cluster(&root, &nodes)?;
    std::fs::write(molten::cluster::cluster_manifest_path(&root), molten::cluster::render_cluster_manifest(&plan))?;

    let denied = molten_cmd()
        .args(["cluster", "init", "--state-root"])
        .arg(&root)
        .args(["--node", CLUSTER_TEST_NODE])
        .output()?;
    assert_failure(&denied, "cluster init manifest collision");
    assert!(stderr(&denied).contains("manifest already exists"));
    Ok(())
}

#[test]
fn cli_cluster_lifecycle_commands_fail_closed_for_bad_manifests() -> CliResult<()> {
    let missing_root = temp_dir("cli-cluster-missing-manifest")?;
    let missing = molten_cmd()
        .args(["cluster", "start", "--state-root"])
        .arg(&missing_root)
        .output()?;
    assert_failure(&missing, "cluster start missing manifest");

    let empty_root = temp_dir("cli-cluster-empty-manifest")?;
    std::fs::write(molten::cluster::cluster_manifest_path(&empty_root), "")?;
    let empty = molten_cmd()
        .args(["cluster", "status", "--state-root"])
        .arg(&empty_root)
        .output()?;
    assert_failure(&empty, "cluster status empty manifest");
    assert!(stderr(&empty).contains("manifest is empty"));

    let malformed_root = temp_dir("cli-cluster-malformed-manifest")?;
    std::fs::write(
        molten::cluster::cluster_manifest_path(&malformed_root),
        "not-a-cluster\nnode:node-a\n",
    )?;
    let malformed = molten_cmd()
        .args(["cluster", "stop", "--state-root"])
        .arg(&malformed_root)
        .output()?;
    assert_failure(&malformed, "cluster stop malformed manifest");
    assert!(stderr(&malformed).contains("unsupported header"));

    let stale_root = temp_dir("cli-cluster-stale-manifest")?;
    std::fs::write(
        molten::cluster::cluster_manifest_path(&stale_root),
        "molten.cluster.nodes.v1\nnode:node-a\n",
    )?;
    let stale = molten_cmd()
        .args(["cluster", "start", "--state-root"])
        .arg(&stale_root)
        .output()?;
    assert_failure(&stale, "cluster start stale manifest");
    assert!(!stderr(&stale).trim().is_empty());
    Ok(())
}

#[test]
fn cli_cluster_two_node_lifecycle_roundtrip_writes_canonical_receipts() -> CliResult<()> {
    let root = temp_dir("cli-cluster-lifecycle")?;
    let init = molten_cmd()
        .args(["cluster", "init", "--state-root"])
        .arg(&root)
        .args(["--node", CLUSTER_TEST_NODE, "--node", CLUSTER_TEST_NODE_B])
        .output()?;
    assert_success(&init, "cluster init roundtrip");

    let manifest_path = molten::cluster::cluster_manifest_path(&root);
    let manifest = std::fs::read_to_string(&manifest_path)?;
    let manifest_nodes = molten::cluster::parse_cluster_manifest(&manifest)?;
    assert_eq!(manifest_nodes, vec!["node:node-a".to_string(), "node:node-b".to_string()]);
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_CONFIG_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_IDENTITY_RECEIPT_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_CONFIG_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_IDENTITY_RECEIPT_FILE)?;

    let start = molten_cmd().args(["cluster", "start", "--state-root"]).arg(&root).output()?;
    assert_success(&start, "cluster start roundtrip");
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_STARTUP_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_STARTUP_FILE)?;
    let node_a_startup = std::fs::read(root.join(CLUSTER_TEST_NODE).join(CLUSTER_STARTUP_FILE))?;
    let node_b_startup = std::fs::read(root.join(CLUSTER_TEST_NODE_B).join(CLUSTER_STARTUP_FILE))?;

    let already_running = molten_cmd().args(["cluster", "start", "--state-root"]).arg(&root).output()?;
    assert_success(&already_running, "cluster start already running");
    assert_eq!(node_a_startup, std::fs::read(root.join(CLUSTER_TEST_NODE).join(CLUSTER_STARTUP_FILE))?);
    assert_eq!(node_b_startup, std::fs::read(root.join(CLUSTER_TEST_NODE_B).join(CLUSTER_STARTUP_FILE))?);

    let status = molten_cmd().args(["cluster", "status", "--state-root"]).arg(&root).output()?;
    assert_success(&status, "cluster status roundtrip");
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_HEALTH_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_STATUS_CONTROL_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_HEALTH_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_STATUS_CONTROL_FILE)?;

    let stop = molten_cmd().args(["cluster", "stop", "--state-root"]).arg(&root).output()?;
    assert_success(&stop, "cluster stop roundtrip");
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_SHUTDOWN_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE, CLUSTER_STOP_CONTROL_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_SHUTDOWN_FILE)?;
    assert_cluster_file_is_canonical(&root, CLUSTER_TEST_NODE_B, CLUSTER_STOP_CONTROL_FILE)?;
    let stop_stdout = stdout(&stop);
    let node_b_index = stop_stdout.find("node=node:node-b").expect("node-b stop output");
    let node_a_index = stop_stdout.find("node=node:node-a").expect("node-a stop output");
    assert!(node_b_index < node_a_index, "cluster stop should visit nodes in reverse manifest order");
    Ok(())
}

fn assert_cluster_file_is_canonical(root: &std::path::Path, node: &str, name: &str) -> CliResult<()> {
    let value = read_preserves(&root.join(node).join(name))?;
    let reference = molten::preserves_rail::canonical_hash(&value)?;
    molten::preserves_rail::validate_content_ref(&reference)?;
    Ok(())
}
