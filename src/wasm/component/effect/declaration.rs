//! The declared surface is useful for pre-effect admission. It is deliberately
//! not another admitted execution profile before the product adapter is ready.

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    schema_id: String,
    schema_version: u32,
    profile_id: String,
    #[serde(rename = "execution_admitted")]
    is_execution_admitted: bool,
    wit: super::super::model::WitCohort,
    allowed_imports: Vec<String>,
    allowed_exports: Vec<String>,
    allowed_wasi_interfaces: Vec<String>,
    closed_operations: Vec<Operation>,
    request_schema: String,
    response_schema: String,
    max_request_bytes: u64,
    max_response_bytes: u64,
    max_depth: u64,
    max_nodes: u64,
    required_current_inputs: Vec<String>,
    non_claims: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    effect_id: String,
    operation: String,
    capability: String,
}

// r[impl molten.wasm_component_adoption.hostcalls]
pub fn host_world() -> super::super::model::Result<super::super::imports::surface::DeclaredWorld> {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/wasm-component-runtime/generated/effect-contract.json"
    ));
    let contract: Contract = serde_json::from_str(source).map_err(|error| super::invalid(error.to_string()))?;
    validate(&contract)?;
    Ok(super::super::imports::surface::DeclaredWorld {
        profile_id: contract.profile_id,
        wit_package: contract.wit.package,
        world: contract.wit.world,
        wit_ref: contract.wit.source_ref,
        imports: contract.allowed_imports,
        exports: contract.allowed_exports,
    })
}

fn validate(contract: &Contract) -> super::super::model::Result<()> {
    let is_identity_bound = contract.schema_id == "molten.wasm-component-effects.v1"
        && contract.schema_version == 1
        && contract.profile_id == super::HOST_PROFILE_ID
        && !contract.is_execution_admitted
        && contract.wit.package == super::HOST_WIT_PACKAGE
        && contract.wit.world == super::HOST_WIT_WORLD
        && contract.wit.source_ref == super::HOST_WIT_SOURCE_REF;
    let is_carrier_bound = contract.request_schema == super::HOST_REQUEST_SCHEMA
        && contract.response_schema == super::HOST_RESPONSE_SCHEMA
        && contract.max_request_bytes == super::HOST_PAYLOAD_BYTES
        && contract.max_response_bytes == super::HOST_PAYLOAD_BYTES
        && contract.max_depth == super::HOST_MAX_DEPTH
        && contract.max_nodes == super::HOST_MAX_NODES;
    let is_linker_closed = contract.allowed_imports == [super::HOST_REQUEST_IMPORT]
        && contract.allowed_exports == [super::super::admission::COMPONENT_INVOKE_EXPORT]
        && contract.allowed_wasi_interfaces.is_empty()
        && matches!(contract.closed_operations.as_slice(), [operation]
            if operation.effect_id == "blob.get" && operation.operation == "get"
                && operation.capability == super::HOST_BLOB_CAPABILITY);
    let is_authority_bound = contract.required_current_inputs
        == [
            "policy",
            "basalt-ucan",
            "generation",
            "incarnation",
            "resources",
            "recorded-effect",
            "request-sequence",
        ]
        && contract.non_claims
            == [
                "carrier-is-not-authority",
                "decoding-is-not-admission",
                "recording-is-not-effect-success",
                "contract-is-not-consumer-installation",
            ];
    if !(is_identity_bound && is_carrier_bound && is_linker_closed && is_authority_bound) {
        return Err(super::invalid("component effect contract differs from the reviewed closed cohort"));
    }
    Ok(())
}
