const MAX_WASM_IMPORT_EVIDENCE: usize = 1024;

const _: () = assert!(MAX_WASM_IMPORT_EVIDENCE <= 16_384);

#[derive(Debug, Clone, PartialEq, Eq)]
struct WasmInspection {
    module_kind: String,
    imports: Vec<super::observed::WasmImportEvidence>,
}

pub(super) fn wasm_inspection_receipt_value(
    config: &super::model::WasmExecutorConfig,
) -> crate::error::Result<preserves::IOValue> {
    let inspection = inspect_wasm_module(config)?;
    let allowed_hostcalls: &[String] = config.allowed_hostcalls.as_slice();
    Ok(crate::preserves_rail::record("wasm-inspection-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_WASM_INSPECTION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("module-ref", vec![crate::preserves_rail::string(wasm_module_ref(config)?)]),
        crate::preserves_rail::record("module-kind", vec![crate::preserves_rail::string(&inspection.module_kind)]),
        crate::preserves_rail::record("imports", vec![crate::preserves_rail::sequence(
            inspection.imports.iter().map(wasm_import_value).collect(),
        )]),
        crate::preserves_rail::record("wit-ref", vec![crate::preserves_rail::string(wasm_wit_ref(config)?)]),
        crate::preserves_rail::record("allowed-hostcalls", vec![crate::preserves_rail::sequence(
            allowed_hostcalls
                .iter()
                .map(|hostcall: &String| crate::preserves_rail::string(hostcall.as_str()))
                .collect::<Vec<_>>(),
        )]),
        super::calls::hostcall_checks_value(&[
            "module-ref-binding",
            "wasmparser-validated",
            "deny-by-default-wasi",
            "allowed-hostcall-contract",
            "wit-interface-binding",
        ]),
    ]))
}

pub(super) fn validate_wasm_executor_config(
    actor_id: &str,
    config: &super::model::WasmExecutorConfig,
) -> crate::error::Result<()> {
    if config.wit.trim().is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm executor WIT interface for actor {actor_id} is empty"
        )));
    }
    let inspection = inspect_wasm_module(config).map_err(|error| {
        crate::error::Failure::invalid_harness(format!(
            "Wasm executor module for actor {actor_id} failed preflight: {error}"
        ))
    })?;
    validate_wasm_imports(actor_id, &inspection.imports, &config.allowed_hostcalls)
}

