
#[cfg(test)]
mod tests {
    use super::*;

    const CONCURRENT_WORKSPACE_COUNT: usize = 8;
    const NO_ACCESS_MODE: u32 = 0o000;
    const OWNER_READ_WRITE_MODE: u32 = 0o600;

    #[test]
    fn concurrent_workspaces_and_typed_subroots_are_isolated() {
        // r[verify molten.testing.cap_std_workspace]
        // r[verify molten.testing.cap_std_subroots]
        cleanup_stale_molten_temp_dirs();
        let handles = (0..CONCURRENT_WORKSPACE_COUNT)
            .map(|index| {
                std::thread::spawn(move || {
                    let workspace = TestWorkspace::new(&format!("concurrent_{index}")).expect("workspace");
                    assert_eq!(workspace.logical_label(), format!("concurrent_{index}"));
                    let state = workspace.state().expect("state root");
                    let input = workspace.input().expect("input root");
                    let output = workspace.output().expect("output root");
                    let transport = workspace.transport().expect("transport root");
                    let ledger = workspace.ledger().expect("ledger root");
                    let cache = workspace.cache().expect("cache root");
                    let adversarial = workspace.adversarial().expect("adversarial root");
                    for root in [
                        input.dir(),
                        transport.dir(),
                        ledger.dir(),
                        cache.dir(),
                        adversarial.dir(),
                    ] {
                        assert!(root.metadata(".").expect("role root metadata").is_dir());
                    }
                    let path = WorkspacePath::parse("shared/value.bin").expect("workspace path");
                    let bytes = format!("state-{index}").into_bytes();
                    state.write(&path, &bytes).expect("state write");
                    output.write(&path, b"output").expect("output write");
                    assert_eq!(state.read(&path).expect("state read"), bytes);
                    assert_eq!(output.read(&path).expect("output read"), b"output");
                    workspace.inner.workspace_id
                })
            })
            .collect::<Vec<_>>();
        let mut ids = handles.into_iter().map(|handle| handle.join().expect("workspace thread")).collect::<Vec<_>>();
        ids.sort_by_key(blake3::Hash::to_hex);
        ids.dedup();
        assert_eq!(ids.len(), CONCURRENT_WORKSPACE_COUNT);
    }

    #[tokio::test]
    async fn async_workspace_survives_yield_and_exports_selected_artifact() {
        // r[verify molten.testing.cap_std_validation]
        let source_workspace = TestWorkspace::new("async_source").expect("source workspace");
        let destination_workspace = TestWorkspace::new("async_destination").expect("destination workspace");
        let state = source_workspace.state().expect("state root");
        let output = destination_workspace.output().expect("output root");
        let source_path = WorkspacePath::parse("receipts/run.preserves").expect("source path");
        state.write(&source_path, b"receipt").expect("source write");
        tokio::task::yield_now().await;
        let plan = ArtifactExportPlan::new("run_receipt", "receipts/run.preserves", "selected/run.preserves")
            .expect("export plan");
        let receipt = source_workspace.export_selected(&state, &output, &plan).expect("selected export");
        assert_eq!(receipt.artifact_label, "run_receipt");
        assert!(receipt.content_ref.starts_with("blake3:"));
        assert_eq!(
            output
                .read(&WorkspacePath::parse("selected/run.preserves").expect("destination path"))
                .expect("exported bytes"),
            b"receipt"
        );
    }

    #[test]
    fn process_bridge_runs_child_without_putting_host_path_in_evidence() {
        // r[verify molten.testing.cap_std_process_bridge]
        let workspace = TestWorkspace::new("child_process").expect("workspace");
        let state = workspace.state().expect("state root");
        let plan = workspace.process_bridge().plan(&state).expect("process plan");
        let output = std::process::Command::new(std::env::current_exe().expect("current test executable"))
            .arg(TEST_LIST_ARGUMENT)
            .current_dir(plan.path())
            .output()
            .expect("run child test executable");
        assert!(output.status.success());
        validate_portable_evidence(&[plan.logical_root()], &[plan.path()]).expect("portable logical evidence");
        assert!(!format!("{plan:?}").contains(&plan.path().to_string_lossy().into_owned()));
    }

    #[test]
    fn workspace_drop_cleans_only_its_owned_root() {
        // r[verify molten.testing.cap_std_cleanup]
        let process_workspace = ProcessWorkspace::new("cleanup_owned").expect("process workspace");
        let state_path = process_workspace.to_path_buf();
        std::fs::write(state_path.join("owned.bin"), b"owned").expect("owned fixture");
        assert!(state_path.exists());
        drop(process_workspace);
        assert!(!state_path.exists());
    }

