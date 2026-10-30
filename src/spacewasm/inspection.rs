#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    pub artifact_ref: String,
    pub hostcalls: Vec<String>,
    pub memory_pages: u32,
    pub table_elements: u32,
}

#[derive(Default)]
struct Surface {
    types: Vec<wasmparser::FuncType>,
    functions: Vec<u32>,
    exports: Vec<(String, wasmparser::ExternalKind, u32)>,
    hostcalls: Vec<String>,
    memories: Vec<wasmparser::MemoryType>,
    tables: Vec<wasmparser::TableType>,
}

pub fn inspect(bytes: &[u8], profile: &super::profile::Profile) -> Result<Inspection, String> {
    profile.validate()?;
    if bytes.len() as u64 > profile.limits.ir_bytes {
        return Err("module byte bound exceeded".into());
    }
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::WASM1)
        .validate_all(bytes)
        .map_err(|error| format!("MVP validation: {error}"))?;
    let mut surface = Surface::default();
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        observe(payload.map_err(parse_error)?, &mut surface)?;
    }
    validate_surface(&surface, profile)?;
    Ok(Inspection {
        artifact_ref: format!("blake3:{}", blake3::hash(bytes).to_hex()),
        hostcalls: surface.hostcalls,
        memory_pages: surface.memories[0].maximum.unwrap_or(0) as u32,
        table_elements: surface.tables.first().and_then(|table| table.maximum).unwrap_or(0) as u32,
    })
}

fn observe(payload: wasmparser::Payload<'_>, surface: &mut Surface) -> Result<(), String> {
    match payload {
        wasmparser::Payload::Version { encoding, .. } if encoding != wasmparser::Encoding::Module => {
            return Err("only core modules are admitted".into());
        }
        wasmparser::Payload::TypeSection(section) => {
            for ty in section.into_iter_err_on_gc_types() {
                surface.types.push(ty.map_err(parse_error)?);
            }
        }
        wasmparser::Payload::ImportSection(section) => imports(section, surface)?,
        wasmparser::Payload::FunctionSection(section) => {
            for ty in section {
                surface.functions.push(ty.map_err(parse_error)?);
            }
        }
        wasmparser::Payload::MemorySection(section) => {
            for memory in section {
                surface.memories.push(memory.map_err(parse_error)?);
            }
        }
        wasmparser::Payload::TableSection(section) => {
            for table in section {
                surface.tables.push(table.map_err(parse_error)?.ty);
            }
        }
        wasmparser::Payload::ExportSection(section) => {
            for export in section {
                let export = export.map_err(parse_error)?;
                surface.exports.push((export.name.into(), export.kind, export.index));
            }
        }
        wasmparser::Payload::StartSection { .. } => return Err("start function is forbidden".into()),
        wasmparser::Payload::CodeSectionEntry(body) => {
            let mut reader = body.get_operators_reader().map_err(parse_error)?;
            while !reader.eof() {
                if matches!(reader.read().map_err(parse_error)?, wasmparser::Operator::MemoryGrow { .. }) {
                    return Err("memory.grow is forbidden".into());
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn imports(section: wasmparser::ImportSectionReader<'_>, surface: &mut Surface) -> Result<(), String> {
    for import in section {
        let import = import.map_err(parse_error)?;
        let wasmparser::TypeRef::Func(index) = import.ty else {
            return Err("only function imports are admitted".into());
        };
        if import.module != "molten:hostcall" || !matches!(import.name, "assert" | "send") {
            return Err("undeclared or ambient import".into());
        }
        signature(surface, index, &[wasmparser::ValType::I32, wasmparser::ValType::I32], &[wasmparser::ValType::I64])?;
        if surface.hostcalls.iter().any(|name| name == import.name) {
            return Err("duplicate hostcall import".into());
        }
        surface.hostcalls.push(import.name.into());
        surface.functions.push(index);
    }
    Ok(())
}

fn validate_surface(surface: &Surface, profile: &super::profile::Profile) -> Result<(), String> {
    if surface.memories.len() != 1 || !surface.tables.is_empty() {
        return Err("exactly one memory and no table required".into());
    }
    let memory = surface.memories[0];
    if memory.maximum.is_none_or(|maximum| {
        maximum > u64::from(profile.limits.memory_pages) || memory.initial == 0 || memory.initial > maximum
    }) || memory.memory64
        || memory.shared
        || memory.page_size_log2.is_some()
    {
        return Err("explicit bounded MVP memory maximum required".into());
    }
    if surface.exports.len() != 4 {
        return Err("exact ABI export set required".into());
    }
    export(surface, "memory", None)?;
    export(surface, "molten_alloc", Some((&[wasmparser::ValType::I32], &[wasmparser::ValType::I32])))?;
    export(surface, "molten_dealloc", Some((&[wasmparser::ValType::I32, wasmparser::ValType::I32], &[])))?;
    export(
        surface,
        "molten_run",
        Some((&[wasmparser::ValType::I32, wasmparser::ValType::I32], &[wasmparser::ValType::I64])),
    )
}

fn export(
    surface: &Surface,
    name: &str,
    sig: Option<(&[wasmparser::ValType], &[wasmparser::ValType])>,
) -> Result<(), String> {
    let (_, kind, index) = surface
        .exports
        .iter()
        .find(|export| export.0 == name)
        .ok_or_else(|| format!("missing ABI export {name}"))?;
    if let Some((params, results)) = sig {
        if *kind != wasmparser::ExternalKind::Func {
            return Err(format!("wrong export kind {name}"));
        }
        let ty = surface.functions.get(*index as usize).ok_or("function index out of bounds")?;
        signature(surface, *ty, params, results)
    } else if *kind == wasmparser::ExternalKind::Memory && *index == 0 {
        Ok(())
    } else {
        Err("ABI memory export must select memory zero".into())
    }
}

fn signature(
    surface: &Surface,
    index: u32,
    params: &[wasmparser::ValType],
    results: &[wasmparser::ValType],
) -> Result<(), String> {
    let ty = surface.types.get(index as usize).ok_or("type index out of bounds")?;
    if ty.params() != params || ty.results() != results {
        return Err("ABI function signature mismatch".into());
    }
    Ok(())
}

fn parse_error(error: wasmparser::BinaryReaderError) -> String {
    format!("module inspection: {error}")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