fn inspect_wasm_module(config: &super::model::WasmExecutorConfig) -> crate::error::Result<WasmInspection> {
    let bytes = wasm_module_bytes(config)?;
    wasmparser::Validator::new()
        .validate_all(&bytes)
        .map_err(|error| crate::error::Failure::invalid_harness(format!("wasmparser validation failed: {error}")))?;
    let mut module_kind = None;
    let mut imports = Vec::new();
    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        match payload
            .map_err(|error| crate::error::Failure::invalid_harness(format!("wasmparser parse failed: {error}")))?
        {
            wasmparser::Payload::Version { encoding, .. } => {
                module_kind = Some(match encoding {
                    wasmparser::Encoding::Module => "core-module".to_string(),
                    wasmparser::Encoding::Component => "component".to_string(),
                });
            }
            wasmparser::Payload::ImportSection(section) => {
                for import in section {
                    let import = import.map_err(|error| {
                        crate::error::Failure::invalid_harness(format!("wasm import parse failed: {error}"))
                    })?;
                    super::tape::push_bounded(
                        &mut imports,
                        super::observed::WasmImportEvidence {
                            module: import.module.to_string(),
                            name: import.name.to_string(),
                            kind: wasm_type_ref_kind(&import.ty).to_string(),
                        },
                        MAX_WASM_IMPORT_EVIDENCE,
                        "wasm import evidence",
                    )?;
                }
            }
            wasmparser::Payload::ComponentImportSection(section) => {
                for import in section {
                    let import = import.map_err(|error| {
                        crate::error::Failure::invalid_harness(format!("wasm component import parse failed: {error}"))
                    })?;
                    super::tape::push_bounded(
                        &mut imports,
                        super::observed::WasmImportEvidence {
                            module: "component".to_string(),
                            name: import.name.0.to_string(),
                            kind: format!("component:{:?}", import.ty.kind()),
                        },
                        MAX_WASM_IMPORT_EVIDENCE,
                        "wasm import evidence",
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(WasmInspection {
        module_kind: module_kind.unwrap_or_else(|| "unknown".to_string()),
        imports,
    })
}

pub(super) fn validate_wasm_imports(
    actor_id: &str,
    imports: &[super::observed::WasmImportEvidence],
    allowed_hostcalls: &[String],
) -> crate::error::Result<()> {
    for import in imports {
        if import.module == "molten:hostcall" {
            if import.kind != "func" {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "Wasm executor import {}::{} for actor {actor_id} must be a function hostcall",
                    import.module, import.name
                )));
            }
            if !allowed_hostcalls.iter().any(|allowed| allowed == &import.name) {
                return Err(crate::error::Failure::invalid_harness(format!(
                    "Wasm executor import {}::{} for actor {actor_id} is not in allowed hostcalls",
                    import.module, import.name
                )));
            }
            continue;
        }
        if import.module == "component" && allowed_hostcalls.iter().any(|allowed| allowed == &import.name) {
            continue;
        }
        return Err(crate::error::Failure::invalid_harness(format!(
            "Wasm executor import {}::{} for actor {actor_id} is not an allowed Molten hostcall; WASI and ambient imports remain disabled",
            import.module, import.name
        )));
    }
    Ok(())
}

fn wasm_type_ref_kind(ty: &wasmparser::TypeRef) -> &'static str {
    match ty {
        wasmparser::TypeRef::Func(_) => "func",
        wasmparser::TypeRef::Table(_) => "table",
        wasmparser::TypeRef::Memory(_) => "memory",
        wasmparser::TypeRef::Global(_) => "global",
        wasmparser::TypeRef::Tag(_) => "tag",
    }
}

fn wasm_import_value(import: &super::observed::WasmImportEvidence) -> preserves::IOValue {
    crate::preserves_rail::record("import", vec![
        crate::preserves_rail::string(&import.module),
        crate::preserves_rail::string(&import.name),
        crate::preserves_rail::string(&import.kind),
    ])
}

pub fn wasm_module_ref(config: &super::model::WasmExecutorConfig) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("wasm-module-bytes-v1", vec![
        crate::preserves_rail::string(&config.module_hex),
    ]))
}

pub fn wasm_executor_export_name(operation: &str) -> String {
    format!("molten_hostcall_{operation}")
}

pub(super) fn wasm_wit_ref(config: &super::model::WasmExecutorConfig) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::string(&config.wit))
}

pub fn wasm_module_bytes(config: &super::model::WasmExecutorConfig) -> crate::error::Result<Vec<u8>> {
    decode_hex_bytes(&config.module_hex, "Wasm executor module hex")
}

pub(super) fn normalize_hex(input: &str, field: &str) -> crate::error::Result<String> {
    let mut normalized = String::new();
    for character in input.chars() {
        if character.is_ascii_whitespace() || character == '_' {
            continue;
        }
        if !character.is_ascii_hexdigit() {
            return Err(crate::error::Failure::invalid_harness(format!(
                "{field} contains non-hex character {character:?}"
            )));
        }
        normalized.push(character.to_ascii_lowercase());
    }
    if normalized.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!("{field} must not be empty")));
    }
    if !normalized.len().is_multiple_of(2) {
        return Err(crate::error::Failure::invalid_harness(format!(
            "{field} must contain an even number of hex digits"
        )));
    }
    Ok(normalized)
}

fn decode_hex_bytes(input: &str, field: &str) -> crate::error::Result<Vec<u8>> {
    let normalized = normalize_hex(input, field)?;
    let mut bytes = Vec::with_capacity(normalized.len() / 2);
    for index in (0..normalized.len()).step_by(2) {
        let byte = u8::from_str_radix(&normalized[index..index + 2], 16).map_err(|error| {
            crate::error::Failure::invalid_harness(format!("{field} contains invalid byte at offset {index}: {error}"))
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}