    #[test]
    fn wrong_workspace_and_invalid_export_are_denied() {
        // r[verify molten.testing.cap_std_validation]
        let first = TestWorkspace::new("wrong_root_first").expect("first workspace");
        let second = TestWorkspace::new("wrong_root_second").expect("second workspace");
        let second_state = second.state().expect("second state");
        let first_output = first.output().expect("first output");
        let bridge_error = first.process_bridge().plan(&second_state).expect_err("cross-workspace bridge denied");
        assert_eq!(bridge_error.kind(), std::io::ErrorKind::PermissionDenied);
        let plan = ArtifactExportPlan::new("missing", "missing.bin", "selected/missing.bin").expect("export plan");
        let export_error = first
            .export_selected(&second_state, &first_output, &plan)
            .expect_err("cross-workspace export denied");
        assert_eq!(export_error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(ArtifactExportPlan::new("escape", "../outside", "selected/outside").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn adversarial_symlink_corruption_replacement_and_mode_stay_in_test_shell() {
        // r[verify molten.testing.cap_std_validation]
        let target_workspace = TestWorkspace::new("adversarial_target").expect("target workspace");
        let outside_workspace = ProcessWorkspace::new("adversarial_outside").expect("outside workspace");
        let state = target_workspace.state().expect("target state");
        let setup = target_workspace.adversarial_setup();
        let value_path = WorkspacePath::parse("values/value.bin").expect("value path");
        state.write(&value_path, b"original").expect("initial write");
        setup.corrupt(&state, &value_path, b"corrupt").expect("corrupt fixture");
        assert_eq!(state.read(&value_path).expect("corrupt read"), b"corrupt");
        setup.replace(&state, &value_path, b"replacement").expect("replace fixture");
        assert_eq!(state.read(&value_path).expect("replacement read"), b"replacement");
        setup.set_mode(&state, &value_path, NO_ACCESS_MODE).expect("deny mode");
        setup.set_mode(&state, &value_path, OWNER_READ_WRITE_MODE).expect("restore mode");
        setup.remove(&state, &value_path).expect("remove fixture");
        assert!(!state.try_exists(&value_path).expect("removed state"));
        let link_path = WorkspacePath::parse("values/outside-link").expect("link path");
        setup
            .symlink_to_host(&state, &link_path, outside_workspace.as_ref())
            .expect("outside symlink fixture");
        assert!(state.read(&link_path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_does_not_follow_replaced_symlink_or_remove_external_workspace() {
        // r[verify molten.testing.cap_std_validation]
        let outside = ProcessWorkspace::new("cleanup_external").expect("outside workspace");
        std::fs::write(outside.join("marker.bin"), b"external").expect("outside marker");
        let target = ProcessWorkspace::new("cleanup_replaced").expect("target workspace");
        let target_path = target.to_path_buf();
        std::fs::remove_dir(&target_path).expect("remove empty target state");
        std::os::unix::fs::symlink(&*outside, &target_path).expect("replace state with symlink");
        drop(target);
        assert_eq!(std::fs::read(outside.join("marker.bin")).expect("outside marker survives"), b"external");
    }

    #[test]
    fn canonical_evidence_rejects_temporary_host_path_leakage() {
        // r[verify molten.testing.cap_std_validation]
        let process_workspace = ProcessWorkspace::new("portable_evidence").expect("process workspace");
        validate_portable_evidence(&[process_workspace.logical_root()], &[process_workspace.as_ref()])
            .expect("logical evidence passes");
        let leaked = format!("state-root={}", process_workspace.display());
        let error = validate_portable_evidence(&[&leaked], &[process_workspace.as_ref()])
            .expect_err("host path leakage denied");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(!format!("{process_workspace:?}").contains(&process_workspace.display().to_string()));
    }

    #[test]
    fn logical_labels_and_paths_reject_ambient_or_traversing_inputs() {
        assert!(TestWorkspace::new("../escape").is_err());
        assert!(WorkspacePath::parse("/absolute").is_err());
        assert!(WorkspacePath::parse("../parent").is_err());
        assert!(WorkspacePath::parse(r"C:\outside").is_err());
        assert!(WorkspacePath::parse("https://example.invalid/value").is_err());
    }
}
