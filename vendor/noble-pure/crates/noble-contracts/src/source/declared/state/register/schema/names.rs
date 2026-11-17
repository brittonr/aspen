pub(super) fn available(
    session: &crate::source::declared::ModuleSession,
    schema_count: usize,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let type_slots = attempt!(capacity(session, schema_count));
    let local_types = alloc::vec::Vec::with_capacity(type_slots);
    let local_types = attempt!(from_aliases(session, local_types));
    from_modules(session, local_types)
}

fn from_aliases(
    session: &crate::source::declared::ModuleSession,
    local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let mut alias_slot = 0;
    let mut outcome = Ok(local_types);
    while alias_slot < session.aliases.len() && outcome.is_ok() {
        outcome = match outcome {
            Ok(local_types) => alias_exports(session, alias_slot, local_types),
            Err(problem) => Err(problem),
        };
        alias_slot += 1;
    }
    outcome
}

fn alias_exports(
    session: &crate::source::declared::ModuleSession,
    alias_slot: usize,
    local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let alias = &session.aliases[alias_slot];
    if alias.pending {
        return Ok(local_types);
    }
    let Some(module) = session.modules.get(alias.module) else {
        return Ok(local_types);
    };
    alias_export_types(module, alias, local_types)
}

fn alias_export_types(
    module: &crate::source::declared::Module,
    alias: &crate::source::declared::Alias,
    local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let mut entry_slot = 0;
    let mut outcome = Ok(local_types);
    while entry_slot < module.exports.len() && outcome.is_ok() {
        outcome = match outcome {
            Ok(local_types) => append_alias_type(local_types, alias, &module.exports[entry_slot]),
            Err(problem) => Err(problem),
        };
        entry_slot += 1;
    }
    outcome
}

fn append_alias_type(
    mut local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
    alias: &crate::source::declared::Alias,
    export: &crate::source::declared::Export,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let Some(ty) = &export.ty else {
        return Ok(local_types);
    };
    let spelling = attempt!(alias_spelling(alias, export));
    local_types.push((spelling, ty.clone()));
    Ok(local_types)
}

fn alias_spelling(
    alias: &crate::source::declared::Alias,
    export: &crate::source::declared::Export,
) -> Result<alloc::string::String, crate::source::Error> {
    let Some(capacity_bytes) = bounded_spelling_capacity(
        alias
            .spelling
            .len()
            .checked_add(1)
            .and_then(|length| length.checked_add(export.name.len())),
    ) else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "aliased export word length overflow",
        ));
    };
    let mut spelling = alloc::string::String::with_capacity(capacity_bytes);
    spelling.push_str(&alias.spelling);
    spelling.push('.');
    spelling.push_str(&export.name);
    Ok(spelling)
}

fn from_modules(
    session: &crate::source::declared::ModuleSession,
    local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let mut module_slot = 0;
    let mut outcome = Ok(local_types);
    while module_slot < session.modules.len() && outcome.is_ok() {
        outcome = match outcome {
            Ok(local_types) => qualified_exports(&session.modules[module_slot], local_types),
            Err(problem) => Err(problem),
        };
        module_slot += 1;
    }
    outcome
}

fn qualified_exports(
    module: &crate::source::declared::Module,
    local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let version_width = version_width(module.version);
    let mut entry_slot = 0;
    let mut outcome = Ok(local_types);
    while entry_slot < module.exports.len() && outcome.is_ok() {
        outcome = match outcome {
            Ok(local_types) => append_qualified_type(
                local_types,
                module,
                &module.exports[entry_slot],
                version_width,
            ),
            Err(problem) => Err(problem),
        };
        entry_slot += 1;
    }
    outcome
}

fn append_qualified_type(
    mut local_types: alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>,
    module: &crate::source::declared::Module,
    export: &crate::source::declared::Export,
    version_width: usize,
) -> Result<alloc::vec::Vec<(alloc::string::String, noble_kernel::types::Ty)>, crate::source::Error>
{
    let Some(ty) = &export.ty else {
        return Ok(local_types);
    };
    let spelling = attempt!(qualified_spelling(module, export, version_width));
    local_types.push((spelling, ty.clone()));
    Ok(local_types)
}

const fn version_width(mut remaining: u32) -> usize {
    let mut width = 1;
    while remaining >= 10 {
        width += 1;
        remaining /= 10;
    }
    width
}

fn qualified_spelling(
    module: &crate::source::declared::Module,
    export: &crate::source::declared::Export,
    version_width: usize,
) -> Result<alloc::string::String, crate::source::Error> {
    let capacity_bytes = attempt!(qualified_capacity(module, export, version_width));
    let mut spelling = alloc::string::String::with_capacity(capacity_bytes);
    spelling.push_str(&module.name);
    spelling.push('@');
    spelling = crate::source::declared::append_decimal(spelling, module.version);
    spelling.push('.');
    spelling.push_str(&export.name);
    Ok(spelling)
}

fn qualified_capacity(
    module: &crate::source::declared::Module,
    export: &crate::source::declared::Export,
    version_width: usize,
) -> Result<usize, crate::source::Error> {
    let capacity_bytes = module
        .name
        .len()
        .checked_add(1)
        .and_then(|length| length.checked_add(version_width))
        .and_then(|length| length.checked_add(1))
        .and_then(|length| length.checked_add(export.name.len()));
    let Some(capacity_bytes) = bounded_spelling_capacity(capacity_bytes) else {
        return Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "qualified export word length overflow",
        ));
    };
    Ok(capacity_bytes)
}

const fn bounded_spelling_capacity(capacity_bytes: Option<usize>) -> Option<usize> {
    let Some(capacity_bytes) = capacity_bytes else {
        return None;
    };
    if capacity_bytes > isize::MAX as usize {
        return None;
    }
    Some(capacity_bytes)
}

fn capacity(
    session: &crate::source::declared::ModuleSession,
    schema_count: usize,
) -> Result<usize, crate::source::Error> {
    let total = alias_capacity(session, Some(schema_count));
    checked_type_slots(module_capacity(session, total))
}

fn alias_capacity(
    session: &crate::source::declared::ModuleSession,
    mut total: Option<usize>,
) -> Option<usize> {
    let mut alias_slot = 0;
    while alias_slot < session.aliases.len() {
        let module_slot = session.aliases[alias_slot].module;
        let entries_count = if module_slot < session.modules.len() {
            session.modules[module_slot].exports.len()
        } else {
            0
        };
        total = total.and_then(|count| count.checked_add(entries_count));
        alias_slot += 1;
    }
    total
}

fn module_capacity(
    session: &crate::source::declared::ModuleSession,
    mut total: Option<usize>,
) -> Option<usize> {
    let mut module_slot = 0;
    while module_slot < session.modules.len() {
        total =
            total.and_then(|count| count.checked_add(session.modules[module_slot].exports.len()));
        module_slot += 1;
    }
    total
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0277/E0658 Option::and_then const FnOnce and E0015 declared::error prevent const on both pinned Rust compilers; reassess const schema capacity errors."
)]
fn checked_type_slots(total: Option<usize>) -> Result<usize, crate::source::Error> {
    let byte_slots = total.and_then(|count| {
        count.checked_mul(
            const { core::mem::size_of::<(alloc::string::String, noble_kernel::types::Ty)>() },
        )
    });
    match (total, byte_slots) {
        (Some(type_slots), Some(bytes)) if bytes <= isize::MAX as usize => Ok(type_slots),
        _ => Err(crate::source::declared::error(
            crate::source::Stage::Check,
            "schema name capacity overflow",
        )),
    }
}
