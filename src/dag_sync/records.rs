use molten_core::dag_sync::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

const DAG_RECORD_IDENTITY_CONTEXT: &str = "onixresearch.molten.dag-sync.record.identity.v1";
const DAG_NODE_RECORD: &str = "molten-dag-node-v1";
const DAG_ROOT_RECORD: &str = "molten-dag-root-v1";
const DAG_REQUEST_RECORD: &str = "molten-dag-sync-request-v1";
const DAG_PLAN_RECORD: &str = "molten-dag-sync-plan-v1";
const DAG_RESPONSE_RECORD: &str = "molten-dag-sync-response-v1";
const DAG_PROGRESS_RECORD: &str = "molten-dag-sync-progress-v1";
const DAG_RECEIPT_RECORD: &str = "molten-dag-sync-receipt-v1";

#[derive(Debug, Clone)]
pub struct CanonicalDagRecord {
    pub record_ref: String,
    pub value: IOValue,
    pub bytes: Vec<u8>,
}

pub fn canonical_dag_node(node: &DagNode) -> Result<CanonicalDagRecord> {
    let mut edges = node.edges.clone();
    edges.sort();
    let value = record(DAG_NODE_RECORD, vec![
        field("node-ref", string(node.node_ref.as_str())),
        field("schema-ref", string(node.schema_ref.as_str())),
        field("payload-ref", optional(node.payload_ref.as_ref().map(DagContentRef::as_str))),
        field("encoded-bytes", number(node.encoded_bytes)),
        field(
            "edges",
            sequence(
                edges
                    .iter()
                    .map(|edge| record("dag-edge", vec![string(edge.kind.as_str()), string(edge.target.as_str())]))
                    .collect(),
            ),
        ),
    ]);
    canonical("node", value)
}

pub fn canonical_dag_root(root: &DagRoot) -> Result<CanonicalDagRecord> {
    root.validate_domain()
        .map_err(|issue| MoltenError::invalid_harness(format!("invalid DAG root domain: {issue:?}")))?;
    canonical(
        "root",
        record(DAG_ROOT_RECORD, vec![
            field("root-ref", string(root.root_ref.as_str())),
            field("domain", string(&root.domain)),
            field("node-ref", string(root.node_ref.as_str())),
            field("schema-ref", string(root.schema_ref.as_str())),
        ]),
    )
}

pub fn canonical_dag_request(request: &DagSyncRequest) -> Result<CanonicalDagRecord> {
    let mut roots = request.root_refs.clone();
    roots.sort();
    let mut available = request.inventory.available.clone();
    available.sort();
    let mut peers = request.peers.clone();
    peers.sort();
    canonical(
        "request",
        record(DAG_REQUEST_RECORD, vec![
            field("roots", refs(roots.iter().map(DagRootRef::as_str))),
            field("strategy", string(request.strategy.as_str())),
            field("available", sequence(available.iter().map(object_value).collect())),
            field(
                "progress",
                request.progress.as_ref().map_or_else(
                    || record("none", Vec::new()),
                    |progress| record("some", vec![progress_value(progress)]),
                ),
            ),
            field("peers", refs(peers.iter().map(DagPeerId::as_str))),
            field("epoch-ref", string(request.epoch_ref.as_str())),
            field("generation", number(request.generation)),
            field("policy-ref", string(request.policy_ref.as_str())),
            field("bounds", bounds_value(&request.bounds)),
        ]),
    )
}

pub fn canonical_dag_plan(plan: &DagSyncPlan) -> Result<CanonicalDagRecord> {
    canonical(
        "plan",
        record(DAG_PLAN_RECORD, vec![
            field("plan-ref", string(plan.plan_ref.as_str())),
            field("epoch-ref", string(plan.epoch_ref.as_str())),
            field("generation", number(plan.generation)),
            field("strategy", string(plan.strategy.as_str())),
            field("roots", refs(plan.roots.iter().map(DagRootRef::as_str))),
            field("topological-nodes", refs(plan.topological_nodes.iter().map(DagNodeRef::as_str))),
            field("missing", sequence(plan.missing.iter().map(object_value).collect())),
            field(
                "requests",
                sequence(
                    plan.requests
                        .iter()
                        .map(|request| {
                            record("fetch-request", vec![
                                object_value(&request.object_ref),
                                optional(request.assigned_peer.as_ref().map(DagPeerId::as_str)),
                                usize_value(request.sequence),
                            ])
                        })
                        .collect(),
                ),
            ),
            field("complete", boolean(plan.complete)),
            non_claims(),
        ]),
    )
}

