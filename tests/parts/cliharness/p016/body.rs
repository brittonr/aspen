#[test]
fn cli_cluster_node_lifecycle_routes_are_removed_without_effects() -> CliResult<()> {
    let dir = temp_dir("cli-cluster-node-routes-removed")?;
    let help = molten_cmd().args(["cluster", "--help"]).output()?;
    assert_success(&help, "cluster command help");
    let help_text = stdout(&help);
    assert!(lists_subcommand(&help_text, "harness-run"));
    assert!(lists_subcommand(&help_text, "fabric-transport-run"));
    for route in ["init", "start", "status", "stop"] {
        assert!(!lists_subcommand(&help_text, route), "cluster {route} is still exposed");
        let root = dir.join(format!("{route}-state"));
        let output = molten_cmd()
            .args(["cluster", route, "--state-root"])
            .arg(&root)
            .output()?;
        assert_failure(&output, &format!("removed cluster {route}"));
        assert!(stderr(&output).contains(route));
        assert!(output.stdout.is_empty());
        assert!(!root.exists(), "removed cluster {route} mutated state");
    }
    Ok(())
}

#[test]
fn cli_root_node_routes_are_removed_without_effects() -> CliResult<()> {
    let dir = temp_dir("cli-root-node-routes-removed")?;
    for args in [&["--help"][..], &["test", "--help"][..]] {
        let help = molten_cmd().args(args).output()?;
        assert_success(&help, "root or test command help");
        assert!(!lists_subcommand(&stdout(&help), "node"));
    }
    for (label, args) in [("node", &["node", "run"][..]), ("test-node", &["test", "node", "run"][..])] {
        let state_root = dir.join(format!("{label}-state"));
        let output = molten_cmd()
            .args(args)
            .arg("--state-root")
            .arg(&state_root)
            .output()?;
        assert_failure(&output, &format!("removed root {label} route"));
        assert!(!stderr(&output).is_empty());
        assert!(output.stdout.is_empty());
        assert!(!state_root.exists(), "removed root {label} route mutated state");
    }
    Ok(())
}

fn lists_subcommand(help: &str, name: &str) -> bool {
    help.lines().any(|line| line.split_whitespace().next() == Some(name))
}
