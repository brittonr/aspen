#![cfg(feature = "job-dag-construction")]
//! Shared Choregraph construction route through the real nickel-eval companion worker.

// r[verify molten.job_dag.captured_authoring]

mod denials;
#[path = "../src/test/support.rs"]
mod test_support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type IoValue = preserves::IOValue;

const WORKER: &str = env!("CARGO_BIN_EXE_molten-nickel-eval-worker");
const ENTRY: &str = "workflow.ncl";
const SEQUENCE: &str = include_str!("../../config/job-graphs/construction/sequence.ncl");
const DIAMOND: &str = include_str!("../../config/job-graphs/construction/diamond.ncl");
const INSTANCES: &str = include_str!("../../config/job-graphs/construction/instances.ncl");
const STAGES: &str = include_str!("../../config/job-graphs/construction/stages.ncl");
const UNIT_SEQUENCE: &str = include_str!("../../config/job-graphs/construction/unit-sequence.ncl");
const SINGLE: &str = include_str!("../../config/job-graphs/construction/single.ncl");
const SINGLE_FIXTURE: &str = include_str!("../../config/job-graphs/construction/single.preserves.in");
const SEQUENCE_TOTAL: &[&str] = &["pipeline", "root", "steps", "total"];

/// The worker bytes do not change during one test process, so the caller's selection is measured
/// once.
static WORKER_REF: std::sync::LazyLock<Result<String, String>> = std::sync::LazyLock::new(|| {
    std::fs::read(WORKER)
        .map(|bytes| molten::preserves_rail::content_ref_from_bytes(&bytes))
        .map_err(|error| error.to_string())
});

fn worker_ref() -> TestResult<String> {
    Ok(WORKER_REF.clone()?)
}

struct Authoring<'a> {
    source: &'a str,
    modules: &'a [molten::job_dag::ConstructionModule<'a>],
    parameters: serde_json::Value,
    outputs: &'a [&'a [&'a str]],
}

fn lower_with(
    authoring: &Authoring<'_>,
    expected_worker_ref: &str,
) -> TestResult<molten::job_dag::ConstructedNickelJob> {
    let mut modules = Vec::with_capacity(authoring.modules.len() + 1);
    modules.push(molten::job_dag::ConstructionModule {
        logical_id: ENTRY,
        source: authoring.source,
    });
    modules.extend_from_slice(authoring.modules);
    Ok(molten::job_dag::lower_constructed_nickel_job(molten::job_dag::ConstructedNickelJobInput {
        entry: ENTRY,
        modules: &modules,
        parameters: &authoring.parameters,
        outputs: authoring.outputs,
        worker: std::path::Path::new(WORKER),
        expected_worker_ref,
    })?)
}

fn lower(authoring: &Authoring<'_>) -> TestResult<molten::job_dag::ConstructedNickelJob> {
    lower_with(authoring, &worker_ref()?)
}

