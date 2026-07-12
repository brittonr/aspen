    struct JobSetup {
        dir: crate::test_support::ProcessWorkspace,
        registry: PathBuf,
        storage: PathBuf,
        cache: PathBuf,
        chunks: PathBuf,
        ledger_root: PathBuf,
        output: PathBuf,
        run_receipt: PathBuf,
        dag: molten::job_dag::JobDag,
        target_registry: PathBuf,
    }

    struct JobSync {
        sync_ref: String,
        source_gate_ref: String,
        admission_policy_ref: String,
        context_ref: String,
        worker_resource_refs: Vec<String>,
    }

    struct JobAdmission {
        admit_loopback_receipt: PathBuf,
        worker_execution_request: PathBuf,
    }

    #[test]
    fn cli_job_dag_commands_work() {
        let setup = setup_job_cli_fixture();
        exercise_job_planning(&setup);
        let sync = exercise_job_sync(&setup);
        let admission = exercise_job_admission(&setup, &sync);
        exercise_job_worker(&setup, &sync, &admission);
        exercise_job_run_status(&setup);
    }
