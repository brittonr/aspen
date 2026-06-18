    fn exercise_job_planning(setup: &JobSetup) {
        let plan_out = setup.dir.join("job-plan.preserves");
        let profile_out = setup.dir.join("job-profile.preserves");
        let fusion_out = setup.dir.join("job-fusion.preserves");
        run_job_command(JobCommand::Plan(cli_job::command::base::Plan {
            job: setup.dag.job_ref.clone(),
            registry: setup.registry.clone(),
            output_request: None,
            out: Some(plan_out.clone()),
            receipt_out: Some(setup.dir.join("job-plan-receipt.preserves")),
        }))
        .expect("job plan");
        run_job_command(JobCommand::Profile(cli_job::command::base::Profile {
            job: setup.dag.job_ref.clone(),
            registry: setup.registry.clone(),
            cache: Some(setup.cache.clone()),
            output_request: None,
            out: Some(profile_out.clone()),
            receipt_out: Some(setup.dir.join("job-profile-receipt.preserves")),
        }))
        .expect("job profile");
        run_job_command(JobCommand::FusionPreview(cli_job::command::base::FusionPreview {
            job: setup.dag.job_ref.clone(),
            registry: setup.registry.clone(),
            output_request: None,
            out: Some(fusion_out.clone()),
            receipt_out: Some(setup.dir.join("job-fusion-receipt.preserves")),
        }))
        .expect("job fusion preview");
        assert!(fs::read_to_string(&plan_out).expect("read plan").contains("job-plan-v1"));
        assert!(fs::read_to_string(&profile_out).expect("read profile").contains("job-profile-v1"));
        assert!(fs::read_to_string(&fusion_out).expect("read fusion").contains("job-fusion-plan-v1"));
    }

    fn exercise_job_sync(setup: &JobSetup) -> JobSync {
        let sync_plan_out = setup.dir.join("job-sync-plan.preserves");
        let sync_loopback_receipt = setup.dir.join("job-sync-loopback-receipt.preserves");
        let source_artifacts = artifacts::list_artifacts(&setup.registry, None).expect("list source artifacts");
        let mut provenance_paths = Vec::with_capacity(source_artifacts.len());
        for artifact in source_artifacts {
            let provenance_path = setup.dir.join(format!("job-provenance-{}.preserves", provenance_paths.len()));
            let provenance_value =
                provenance::synthetic_reviewed_provenance_record(&artifact.artifact_ref).expect("provenance");
            write_file(&provenance_path, &to_text(&provenance_value).expect("provenance text"))
                .expect("write provenance");
            provenance_paths.push(provenance_path);
        }
        run_job_command(JobCommand::SyncPlan(cli_job::command::sync::Plan {
            job: setup.dag.job_ref.clone(),
            source_registry: setup.registry.clone(),
            target_registry: setup.target_registry.clone(),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            out: Some(sync_plan_out.clone()),
            receipt_out: Some(setup.dir.join("job-sync-plan-receipt.preserves")),
        }))
        .expect("job sync plan");
        run_job_command(JobCommand::SyncLoopback(cli_job::command::sync::Loopback {
            job: setup.dag.job_ref.clone(),
            source_registry: setup.registry.clone(),
            target_registry: setup.target_registry.clone(),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            provenance_paths,
            build_verification_paths: Vec::new(),
            plan_out: Some(setup.dir.join("job-sync-loopback-plan.preserves")),
            receipt_out: Some(sync_loopback_receipt.clone()),
        }))
        .expect("job sync loopback");
        assert!(fs::read_to_string(&sync_plan_out).expect("read sync plan").contains("job-sync-plan-v1"));
        assert!(
            !artifacts::list_artifacts(&setup.target_registry, Some(job_dag::JOB_ARTIFACT_KIND))
                .expect("target jobs")
                .is_empty()
        );
        let sync_ref =
            canonical_hash(&read_preserves_file(&sync_loopback_receipt).expect("read sync receipt")).expect("sync ref");
        JobSync {
            sync_ref,
            source_gate_ref: install_cli_clean_octet_gate(&setup.target_registry),
            admission_policy_ref: cli_synthetic_ref("job-worker-admission-policy").expect("policy ref"),
            authority_context_ref: install_cli_job_execute_authority_context(&setup.target_registry, &setup.dag.job_ref),
            worker_resource_refs: vec![
                cli_synthetic_ref("job-worker-resource-a").expect("resource a"),
                cli_synthetic_ref("job-worker-resource-b").expect("resource b"),
                cli_synthetic_ref("job-worker-resource-c").expect("resource c"),
            ],
        }
    }
