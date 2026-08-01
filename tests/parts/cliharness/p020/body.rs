// r[verify molten.fabric_simulation.operator_workflow]
// r[verify molten.fabric_simulation.final_validation]
#[test]
fn fabric_simulation_cli_runs_replays_shrinks_inspects_and_exports() -> CliResult<()> {
    let dir = temp_dir("fabric-simulation-cli")?;
    let run_dir = dir.join("run");
    let shrink_dir = dir.join("shrink");
    let export_dir = dir.join("export");

    let preflight = molten_cmd().args(["fabric-simulation", "preflight"]).output()?;
    assert_success(&preflight, "fabric simulation preflight");
    assert!(stdout(&preflight).contains("nodes=3"));
    assert!(stdout(&preflight).contains("ports=13"));

    let run = molten_cmd()
        .args(["fabric-simulation", "run", "--out"])
        .arg(&run_dir)
        .output()?;
    assert_success(&run, "fabric simulation run");
    assert!(stdout(&run).contains("decision=pass"));
    assert!(run_dir.join("world.preserves").is_file());
    assert!(run_dir.join("report.preserves").is_file());
    assert!(run_dir.join("bundle.preserves").is_file());
    assert!(run_dir.join("differential.preserves").is_file());

    let inspect = molten_cmd()
        .args(["fabric-simulation", "inspect"])
        .arg(run_dir.join("report.preserves"))
        .output()?;
    assert_success(&inspect, "fabric simulation inspect");
    assert!(stdout(&inspect).contains("profile=deterministic-whole-system"));
    assert!(stdout(&inspect).contains("decision=pass"));

    let replay = molten_cmd()
        .args(["fabric-simulation", "replay"])
        .arg(run_dir.join("report.preserves"))
        .output()?;
    assert_success(&replay, "fabric simulation replay");
    assert!(stdout(&replay).contains("replay ok"));

    let shrink = molten_cmd()
        .args(["fabric-simulation", "shrink", "--out"])
        .arg(&shrink_dir)
        .output()?;
    assert_success(&shrink, "fabric simulation shrink");
    assert!(stdout(&shrink).contains("failure-preserved=true"));
    assert!(shrink_dir.join("original-world.preserves").is_file());
    assert!(shrink_dir.join("shrunk-world.preserves").is_file());
    assert!(shrink_dir.join("shrink.preserves").is_file());

    let export = molten_cmd()
        .args(["fabric-simulation", "export", "--out"])
        .arg(&export_dir)
        .output()?;
    assert_success(&export, "fabric simulation export");
    assert!(stdout(&export).contains("artifacts=4"));
    assert!(export_dir.join("bundle.preserves").is_file());
    Ok(())
}

// r[verify molten.fabric_simulation.operator_workflow]
// r[verify molten.fabric_simulation.final_validation]
#[test]
fn fabric_simulation_cli_denies_tampered_report() -> CliResult<()> {
    let dir = temp_dir("fabric-simulation-cli-negative")?;
    let run_dir = dir.join("run");
    let tampered_report = dir.join("tampered-report.preserves");
    let run = molten_cmd()
        .args(["fabric-simulation", "run", "--out"])
        .arg(&run_dir)
        .output()?;
    assert_success(&run, "fabric simulation run for negative fixture");
    let source = std::fs::read_to_string(run_dir.join("report.preserves"))?;
    std::fs::write(&tampered_report, source.replacen("pass", "production-approved", 1))?;

    let inspect = molten_cmd()
        .args(["fabric-simulation", "inspect"])
        .arg(&tampered_report)
        .output()?;

    assert!(!inspect.status.success());
    assert!(String::from_utf8_lossy(&inspect.stderr).contains("unsupported simulation decision"));
    Ok(())
}
