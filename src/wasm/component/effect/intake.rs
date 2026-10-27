// r[impl molten.wasm_component_adoption.hostcalls]
pub fn encode_host_request(input: super::HostRequestInput<'_>) -> super::super::model::Result<Vec<u8>> {
    validate(input.grant, input.effect, input.payload)?;
    let value = crate::preserves_rail::record("component-effect-request-v1", vec![
        crate::preserves_rail::string(super::HOST_REQUEST_SCHEMA),
        crate::preserves_rail::u64_value(input.generation),
        crate::preserves_rail::u64_value(input.incarnation),
        crate::preserves_rail::u64_value(input.sequence),
        grant_value(input.grant),
        input.effect.clone(),
        input.payload.clone(),
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).map_err(super::convert)?;
    super::bounded(&bytes)?;
    Ok(bytes)
}

/// Only shape, identities and the closed operation are checked here. The host
/// must independently recheck current policy, authority, resources and replay.
pub fn decode_host_request(bytes: &[u8]) -> super::super::model::Result<super::HostRequest> {
    super::bounded(bytes)?;
    let value = crate::preserves_rail::parse_canonical_bytes(bytes).map_err(super::convert)?;
    let fields = crate::preserves_rail::simple_record_fields(&value, "component-effect-request-v1", 7)
        .map_err(super::convert)?;
    let schema =
        crate::preserves_rail::required_string_field(&fields[0], "effect request schema").map_err(super::convert)?;
    if schema != super::HOST_REQUEST_SCHEMA {
        return Err(super::invalid("unsupported component effect request schema"));
    }
    let grant = parse_grant(&crate::preserves_rail::value_to_iovalue(&fields[4]))?;
    let effect = crate::preserves_rail::value_to_iovalue(&fields[5]);
    let payload = crate::preserves_rail::value_to_iovalue(&fields[6]);
    let parsed = validate(&grant, &effect, &payload)?;
    Ok(super::HostRequest {
        generation: integer(&fields[1])?,
        incarnation: integer(&fields[2])?,
        sequence: integer(&fields[3])?,
        grant,
        effect: parsed,
        payload,
        request_ref: crate::preserves_rail::content_ref_from_bytes(bytes),
    })
}

fn validate(
    grant: &super::super::admission::ImportGrant,
    value: &preserves::IOValue,
    payload: &preserves::IOValue,
) -> super::super::model::Result<crate::effects::EffectRequest> {
    if grant.import != super::HOST_REQUEST_IMPORT || grant.capability != super::HOST_BLOB_CAPABILITY {
        return Err(super::invalid("component effect import or capability is outside the closed set"));
    }
    for reference in [
        &grant.policy_ref,
        &grant.authority_ref,
        &grant.resource_ref,
        &grant.recorded_effect_ref,
    ] {
        crate::preserves_rail::validate_content_ref(reference).map_err(super::convert)?;
    }
    let request = crate::effects::parse_effect_request(value).map_err(super::convert)?;
    if request.effect_id != "blob.get" || request.operation != "get" || request.capability_refs.is_empty() {
        return Err(super::invalid("component effect operation is not the declared blob.get request"));
    }
    let blob = crate::preserves_rail::simple_record_fields(payload, "blob-get", 1).map_err(super::convert)?;
    crate::preserves_rail::required_content_ref(&blob[0], "blob object ref").map_err(super::convert)?;
    if crate::preserves_rail::canonical_hash(payload).map_err(super::convert)? != request.input_ref {
        return Err(super::invalid("component effect input identity does not match its materialized payload"));
    }
    Ok(request)
}

fn grant_value(grant: &super::super::admission::ImportGrant) -> preserves::IOValue {
    crate::preserves_rail::record("import-grant", vec![
        crate::preserves_rail::string(&grant.import),
        crate::preserves_rail::string(&grant.capability),
        crate::preserves_rail::string(&grant.policy_ref),
        crate::preserves_rail::string(&grant.authority_ref),
        crate::preserves_rail::string(&grant.resource_ref),
        crate::preserves_rail::string(&grant.recorded_effect_ref),
    ])
}

fn parse_grant(value: &preserves::IOValue) -> super::super::model::Result<super::super::admission::ImportGrant> {
    let fields = crate::preserves_rail::simple_record_fields(value, "import-grant", 6).map_err(super::convert)?;
    let string = |index: usize| {
        crate::preserves_rail::required_string_field(&fields[index], "component effect binding").map_err(super::convert)
    };
    Ok(super::super::admission::ImportGrant {
        import: string(0)?,
        capability: string(1)?,
        policy_ref: string(2)?,
        authority_ref: string(3)?,
        resource_ref: string(4)?,
        recorded_effect_ref: string(5)?,
    })
}

fn integer(value: &preserves::Value<preserves::IOValue>) -> super::super::model::Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| super::invalid("component effect counter must be an integer"))?
        .map_err(|error| super::invalid(format!("component effect counter is outside u64: {error}")))
}
