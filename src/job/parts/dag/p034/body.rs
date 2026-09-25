
fn parse_job_node_value(value: &IoValue) -> Result<JobNode> {
    let fields = value
        .collect_simple_record("job-node-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <job-node-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::JOB_DAG_NODE_SCHEMA, "job node")?;
    let id = record_string(&fields[1], "id")?;
    validate_node_id(&id)?;
    let kind = record_string(&fields[2], "kind")?;
    validate_stage_kind(&kind)?;
    let stage_artifact_ref = record_optional_ref(&fields[3], "stage-artifact")?;
    let input_ports = record_port_sequence(&fields[4], "inputs")?;
    let output_ports = record_port_sequence(&fields[5], "outputs")?;
    let config = record_iovalue(&fields[6], "config")?;
    reject_mobile_closure_config(&config)?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "stage-artifact-not-closure", "job node")?;
    Ok(JobNode {
        id,
        kind,
        stage_artifact_ref,
        input_ports,
        output_ports,
        config,
        effect_manifest_refs: record_ref_sequence(&fields[7], "effects")?,
        policy_refs: record_ref_sequence(&fields[8], "policy")?,
        evidence_refs: record_ref_sequence(&fields[9], "evidence")?,
        checks,
    })
}
