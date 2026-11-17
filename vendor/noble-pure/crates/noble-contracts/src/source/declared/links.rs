//! Validate exact adapter bindings and qualified exported words.

pub(super) fn alias_conflicts_definition(
    module: &super::Module,
    alias: &str,
    definition: &crate::source::Named,
) -> bool {
    definition.owner.is_none() && alias_word(module, alias.as_bytes(), &definition.name)
}
pub(super) fn alias_word(module: &super::Module, alias: &[u8], name: &str) -> bool {
    match super::strip_qualified(name, alias, b'.') {
        Some(word) => has_word(module, word),
        None => false,
    }
}
pub(super) fn module_word(module: &super::Module, name: &str) -> bool {
    let Some(rest) = super::strip_qualified(name, module.name.as_bytes(), b'@') else {
        return false;
    };
    let Some((version, word)) = super::split_once_ascii(rest, b'.') else {
        return false;
    };
    super::canonical_version(version) == Some(module.version) && has_word(module, word)
}

fn has_word(module: &super::Module, word: &str) -> bool {
    let mut entry_slot = 0usize;
    let mut is_found = false;
    while entry_slot < module.exports.len() && !is_found {
        is_found = module.exports[entry_slot]
            .words
            .iter()
            .any(|(name, _)| name == word);
        entry_slot += 1;
    }
    is_found
}
pub(super) fn valid_binding(binding: &super::BoundOperation) -> bool {
    super::parsing::name(&binding.module_name)
        && binding.module_version != 0
        && matches!(
            binding.operation.as_str(),
            "test.emit" | "test.clock" | "test.code-export" | "test.send"
        )
        && !binding.adapter_identity.is_empty()
}
pub(super) fn duplicate_binding(
    binding: &super::BoundOperation,
    previous: &super::BoundOperation,
) -> bool {
    (previous.module_name == binding.module_name
        && previous.module_version == binding.module_version
        && previous.operation == binding.operation)
        || previous.adapter_slot == binding.adapter_slot
}
