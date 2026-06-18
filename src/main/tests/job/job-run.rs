    fn exercise_job_run_status(setup: &JobSetup) {
        run_job_command(JobCommand::Run(cli_job::command::base::Run {
            job: setup.dag.job_ref.clone(),
            registry: setup.registry.clone(),
            storage: setup.storage.clone(),
            cache: setup.cache.clone(),
            chunks: Some(setup.chunks.clone()),
            ledger: Some(setup.ledger_root.clone()),
            output_request: None,
            out: Some(setup.output.clone()),
            receipt_out: Some(setup.run_receipt.clone()),
        }))
        .expect("job run");
        assert!(fs::read_to_string(&setup.output).expect("read output").contains("3"));
        let receipt_ref =
            canonical_hash(&read_preserves_file(&setup.run_receipt).expect("read run receipt")).expect("receipt ref");
        run_job_command(JobCommand::Status(cli_job::command::refs::Status {
            ledger: setup.ledger_root.clone(),
            job: Some(setup.dag.job_ref.clone()),
        }))
        .expect("job status");
        run_job_command(JobCommand::ReceiptShow(cli_job::command::refs::ReceiptShow {
            receipt_ref,
            ledger: setup.ledger_root.clone(),
        }))
        .expect("job receipt show");
    }
