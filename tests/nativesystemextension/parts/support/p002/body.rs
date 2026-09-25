
fn effect_requirement() -> FabricPortRequirement {
    FabricPortRequirement {
        port_id: EFFECT_PORT_ID.to_string(),
        version: EFFECT_PORT_VERSION.to_string(),
        class: FabricPortClass::Evidence,
        operation_classes: vec![EFFECT_OPERATION.to_string()],
        input_schema_refs: vec![EFFECT_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![EFFECT_OUTPUT_SCHEMA.to_string()],
        allowed_authorities: vec![FabricAuthority::Evidence],
        available_resources: vec![FabricResource::Diagnostics],
        expected_determinism: DeterminismClass::ExternalEffect,
        expected_replay: ReplayClass::RecordedEffectRequired,
        expected_profile: "native-fixture-effect-v1".to_string(),
    }
}