fn inline<'a>(source: &'a str, parameters: serde_json::Value, outputs: &'a [&'a [&'a str]]) -> Authoring<'a> {
    Authoring {
        source,
        modules: &[],
        parameters,
        outputs,
    }
}

fn values(items: &[u64]) -> IoValue {
    molten::preserves_rail::sequence(items.iter().copied().map(molten::preserves_rail::u64_value).collect())
}

fn data_edge(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> TestResult<IoValue> {
    let schema = molten::job_dag::selected_sequence_schema()?;
    Ok(molten::job_dag::job_edge_value(molten::job_dag::EdgeValueInput {
        from_node,
        from_port,
        to_node,
        to_port,
        schema_ref: Some(&schema),
        partitioning: "single",
        materialization: "stream",
    })?)
}

fn direct_dag(nodes: Vec<IoValue>, edges: Vec<IoValue>, root: &str) -> TestResult<IoValue> {
    Ok(molten::job_dag::selected_dag_value(molten::job_dag::DagValueInput {
        nodes,
        edges,
        output_roots: &[root.to_owned()],
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })?)
}

fn run(value: &IoValue, root: &std::path::Path) -> TestResult<molten::job_dag::JobRun> {
    Ok(molten::job_dag::run_job_dag_value(value, &molten::job_dag::JobRunOptions {
        registry_root: &root.join("registry"),
        storage_root: &root.join("storage"),
        cache_root: &root.join("cache"),
        chunk_root: &root.join("chunks"),
        ledger_root: None,
        output_request: None,
    })?)
}

#[test]
fn constructed_sequence_executes_like_the_direct_graph() -> TestResult {
    let workspace = test_support::process_workspace("construction_sequence")?;
    let job = lower(&inline(SEQUENCE, serde_json::json!({"values": [1, 2, 3, 4]}), &[SEQUENCE_TOTAL]))?;
    let emit = molten::job_dag::constructed_node_id(&job, &["pipeline", "root", "steps", "emit"])?;
    let copy = molten::job_dag::constructed_node_id(&job, &["pipeline", "root", "steps", "copy"])?;
    let total = molten::job_dag::constructed_node_id(&job, SEQUENCE_TOTAL)?;
    assert_eq!(job.dag.nodes.len(), 3);
    assert_eq!(job.projection.edges.len(), 2);
    assert_eq!(job.projection.control_edges.len(), 2, "sequence carries exit/entry barriers");
    let direct = direct_dag(
        vec![
            molten::job_dag::selected_node_value(&emit, "source-u64", values(&[1, 2, 3, 4]))?,
            molten::job_dag::selected_node_value(&copy, "identity-u64", values(&[]))?,
            molten::job_dag::selected_node_value(&total, "sum-u64", values(&[]))?,
        ],
        vec![
            data_edge(&emit, "out", &copy, "in")?,
            data_edge(&copy, "out", &total, "in")?,
        ],
        &total,
    )?;
    let constructed = run(&job.dag.value, &workspace.join("constructed"))?;
    let reference = run(&direct, &workspace.join("direct"))?;
    assert_eq!(constructed.output_value, reference.output_value);
    assert_eq!(molten::job_dag::plan_job_dag(&job.dag, None)?.stage_order, vec![emit, copy, total]);
    Ok(())
}

#[test]
fn shared_diamond_runs_one_producer_and_joins_both_arms() -> TestResult {
    let workspace = test_support::process_workspace("construction_diamond")?;
    let join_path: &[&str] = &["diamond", "root", "join"];
    let job = lower(&inline(DIAMOND, serde_json::json!({"values": [1, 2, 3, 4]}), &[join_path]))?;
    let emit = molten::job_dag::constructed_node_id(&job, &["diamond", "root", "emit"])?;
    let left = molten::job_dag::constructed_node_id(&job, &["diamond", "root", "left"])?;
    let right = molten::job_dag::constructed_node_id(&job, &["diamond", "root", "right"])?;
    let join = molten::job_dag::constructed_node_id(&job, join_path)?;
    assert_eq!(job.dag.nodes.len(), 4, "the reused output handle retains one producer");
    let direct = direct_dag(
        vec![
            molten::job_dag::selected_node_value(&emit, "source-u64", values(&[1, 2, 3, 4]))?,
            molten::job_dag::selected_node_value(&left, "identity-u64", values(&[]))?,
            molten::job_dag::selected_node_value(&right, "identity-u64", values(&[]))?,
            molten::job_dag::selected_node_value(&join, "sum-u64", values(&[]))?,
        ],
        vec![
            data_edge(&emit, "out", &left, "in")?,
            data_edge(&emit, "out", &right, "in")?,
            data_edge(&left, "out", &join, "in")?,
            data_edge(&right, "out", &join, "in")?,
        ],
        &join,
    )?;
    let constructed = run(&job.dag.value, &workspace.join("constructed"))?;
    let reference = run(&direct, &workspace.join("direct"))?;
    assert_eq!(constructed.output_value, reference.output_value);
    assert_eq!(constructed.stage_receipt_refs.len(), 4, "each instance executes once");
    Ok(())
}

#[test]
fn reusable_module_template_instances_stay_distinct() -> TestResult {
    let workspace = test_support::process_workspace("construction_instances")?;
    let modules = [molten::job_dag::ConstructionModule {
        logical_id: "stages.ncl",
        source: STAGES,
    }];
    let join_path: &[&str] = &["instances", "root", "join"];
    let job = lower(&Authoring {
        source: INSTANCES,
        modules: &modules,
        parameters: serde_json::json!({"first": [1, 2], "second": [3, 4]}),
        outputs: &[join_path],
    })?;
    let first = molten::job_dag::constructed_node_id(&job, &["instances", "root", "first", "body"])?;
    let second = molten::job_dag::constructed_node_id(&job, &["instances", "root", "second", "body"])?;
    let join = molten::job_dag::constructed_node_id(&job, join_path)?;
    assert_ne!(first, second, "two instantiations are two occurrences");
    assert_eq!(job.dag.nodes.len(), 3);
    let direct = direct_dag(
        vec![
            molten::job_dag::selected_node_value(&first, "source-u64", values(&[1, 2]))?,
            molten::job_dag::selected_node_value(&second, "source-u64", values(&[3, 4]))?,
            molten::job_dag::selected_node_value(&join, "sum-u64", values(&[]))?,
        ],
        vec![
            data_edge(&first, "out", &join, "in")?,
            data_edge(&second, "out", &join, "in")?,
        ],
        &join,
    )?;
    let constructed = run(&job.dag.value, &workspace.join("constructed"))?;
    let reference = run(&direct, &workspace.join("direct"))?;
    assert_eq!(constructed.output_value, reference.output_value);
    Ok(())
}

#[test]
fn zero_data_sequence_retains_its_dependency() -> TestResult {
    let job = lower(&inline(UNIT_SEQUENCE, serde_json::json!({}), &[]))?;
    let first = molten::job_dag::constructed_node_id(&job, &["ordered", "root", "z_first", "emit"])?;
    let later = molten::job_dag::constructed_node_id(&job, &["ordered", "root", "a_later", "emit"])?;
    assert!(later < first, "identities allocate opposite to the required order");
    assert!(job.projection.edges.is_empty());
    assert_eq!(job.projection.control_edges.len(), 1);
    assert_eq!(molten::job_dag::plan_job_dag(&job.dag, None)?.stage_order, vec![first, later]);
    Ok(())
}

#[test]
fn constructed_graph_matches_independent_canonical_fixture() -> TestResult {
    let job = lower(&inline(SINGLE, serde_json::json!({}), &[&["single", "root", "emit"]]))?;
    let fixture =
        SINGLE_FIXTURE.replace("@SOURCE_SIGNATURE@", &molten::job_dag::selected_stage_signature("source-u64")?);
    let expected = molten::preserves_rail::parse_text(&fixture)?;
    assert_eq!(job.dag.value, expected);
    assert_eq!(job.dag.job_ref, molten::preserves_rail::canonical_hash(&expected)?);
    Ok(())
}

#[test]
fn bindings_follow_parameters_modules_and_selected_outputs() -> TestResult {
    let parameters = serde_json::json!({"values": [1, 2]});
    let base = lower(&inline(SEQUENCE, parameters.clone(), &[SEQUENCE_TOTAL]))?;
    let changed = lower(&inline(SEQUENCE, serde_json::json!({"values": [3, 2]}), &[SEQUENCE_TOTAL]))?;
    assert_ne!(base.binding.association, changed.binding.association);
    assert_ne!(base.dag.job_ref, changed.dag.job_ref);
    let unused = [molten::job_dag::ConstructionModule {
        logical_id: "unused.ncl",
        source: "null",
    }];
    let captured = lower(&Authoring {
        source: SEQUENCE,
        modules: &unused,
        parameters: parameters.clone(),
        outputs: &[SEQUENCE_TOTAL],
    })?;
    assert_eq!(base.dag.job_ref, captured.dag.job_ref, "an unimported module does not change the graph");
    assert_ne!(base.binding.association, captured.binding.association, "the submitted module map is bound");
    assert_ne!(base.provenance, captured.provenance);
    let other_output = lower(&inline(SEQUENCE, parameters, &[&["pipeline", "root", "steps", "copy"]]))?;
    assert_ne!(base.dag.job_ref, other_output.dag.job_ref, "selected outputs are graph identity");
    Ok(())
}
