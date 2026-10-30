use super::super::continuation;

/// Selected final-state projection: actual guest memory bytes and typed global bits.
/// It is captured before cleanup, never reconstructed from a receipt or zero-filled.
pub(super) fn identity(store: &mut ::spacewasm::Store) -> Result<String, continuation::Failure> {
    if store.modules().len() != 1 {
        return Err(continuation::Failure::Proof("terminal-module-state-unavailable".into()));
    }
    let memory = store.get_memory(::spacewasm::ModuleRef(0));
    let bytes = memory
        .load(0, memory.size() as usize * 65536)
        .map_err(|_| continuation::Failure::Proof("terminal-memory-state-unavailable".into()))?;
    let memory_ref = crate::codec::content_ref_from_bytes(bytes);
    let globals = store.modules()[0]
        .globals
        .iter()
        .map(|global| {
            let (kind, bits) = match global.type_.ty {
                ::spacewasm::ValType::I32 => ("i32", u64::from(global.value.read_32())),
                ::spacewasm::ValType::I64 => ("i64", global.value.read_64()),
                ::spacewasm::ValType::F32 => ("f32", u64::from(global.value.read_32())),
                ::spacewasm::ValType::F64 => ("f64", global.value.read_64()),
            };
            crate::codec::record("global", vec![
                crate::codec::string(kind),
                crate::codec::bool_value(global.type_.mutable),
                crate::codec::u64_value(bits),
            ])
        })
        .collect();
    let value = crate::codec::record("spacewasm-final-state-v1", vec![
        crate::codec::string(memory_ref),
        crate::codec::sequence(globals),
    ]);
    crate::codec::canonical_hash(&value).map_err(|_| continuation::Failure::Proof("terminal-state-identity".into()))
}