pub fn canonical_dag_response(response: &DagResponseObservation) -> Result<CanonicalDagRecord> {
    canonical(
        "response",
        record(DAG_RESPONSE_RECORD, vec![
            field("epoch-ref", string(response.epoch_ref.as_str())),
            field("generation", number(response.generation)),
            field("object", object_value(&response.object_ref)),
            field("assigned-peer", optional(response.assigned_peer.as_ref().map(DagPeerId::as_str))),
            field("identity-verified", boolean(response.identity_verified)),
            field("authorization-admitted", boolean(response.authorization_admitted)),
            field("encoded-bytes", number(response.encoded_bytes)),
            non_claims(),
        ]),
    )
}

pub fn canonical_dag_progress(progress: &DagSyncProgress) -> Result<CanonicalDagRecord> {
    canonical("progress", progress_value(progress))
}

pub fn canonical_dag_receipt(receipt: &DagSyncReceipt) -> Result<CanonicalDagRecord> {
    let expected_non_claims = DAG_SYNC_NON_CLAIMS.iter().map(ToString::to_string).collect::<Vec<_>>();
    if receipt.non_claims != expected_non_claims {
        return Err(MoltenError::invalid_harness("DAG-sync receipt non-claims are incomplete"));
    }
    canonical(
        "receipt",
        record(DAG_RECEIPT_RECORD, vec![
            field("decision", string(receipt.decision.as_str())),
            field("plan-ref", optional(receipt.plan_ref.as_ref().map(DagPlanRef::as_str))),
            field("epoch-ref", string(receipt.epoch_ref.as_str())),
            field("generation", number(receipt.generation)),
            field("strategy", string(receipt.strategy.as_str())),
            field("requested", usize_value(receipt.requested)),
            field("verified", usize_value(receipt.verified)),
            field("missing", sequence(receipt.missing.iter().map(object_value).collect())),
            field("issues", sequence(receipt.issues.iter().map(|issue| string(issue.as_str())).collect())),
            non_claims(),
        ]),
    )
}

fn progress_value(progress: &DagSyncProgress) -> IOValue {
    let mut verified = progress.verified.clone();
    verified.sort();
    record(DAG_PROGRESS_RECORD, vec![
        field("epoch-ref", string(progress.epoch_ref.as_str())),
        field("generation", number(progress.generation)),
        field("strategy", string(progress.strategy.as_str())),
        field("policy-ref", string(progress.policy_ref.as_str())),
        field("verified", sequence(verified.iter().map(object_value).collect())),
        field("steps-completed", usize_value(progress.steps_completed)),
    ])
}

fn bounds_value(bounds: &DagBounds) -> IOValue {
    record("dag-bounds", vec![
        usize_value(bounds.max_nodes),
        usize_value(bounds.max_edges),
        usize_value(bounds.max_roots),
        usize_value(bounds.max_depth),
        number(bounds.max_bytes),
        usize_value(bounds.max_steps),
        usize_value(bounds.max_peers),
    ])
}

fn object_value(object: &DagObjectRef) -> IOValue {
    record("dag-object", vec![string(object.kind()), string(object.as_str())])
}

fn canonical(kind: &str, value: IOValue) -> Result<CanonicalDagRecord> {
    let bytes = crate::preserves_rail::canonical_bytes(&value)?;
    let mut hasher = blake3::Hasher::new_derive_key(DAG_RECORD_IDENTITY_CONTEXT);
    update(&mut hasher, kind);
    let length =
        u64::try_from(bytes.len()).map_err(|_| MoltenError::invalid_harness("DAG record byte length exceeds u64"))?;
    hasher.update(&length.to_be_bytes());
    hasher.update(&bytes);
    Ok(CanonicalDagRecord {
        record_ref: format!("blake3:{}", hasher.finalize().to_hex()),
        value,
        bytes,
    })
}

fn update(hasher: &mut blake3::Hasher, value: &str) {
    let length = u64::try_from(value.len()).unwrap_or(u64::MAX);
    hasher.update(&length.to_be_bytes());
    hasher.update(value.as_bytes());
}

fn non_claims() -> IOValue {
    field("non-claims", sequence(DAG_SYNC_NON_CLAIMS.iter().map(string).collect()))
}

fn optional(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn refs<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    sequence(values.map(string).collect())
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn boolean(value: bool) -> IOValue {
    record(if value { "true" } else { "false" }, Vec::new())
}

fn usize_value(value: usize) -> IOValue {
    number(u64::try_from(value).unwrap_or(u64::MAX))
}

fn number(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}
