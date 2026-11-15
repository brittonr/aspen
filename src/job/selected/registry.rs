pub const SELECTED_SEQUENCE_PROFILE: &str = "selected-sequence-v2";

/// Closed value schema; identity is canonical Preserves, not a producer's assertion.
pub fn selected_sequence_schema() -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "job-value-schema-v2", vec![crate::preserves_rail::string("sequence-u64")],
    ))
}

pub fn selected_control_schema() -> Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
        "job-value-schema-v2", vec![crate::preserves_rail::string("unit-control")],
    ))
}

fn selected_operation(node: &JobNode) -> Result<(String, IoValue)> {
    let fields = simple_record(&node.config, "selected-stage-v2", 3)?;
    Ok((required_string(&fields[0], "selected operation")?, crate::preserves_rail::value_to_iovalue(&fields[1])))
}

fn selected_kind(operation: &str) -> Result<&'static str> {
    match operation {
        "source-u64" => Ok("source"),
        "identity-u64" | "concat-u64" | "partition-even-u64" => Ok("map"),
        "drop-u64" => Ok("filter"),
        "sum-u64" => Ok("reduce"),
        _ => Err(MoltenError::invalid_harness("unsupported selected stage/profile")),
    }
}

fn selected_ports(operation: &str) -> (Vec<String>, Vec<String>) {
    let inputs = if operation == "source-u64" { vec!["after".into()] } else { vec!["in".into(), "after".into()] };
    let outputs = if operation == "partition-even-u64" {
        vec!["left".into(), "right".into(), "done".into()]
    } else { vec!["out".into(), "done".into()] };
    (inputs, outputs)
}

/// Exact signature includes implementation text, schemas, ordering and usage rules.
pub fn selected_stage_signature(operation: &str) -> Result<String> {
    let kind = selected_kind(operation)?;
    let (inputs, outputs) = selected_ports(operation);
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-stage-signature-v2", vec![
        crate::preserves_rail::string(SELECTED_SEQUENCE_PROFILE),
        crate::preserves_rail::string(kind), crate::preserves_rail::string(operation),
        refs_sequence(&inputs), refs_sequence(&outputs),
        crate::preserves_rail::string(selected_sequence_schema()?),
        crate::preserves_rail::string(selected_control_schema()?),
        crate::preserves_rail::string("required-in;identity/partition/drop-single;concat/sum-many;after-optional-many;closed-copy-drop;lexical-to/from/port;duplicates-denied"),
        crate::preserves_rail::string(include_str!("routing.rs")),
        crate::preserves_rail::string(include_str!("registry.rs")),
        crate::preserves_rail::string(include_str!("registry/admission.rs")),
    ]))
}

pub fn selected_node_value(id: &str, operation: &str, payload: IoValue) -> Result<IoValue> {
    ensure_count_at_most(id.len(), 128, "selected instance identifier bytes")?;
    let (inputs, outputs) = selected_ports(operation);
    let signature = selected_stage_signature(operation)?;
    job_node_value(NodeValueInput {
        id, kind: selected_kind(operation)?, stage_artifact_ref: None,
        input_ports: &inputs, output_ports: &outputs,
        config: crate::preserves_rail::record("selected-stage-v2", vec![crate::preserves_rail::string(operation), payload, crate::preserves_rail::string(signature)]),
        effect_manifest_refs: &[], policy_refs: &[], evidence_refs: &[],
    })
}

pub fn selected_dag_value(input: DagValueInput<'_>) -> Result<IoValue> {
    let legacy = job_dag_value(input)?;
    let fields = simple_record(&legacy, "job-dag-v1", 10)?;
    let mut values: Vec<_> = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect();
    values[0] = crate::preserves_rail::string("molten.job-dag.selected-sequence.v2");
    values[1] = crate::preserves_rail::record("version", vec![crate::preserves_rail::string(SELECTED_SEQUENCE_PROFILE)]);
    let value = crate::preserves_rail::record("job-dag-v2", values);
    parse_job_dag_value(&value)?;
    Ok(value)
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/job/selected/registry/admission.rs"));
