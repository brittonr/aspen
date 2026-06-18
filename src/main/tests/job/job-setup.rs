    fn setup_job_cli_fixture() -> JobSetup {
        let dir = temp_dir("job-cli");
        let registry = dir.join("registry");
        let storage = dir.join("storage");
        let cache = dir.join("cache");
        let chunks = dir.join("chunks");
        let ledger_root = dir.join("ledger");
        let dag_file = dir.join("job.preserves");
        let output = dir.join("job-output.preserves");
        let run_receipt = dir.join("job-run-receipt.preserves");
        let source_stage = install_cli_stage_artifact(&registry, "source");
        let reduce_stage = install_cli_stage_artifact(&registry, "sum-u64");
        let materialize_stage = install_cli_stage_artifact(&registry, "materialize");
        let source = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&source_stage),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: record("source", vec![record("values", vec![molten::preserves_rail::sequence(vec![
                molten::preserves_rail::u64_value(1),
                molten::preserves_rail::u64_value(2),
            ])])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node");
        let reduce = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "sum",
            kind: "reduce",
            stage_artifact_ref: Some(&reduce_stage),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("op", vec![string("sum-u64")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("reduce node");
        let materialize = job_dag::job_node_value(job_dag::NodeValueInput {
            id: "out",
            kind: "materialize",
            stage_artifact_ref: Some(&materialize_stage),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: record("materialize", vec![string("inline")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("materialize node");
        let e1 = job_dag::job_edge_value(job_dag::EdgeValueInput {
            from_node: "source",
            from_port: "out",
            to_node: "sum",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("e1");
        let e2 = job_dag::job_edge_value(job_dag::EdgeValueInput {
            from_node: "sum",
            from_port: "out",
            to_node: "out",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("e2");
        let dag_value = job_dag::job_dag_value(job_dag::DagValueInput {
            nodes: vec![source, reduce, materialize],
            edges: vec![e1, e2],
            output_roots: &["out".to_string()],
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("dag value");
        let dag = job_dag::parse_job_dag_value(&dag_value).expect("parse dag");
        write_file(&dag_file, &to_text(&dag_value).expect("dag text")).expect("write dag");
        run_job_command(JobCommand::Install(cli_job::command::base::Install {
            dag: dag_file,
            registry: registry.clone(),
            receipt_out: Some(dir.join("job-install-receipt.preserves")),
            artifact_out: Some(dir.join("job-artifact.preserves")),
        }))
        .expect("job install");
        run_job_command(JobCommand::Show(cli_job::command::base::Show {
            job: dag.job_ref.clone(),
            registry: registry.clone(),
        }))
        .expect("job show");
        JobSetup {
            target_registry: dir.join("target-registry"),
            dir,
            registry,
            storage,
            cache,
            chunks,
            ledger_root,
            output,
            run_receipt,
            dag,
        }
    }
