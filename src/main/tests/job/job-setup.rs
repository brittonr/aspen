fn setup_job_cli_fixture() -> JobSetup {
    let paths = JobPaths::new();
    let (dag_value, dag) = build_cli_job_dag(&paths.registry);
    write_file(&paths.dag_file, &to_text(&dag_value).expect("dag text")).expect("write dag");
    install_job_artifact(&paths, &dag);
    JobSetup {
        target_registry: paths.dir.join("target-registry"),
        dir: paths.dir,
        registry: paths.registry,
        storage: paths.storage,
        cache: paths.cache,
        chunks: paths.chunks,
        ledger_root: paths.ledger_root,
        output: paths.output,
        run_receipt: paths.run_receipt,
        dag,
    }
}

struct JobPaths {
    dir: PathBuf,
    registry: PathBuf,
    storage: PathBuf,
    cache: PathBuf,
    chunks: PathBuf,
    ledger_root: PathBuf,
    dag_file: PathBuf,
    output: PathBuf,
    run_receipt: PathBuf,
}

impl JobPaths {
    fn new() -> Self {
        let dir = temp_dir("job-cli");
        Self {
            registry: dir.join("registry"),
            storage: dir.join("storage"),
            cache: dir.join("cache"),
            chunks: dir.join("chunks"),
            ledger_root: dir.join("ledger"),
            dag_file: dir.join("job.preserves"),
            output: dir.join("job-output.preserves"),
            run_receipt: dir.join("job-run-receipt.preserves"),
            dir,
        }
    }
}

fn build_cli_job_dag(registry: &Path) -> (preserves::IOValue, job_dag::JobDag) {
    let source_stage = install_cli_stage_artifact(registry, "source");
    let reduce_stage = install_cli_stage_artifact(registry, "sum-u64");
    let materialize_stage = install_cli_stage_artifact(registry, "materialize");
    let dag_value = job_dag::job_dag_value(job_dag::DagValueInput {
        nodes: vec![
            source_job_node(&source_stage),
            reduce_job_node(&reduce_stage),
            materialize_job_node(&materialize_stage),
        ],
        edges: job_edges(),
        output_roots: &["out".to_string()],
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("dag value");
    let dag = job_dag::parse_job_dag_value(&dag_value).expect("parse dag");
    (dag_value, dag)
}

fn source_job_node(stage_ref: &str) -> preserves::IOValue {
    job_dag::job_node_value(job_dag::NodeValueInput {
        id: "source",
        kind: "source",
        stage_artifact_ref: Some(stage_ref),
        input_ports: &[],
        output_ports: &["out".to_string()],
        config: record(
            "source",
            vec![record(
                "values",
                vec![molten::preserves_rail::sequence(vec![
                    molten::preserves_rail::u64_value(1),
                    molten::preserves_rail::u64_value(2),
                ])],
            )],
        ),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("source node")
}

fn reduce_job_node(stage_ref: &str) -> preserves::IOValue {
    job_dag::job_node_value(job_dag::NodeValueInput {
        id: "sum",
        kind: "reduce",
        stage_artifact_ref: Some(stage_ref),
        input_ports: &["in".to_string()],
        output_ports: &["out".to_string()],
        config: record("op", vec![string("sum-u64")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("reduce node")
}

fn materialize_job_node(stage_ref: &str) -> preserves::IOValue {
    job_dag::job_node_value(job_dag::NodeValueInput {
        id: "out",
        kind: "materialize",
        stage_artifact_ref: Some(stage_ref),
        input_ports: &["in".to_string()],
        output_ports: &["out".to_string()],
        config: record("materialize", vec![string("inline")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("materialize node")
}

fn job_edges() -> Vec<preserves::IOValue> {
    vec![job_edge("source", "out", "sum", "in"), job_edge("sum", "out", "out", "in")]
}

fn job_edge(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> preserves::IOValue {
    job_dag::job_edge_value(job_dag::EdgeValueInput {
        from_node,
        from_port,
        to_node,
        to_port,
        schema_ref: None,
        partitioning: "single",
        materialization: "stream",
    })
    .expect("job edge")
}

fn install_job_artifact(paths: &JobPaths, dag: &job_dag::JobDag) {
    run_job_command(JobCommand::Install(cli_job::command::base::Install {
        dag: paths.dag_file.clone(),
        registry: paths.registry.clone(),
        receipt_out: Some(paths.dir.join("job-install-receipt.preserves")),
        artifact_out: Some(paths.dir.join("job-artifact.preserves")),
    }))
    .expect("job install");
    run_job_command(JobCommand::Show(cli_job::command::base::Show {
        job: dag.job_ref.clone(),
        registry: paths.registry.clone(),
    }))
    .expect("job show");
}
