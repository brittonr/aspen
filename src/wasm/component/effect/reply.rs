// r[impl molten.wasm_component_adoption.hostcalls]
pub fn encode_host_response(input: super::HostResponseInput<'_>) -> super::super::model::Result<Vec<u8>> {
    validate(input.request, input.response, input.output)?;
    let output = match input.output {
        Some(value) => crate::preserves_rail::record("some", vec![value.clone()]),
        None => crate::preserves_rail::record("none", Vec::new()),
    };
    let value = crate::preserves_rail::record("component-effect-response-v1", vec![
        crate::preserves_rail::string(super::HOST_RESPONSE_SCHEMA),
        crate::preserves_rail::string(input.request.request_ref()),
        input.response.clone(),
        output,
    ]);
    let bytes = crate::preserves_rail::canonical_bytes(&value).map_err(super::convert)?;
    super::bounded(&bytes)?;
    Ok(bytes)
}

pub fn decode_host_response(
    bytes: &[u8],
    request: &super::HostRequest,
) -> super::super::model::Result<super::HostResponse> {
    super::bounded(bytes)?;
    let value = crate::preserves_rail::parse_canonical_bytes(bytes).map_err(super::convert)?;
    let fields = crate::preserves_rail::simple_record_fields(&value, "component-effect-response-v1", 4)
        .map_err(super::convert)?;
    let schema =
        crate::preserves_rail::required_string_field(&fields[0], "effect response schema").map_err(super::convert)?;
    if schema != super::HOST_RESPONSE_SCHEMA {
        return Err(super::invalid("unsupported component effect response schema"));
    }
    let request_ref = crate::preserves_rail::required_content_ref_string(&fields[1], "request carrier ref")
        .map_err(super::convert)?;
    if request_ref != request.request_ref() {
        return Err(super::invalid("component effect response is bound to another request carrier"));
    }
    let response = crate::preserves_rail::value_to_iovalue(&fields[2]);
    let output = if fields[3].collect_simple_record("none", Some(0)).is_some() {
        None
    } else if let Some(some) = fields[3].collect_simple_record("some", Some(1)) {
        Some(crate::preserves_rail::value_to_iovalue(&some[0]))
    } else {
        return Err(super::invalid("component effect response has no exact optional output"));
    };
    let response = validate(request, &response, output.as_ref())?;
    Ok(super::HostResponse {
        request_ref,
        response,
        output,
    })
}

fn validate(
    request: &super::HostRequest,
    value: &preserves::IOValue,
    output: Option<&preserves::IOValue>,
) -> super::super::model::Result<crate::effects::EffectResponse> {
    let response = crate::effects::parse_effect_response(value).map_err(super::convert)?;
    if response.request_ref != request.effect().request_ref {
        return Err(super::invalid("component effect response is bound to another request"));
    }
    match (response.decision.as_str(), output, response.output_ref.as_deref()) {
        ("pass", Some(output), Some(expected)) => {
            if crate::preserves_rail::canonical_hash(output).map_err(super::convert)? != expected {
                return Err(super::invalid("component effect response output identity is stale"));
            }
        }
        ("deny", None, None) => {}
        _ => return Err(super::invalid("component effect response decision and materialized output disagree")),
    }
    Ok(response)
}
