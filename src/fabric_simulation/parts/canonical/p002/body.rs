
fn fault_value(fault: &SimulationFaultAction) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-fault-v1", vec![
        field("fault-id", crate::preserves_rail::string(&fault.fault_id)),
        field("kind", crate::preserves_rail::string(fault.kind.as_str())),
        field("target", crate::preserves_rail::string(&fault.target)),
        field("boundary", crate::preserves_rail::string(fault.boundary.as_str())),
        field("activate-at-choice", crate::preserves_rail::u64_value(fault.activate_at_choice)),
        field("duration-choices", optional_u64(fault.duration_choices)),
        field("resource-cost", crate::preserves_rail::u64_value(fault.resource_cost)),
        field("expected-observation", crate::preserves_rail::string(&fault.expected_observation)),
        field(
            "direct-extension-state-mutation",
            crate::preserves_rail::bool_value(fault.direct_extension_state_mutation),
        ),
    ])
}

fn invariant_value(invariant: &SimulationInvariant) -> preserves::IOValue {
    match invariant {
        SimulationInvariant::Universal(kind) => crate::preserves_rail::record("universal-invariant", vec![field(
            "kind",
            crate::preserves_rail::string(kind.as_str()),
        )]),
        SimulationInvariant::ExtensionSemantic { service, invariant_id } => {
            crate::preserves_rail::record("extension-invariant", vec![
                field("service", crate::preserves_rail::string(service.as_str())),
                field("invariant-id", crate::preserves_rail::string(invariant_id)),
            ])
        }
    }
}

fn bounds_value(bounds: &SimulationBounds) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-bounds-v1", vec![
        field("max-choices", crate::preserves_rail::u64_value(bounds.max_choices)),
        field("max-events", crate::preserves_rail::u64_value(bounds.max_events)),
        field("max-virtual-ticks", crate::preserves_rail::u64_value(bounds.max_virtual_ticks)),
        field("max-trace-bytes", crate::preserves_rail::u64_value(bounds.max_trace_bytes)),
        field("max-resource-units", crate::preserves_rail::u64_value(bounds.max_resource_units)),
        field("max-shrink-attempts", crate::preserves_rail::u64_value(bounds.max_shrink_attempts)),
    ])
}

fn choice_record_value(record_value: &SchedulerChoiceRecord) -> crate::error::Result<preserves::IOValue> {
    if record_value.semantic_output_ref.is_empty() || record_value.semantic_output_ref == PENDING_SEMANTIC_OUTPUT_REF {
        return Err(crate::error::MoltenError::invalid_harness(
            "fabric simulation choice record lacks its executed semantic output ref",
        ));
    }
    Ok(crate::preserves_rail::record("fabric-simulation-choice-v1", vec![
        field("position", crate::preserves_rail::u64_value(record_value.position)),
        field("virtual-tick", crate::preserves_rail::u64_value(record_value.virtual_tick)),
        field(
            "eligible",
            crate::preserves_rail::sequence(record_value.eligible.iter().map(eligible_choice_value).collect()),
        ),
        field("selected", eligible_choice_value(&record_value.selected)),
        field("semantic-output-ref", crate::preserves_rail::string(&record_value.semantic_output_ref)),
    ]))
}

fn eligible_choice_value(choice: &EligibleChoice) -> preserves::IOValue {
    crate::preserves_rail::record("eligible-choice-v1", vec![
        field("kind", crate::preserves_rail::string(choice.kind.as_str())),
        field("choice-id", crate::preserves_rail::string(&choice.choice_id)),
        field("node-id", crate::preserves_rail::string(&choice.node_id)),
        field("generation", crate::preserves_rail::u64_value(choice.generation)),
        field("ready-at-tick", crate::preserves_rail::u64_value(choice.ready_at_tick)),
    ])
}

fn invariant_result_value(result: &InvariantResult) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-simulation-invariant-result-v1", vec![
        field("invariant", invariant_value(&result.invariant)),
        field("passed", crate::preserves_rail::bool_value(result.passed)),
        field("first-failure-sequence", optional_u64(result.first_failure_sequence)),
    ])
}

fn optional_divergence_value(divergence: Option<&ReplayDivergence>) -> preserves::IOValue {
    match divergence {
        None => crate::preserves_rail::record("none", Vec::new()),
        Some(divergence) => crate::preserves_rail::record("some", vec![crate::preserves_rail::record(
            "fabric-simulation-divergence-v1",
            vec![
                field("position", crate::preserves_rail::u64_value(divergence.position)),
                field("expected-choice-id", crate::preserves_rail::string(&divergence.expected_choice_id)),
                field("eligible-choice-ids", strings_value(divergence.eligible_choice_ids.iter().map(String::as_str))),
                field("diagnostic", crate::preserves_rail::string(&divergence.diagnostic)),
            ],
        )]),
    }
}

fn record_optional_divergence(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<String>> {
    let field_value = named_field_value(value, "first-divergence")?;
    if field_value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = field_value
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected optional simulation divergence"))?;
    Ok(Some(crate::preserves_rail::canonical_hash((&some[0]).into())?))
}

fn validate_count(label: &str, actual: usize) -> crate::error::Result<()> {
    if actual > MAX_CANONICAL_SIMULATION_ITEMS {
        Err(crate::error::MoltenError::invalid_harness(format!(
            "fabric simulation {label} count {actual} exceeds {MAX_CANONICAL_SIMULATION_ITEMS}"
        )))
    } else {
        Ok(())
    }
}

fn validate_refs(label: &str, refs: &[String]) -> crate::error::Result<()> {
    validate_count(label, refs.len())?;
    for reference in refs {
        crate::preserves_rail::validate_content_ref(reference)?;
    }
    Ok(())
}

fn u64_len(value: usize) -> crate::error::Result<preserves::IOValue> {
    let converted = u64::try_from(value)
        .map_err(|_| crate::error::MoltenError::invalid_harness("fabric simulation collection length overflow"))?;
    Ok(crate::preserves_rail::u64_value(converted))
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn checks(values: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(values.iter().copied()))
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::u64_value(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn record_string_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    required_string(&named_field_value(value, label)?, label)
}

fn record_u64_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    named_field_value(value, label)?
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string_sequence_field(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Vec<String>> {
    let field_value = named_field_value(value, label)?;
    let sequence = field_value
        .as_sequence()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    sequence.map(|item| required_string(&item, label)).collect()
}

fn named_field_value(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<preserves::Value<preserves::IOValue>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected named field {label}")))?;
    Ok(fields[0].clone())
}

fn required_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}
