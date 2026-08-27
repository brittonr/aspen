use std::collections::BTreeMap;
use std::collections::BTreeSet;

use molten_core::dag_sync::*;

use crate::error::MoltenError;
use crate::error::Result;

const JOB_NODE_CONTEXT: &str = "onixresearch.molten.dag-sync.job-node.v1";
const JOB_ROOT_CONTEXT: &str = "onixresearch.molten.dag-sync.job-root.v1";
const JOB_SCHEMA_CONTEXT: &str = "onixresearch.molten.dag-sync.job-schema-set.v1";
const ARTIFACT_SCHEMA_CONTEXT: &str = "onixresearch.molten.dag-sync.artifact-schema.v1";

pub fn project_job_dag(dag: &crate::workload::JobDag) -> Result<DagGraph> {
    let schema_ref = derived_schema(JOB_SCHEMA_CONTEXT, &dag.schema_refs)?;
    let node_refs = dag
        .nodes
        .iter()
        .map(|node| Ok((node.id.clone(), derived_node(JOB_NODE_CONTEXT, &[dag.job_ref.as_str(), node.id.as_str()])?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut reverse_edges = BTreeMap::<String, Vec<DagEdge>>::new();
    for edge in &dag.edges {
        let source = node_refs
            .get(&edge.from_node)
            .ok_or_else(|| MoltenError::invalid_harness("job DAG edge source is unknown"))?;
        if !node_refs.contains_key(&edge.to_node) {
            return Err(MoltenError::invalid_harness("job DAG edge destination is unknown"));
        }
        reverse_edges.entry(edge.to_node.clone()).or_default().push(DagEdge {
            kind: DagEdgeKind::Dependency,
            target: source.clone(),
        });
    }
    let nodes = dag
        .nodes
        .iter()
        .map(|node| {
            let bytes = crate::preserves_rail::canonical_bytes(&node.config)?;
            let encoded_bytes =
                u64::try_from(bytes.len()).map_err(|_| MoltenError::invalid_harness("job node bytes exceed u64"))?;
            let payload_ref = node
                .stage_artifact_ref
                .as_ref()
                .map(|reference| DagContentRef::new(reference.clone()))
                .transpose()
                .map_err(dag_reference_error)?;
            let mut edges = reverse_edges.remove(&node.id).unwrap_or_default();
            edges.sort();
            Ok(DagNode {
                node_ref: node_refs
                    .get(&node.id)
                    .ok_or_else(|| MoltenError::invalid_harness("job node identity is missing"))?
                    .clone(),
                schema_ref: schema_ref.clone(),
                payload_ref,
                encoded_bytes,
                edges,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let roots = dag
        .output_roots
        .iter()
        .map(|output| {
            let node_ref =
                node_refs.get(output).ok_or_else(|| MoltenError::invalid_harness("job output root is unknown"))?;
            Ok(DagRoot {
                root_ref: derived_root(JOB_ROOT_CONTEXT, &[dag.job_ref.as_str(), output.as_str()])?,
                domain: "molten-job-dag".to_string(),
                node_ref: node_ref.clone(),
                schema_ref: schema_ref.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(DagGraph { roots, nodes })
}

pub fn project_artifact_closure(
    closure: &crate::objects::ArtifactClosure,
    edges: &[crate::objects::ArtifactDependencyEdge],
) -> Result<DagGraph> {
    if !closure.missing_refs.is_empty() {
        return Err(MoltenError::invalid_harness(
            "incomplete artifact closure cannot become a completed DAG projection",
        ));
    }
    let members = closure.closure_refs.iter().cloned().collect::<BTreeSet<_>>();
    let schema_ref = derived_schema(ARTIFACT_SCHEMA_CONTEXT, &["artifact-dependency-v1".to_string()])?;
    let mut by_source = BTreeMap::<String, Vec<DagEdge>>::new();
    for edge in edges {
        if !members.contains(&edge.source_ref) || !members.contains(&edge.target_ref) {
            continue;
        }
        by_source.entry(edge.source_ref.clone()).or_default().push(DagEdge {
            kind: DagEdgeKind::Dependency,
            target: DagNodeRef::new(edge.target_ref.clone()).map_err(dag_reference_error)?,
        });
    }
    let nodes = members
        .iter()
        .map(|reference| {
            let mut node_edges = by_source.remove(reference).unwrap_or_default();
            node_edges.sort();
            Ok(DagNode {
                node_ref: DagNodeRef::new(reference.clone()).map_err(dag_reference_error)?,
                schema_ref: schema_ref.clone(),
                payload_ref: Some(DagContentRef::new(reference.clone()).map_err(dag_reference_error)?),
                encoded_bytes: u64::try_from(reference.len())
                    .map_err(|_| MoltenError::invalid_harness("artifact reference length exceeds u64"))?,
                edges: node_edges,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let roots = closure
        .roots
        .iter()
        .map(|reference| {
            if !members.contains(reference) {
                return Err(MoltenError::invalid_harness("artifact closure root is not a closure member"));
            }
            Ok(DagRoot {
                root_ref: DagRootRef::new(reference.clone()).map_err(dag_reference_error)?,
                domain: "molten-artifact-closure".to_string(),
                node_ref: DagNodeRef::new(reference.clone()).map_err(dag_reference_error)?,
                schema_ref: schema_ref.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(DagGraph { roots, nodes })
}

fn derived_node(context: &'static str, fields: &[&str]) -> Result<DagNodeRef> {
    DagNodeRef::new(derived(context, fields)).map_err(dag_reference_error)
}

fn derived_root(context: &'static str, fields: &[&str]) -> Result<DagRootRef> {
    DagRootRef::new(derived(context, fields)).map_err(dag_reference_error)
}

fn derived_schema(context: &'static str, fields: &[String]) -> Result<DagSchemaRef> {
    let values = fields.iter().map(String::as_str).collect::<Vec<_>>();
    DagSchemaRef::new(derived(context, &values)).map_err(dag_reference_error)
}

fn derived(context: &'static str, fields: &[&str]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(context);
    for field in fields {
        let length = u64::try_from(field.len()).unwrap_or(u64::MAX);
        hasher.update(&length.to_be_bytes());
        hasher.update(field.as_bytes());
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn dag_reference_error(error: DagReferenceError) -> MoltenError {
    MoltenError::invalid_harness(format!("DAG domain projection reference is invalid: {error:?}"))
}
