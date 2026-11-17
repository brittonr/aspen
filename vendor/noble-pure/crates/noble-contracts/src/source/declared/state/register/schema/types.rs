pub(super) fn parse_shape(
    schema: &crate::source::declared::Schema,
    local_types: &[(alloc::string::String, noble_kernel::types::Ty)],
    families: &[(alloc::string::String, noble_kernel::types::NominalTypeId)],
    environment: Option<&noble_kernel::contracts::Env>,
    mut meter: crate::Meter,
) -> (
    crate::Meter,
    Result<Option<(noble_kernel::types::NominalShape, usize)>, crate::source::Error>,
) {
    let word_len = type_word_len(schema);
    debug_assert!(!schema.name.is_empty());
    debug_assert!(word_len > 0);
    let bytes = match u32::try_from(word_len) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                meter,
                Err(crate::source::declared::error(
                    crate::source::Stage::Check,
                    "type word length overflow",
                )),
            )
        }
    };
    let parsed = match meter.charge(bytes, crate::source::declared::SPAN) {
        Ok(()) => Ok(shape(schema, local_types, families, environment).ok()),
        Err(problem) => Err(crate::source::declared::diagnostic(
            crate::source::Stage::Check,
            problem,
        )),
    };
    (meter, parsed)
}

const fn type_word_len(schema: &crate::source::declared::Schema) -> usize {
    match &schema.kind {
        crate::source::declared::SchemaKind::Opaque { base, .. } => base.len(),
        crate::source::declared::SchemaKind::Variant {
            left_type,
            right_type,
            ..
        } => left_type.len().saturating_add(right_type.len()),
    }
}

pub(super) fn shape(
    schema: &crate::source::declared::Schema,
    local_types: &[(alloc::string::String, noble_kernel::types::Ty)],
    families: &[(alloc::string::String, noble_kernel::types::NominalTypeId)],
    environment: Option<&noble_kernel::contracts::Env>,
) -> Result<(noble_kernel::types::NominalShape, usize), crate::Diagnostic> {
    match &schema.kind {
        crate::source::declared::SchemaKind::Opaque { base, .. } => {
            let parsed = attempt!(crate::source::declared::types::parse_with_families(
                base,
                local_types,
                families,
                environment,
                crate::source::declared::SPAN,
            ));
            Ok((
                noble_kernel::types::NominalShape::Opaque(alloc::boxed::Box::new(parsed.ty)),
                parsed.nodes,
            ))
        }
        crate::source::declared::SchemaKind::Variant {
            left_type,
            right_type,
            ..
        } => {
            let left = attempt!(crate::source::declared::types::parse_with_families(
                left_type,
                local_types,
                families,
                environment,
                crate::source::declared::SPAN,
            ));
            let right = attempt!(crate::source::declared::types::parse_with_families(
                right_type,
                local_types,
                families,
                environment,
                crate::source::declared::SPAN,
            ));
            Ok((
                noble_kernel::types::NominalShape::Variant(
                    alloc::boxed::Box::new(left.ty),
                    alloc::boxed::Box::new(right.ty),
                ),
                left.nodes.saturating_add(right.nodes),
            ))
        }
    }
}

pub(super) fn public_operations(
    schema: &crate::source::declared::Schema,
    exports: &[alloc::string::String],
    is_exported: bool,
) -> [bool; 2] {
    if !is_exported {
        return [false; 2];
    }
    let (suffixes, declared_public) = match &schema.kind {
        crate::source::declared::SchemaKind::Opaque { public, .. } => {
            (["new", "into"], [*public; 2])
        }
        crate::source::declared::SchemaKind::Variant {
            left,
            left_public,
            right,
            right_public,
            ..
        } => (
            [left.as_str(), right.as_str()],
            [*left_public, *right_public],
        ),
    };
    debug_assert!(!suffixes[0].is_empty());
    debug_assert!(!suffixes[1].is_empty());
    exported_operations(schema, exports, suffixes, declared_public)
}

fn exported_operations(
    schema: &crate::source::declared::Schema,
    exports: &[alloc::string::String],
    suffixes: [&str; 2],
    declared_public: [bool; 2],
) -> [bool; 2] {
    let mut public = [false; 2];
    let mut entry_index = 0;
    while entry_index < exports.len()
        && ((declared_public[0] && !public[0]) || (declared_public[1] && !public[1]))
    {
        let matched = matches_operation(&exports[entry_index], schema, suffixes);
        if declared_public[0] && matched[0] {
            public[0] = true;
        }
        if declared_public[1] && matched[1] {
            public[1] = true;
        }
        entry_index += 1;
    }
    public
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; E0015 split_once_ascii and str PartialEq plus E0658 const PartialEq prevent const on both pinned Rust compilers; reassess const operation matching."
)]
fn matches_operation(
    export: &str,
    schema: &crate::source::declared::Schema,
    suffixes: [&str; 2],
) -> [bool; 2] {
    if let Some((base, suffix)) = crate::source::declared::split_once_ascii(export, b'.') {
        if base == schema.name {
            return [suffix == suffixes[0], suffix == suffixes[1]];
        }
    }
    [false; 2]
}
