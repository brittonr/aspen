use super::*;

fn values(items: &[u64]) -> IoValue {
    crate::preserves_rail::sequence(items.iter().copied().map(crate::preserves_rail::u64_value).collect())
}

fn edge(from: &str, port: &str, to: &str, input: &str) -> IoValue {
    let schema = if port == "done" {
        selected_control_schema()
    } else {
        selected_sequence_schema()
    }
    .expect("schema");
    job_edge_value(EdgeValueInput {
        from_node: from,
        from_port: port,
        to_node: to,
        to_port: input,
        schema_ref: Some(&schema),
        partitioning: "single",
        materialization: "stream",
    })
    .expect("edge")
}

fn graph(nodes: Vec<IoValue>, edges: Vec<IoValue>, roots: &[String]) -> Result<IoValue> {
    selected_dag_value(DagValueInput {
        nodes,
        edges,
        output_roots: roots,
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
}

fn partition_graph(selected: &str) -> IoValue {
    graph(
        vec![
            selected_node_value("source", "source-u64", values(&[1, 2, 3, 4])).expect("source"),
            selected_node_value("split", "partition-even-u64", values(&[])).expect("split"),
            selected_node_value("consumer", "identity-u64", values(&[])).expect("consumer"),
        ],
        vec![
            edge("source", "out", "split", "in"),
            edge("split", selected, "consumer", "in"),
        ],
        &["consumer".into()],
    )
    .expect("graph")
}

fn run(value: &IoValue, workspace: &FilePath) -> JobRun {
    run_job_dag_value(value, &JobRunOptions {
        registry_root: &workspace.join("registry"),
        storage_root: &workspace.join("storage"),
        cache_root: &workspace.join("cache"),
        chunk_root: &workspace.join("chunks"),
        ledger_root: None,
        output_request: None,
    })
    .expect("execute actual JobDag")
}

#[path = "../authoring/tests.rs"]
mod authoring;
#[path = "contracts.rs"]
mod contracts;
#[path = "oracle.rs"]
mod oracle;
include!("cases.rs");
