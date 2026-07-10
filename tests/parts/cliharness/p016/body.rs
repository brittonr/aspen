const CLUSTER_TEST_NODE: &str = "node-a";
const CLUSTER_TEST_SENTINEL: &str = "stale-node-root-sentinel";

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
