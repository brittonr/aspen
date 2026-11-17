//! Canonical nominal and required-operation contracts; no candidate controls this table.

mod inspection;
mod payload;
mod schemes;
mod r#type {
    pub(in super::super) mod walk;
}
mod validation;

pub(super) fn append(
    mut env: super::Env,
    scheme: crate::words::Scheme,
    kind: super::Behavior,
    owner: u64,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    let index = match u32::try_from(env.defs.len()) {
        Ok(index) => index,
        Err(_) => return Err(super::NominalError::TooManyDefinitions),
    };
    env.defs.push(scheme);
    env.kinds.push(kind);
    env.deps.push(alloc::vec::Vec::new());
    env.definition_owners.push(Some(owner));
    Ok((env, super::Definition(index)))
}

pub(super) fn register(
    env: super::Env,
    decl: super::NominalDecl,
) -> Result<(super::Env, super::NominalOps), super::NominalError> {
    if env.nominal(decl.id).is_some() || env.generic_variant(decl.id).is_some() {
        return Err(super::NominalError::DuplicateIdentity);
    }
    if !valid_representation(&env, &decl) {
        return Err(super::NominalError::InvalidRepresentation);
    }
    if !has_definition_space(&env, &decl) {
        return Err(super::NominalError::TooManyDefinitions);
    }
    let (mut env, ops) = attempt!(register_ops(env, &decl));
    env.nominals.push(decl);
    env.declared_modules = true;
    Ok((env, ops))
}

/// The descriptor of one instantiation is uniquely determined by the
/// declaration and the two ordered type arguments.
pub(super) fn generic_shape(
    decl: &super::GenericVariantDecl,
    args: &[crate::types::Ty; 2],
) -> Option<crate::types::NominalShape> {
    let [left, right] = decl.payload_params;
    if !matches!((left, right), (0, 1) | (1, 0)) {
        return None;
    }
    Some(crate::types::NominalShape::Variant(
        alloc::boxed::Box::new(args[usize::from(left)].clone()),
        alloc::boxed::Box::new(args[usize::from(right)].clone()),
    ))
}

pub(super) fn generic_descriptor_matches(
    decl: &super::GenericVariantDecl,
    args: &[crate::types::Ty; 2],
    shape: &crate::types::NominalShape,
) -> bool {
    let [left, right] = decl.payload_params;
    let crate::types::NominalShape::Variant(first, second) = shape else {
        return false;
    };
    matches!((left, right), (0, 1) | (1, 0))
        && same_generic_arg(first, &args[usize::from(left)])
        && same_generic_arg(second, &args[usize::from(right)])
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Ty has no const PartialEq, measured as E0277 on both pinned compilers"
)]
fn same_generic_arg(left: &crate::types::Ty, right: &crate::types::Ty) -> bool {
    match (left, right) {
        (crate::types::Ty::Unit, crate::types::Ty::Unit)
        | (crate::types::Ty::Bool, crate::types::Ty::Bool)
        | (crate::types::Ty::I64, crate::types::Ty::I64)
        | (crate::types::Ty::Text, crate::types::Ty::Text) => true,
        _ => left == right,
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
pub(super) fn register_generic_variant(
    env: super::Env,
    decl: super::GenericVariantDecl,
) -> Result<(super::Env, super::NominalOps), super::NominalError> {
    if env.nominal(decl.id).is_some() || env.generic_variant(decl.id).is_some() {
        return Err(super::NominalError::DuplicateIdentity);
    }
    if !matches!(decl.payload_params, [0, 1] | [1, 0]) {
        return Err(super::NominalError::InvalidRepresentation);
    }
    if env
        .defs
        .len()
        .checked_add(3)
        .and_then(|n| u32::try_from(n).ok())
        .is_none()
    {
        return Err(super::NominalError::TooManyDefinitions);
    }
    let (env, left) = attempt!(append(
        env,
        schemes::generic_arm(&decl, false),
        super::Behavior::GenericLeft(decl.id),
        decl.id.module,
    ));
    let (env, right) = attempt!(append(
        env,
        schemes::generic_arm(&decl, true),
        super::Behavior::GenericRight(decl.id),
        decl.id.module,
    ));
    let (mut env, matcher) = attempt!(append(
        env,
        schemes::generic_matcher(&decl),
        super::Behavior::GenericMatch(decl.id),
        decl.id.module,
    ));
    env.generic_variants.push(decl);
    env.declared_modules = true;
    Ok((
        env,
        super::NominalOps {
            new: None,
            into: None,
            left: Some(left),
            right: Some(right),
            matcher: Some(matcher),
        },
    ))
}

fn register_ops(
    env: super::Env,
    decl: &super::NominalDecl,
) -> Result<(super::Env, super::NominalOps), super::NominalError> {
    if matches!(decl.shape, crate::types::NominalShape::Opaque(_)) {
        opaque_ops(env, decl)
    } else if matches!(decl.shape, crate::types::NominalShape::Variant(_, _)) {
        variant_ops(env, decl)
    } else {
        Err(super::NominalError::InvalidRepresentation)
    }
}

fn valid_representation(env: &super::Env, decl: &super::NominalDecl) -> bool {
    if let crate::types::NominalShape::Opaque(representation) = &decl.shape {
        return env.valid_nominal_payload(representation, 512);
    }
    if let crate::types::NominalShape::Variant(left, right) = &decl.shape {
        return env.valid_nominal_payload(left, 512) && env.valid_nominal_payload(right, 512);
    }
    false
}

fn has_definition_space(env: &super::Env, decl: &super::NominalDecl) -> bool {
    let count = if matches!(decl.shape, crate::types::NominalShape::Opaque(_)) {
        2usize
    } else if matches!(decl.shape, crate::types::NominalShape::Variant(_, _)) {
        3
    } else {
        return false;
    };
    env.defs
        .len()
        .checked_add(count)
        .and_then(|n| u32::try_from(n).ok())
        .is_some()
}

fn opaque_ops(
    env: super::Env,
    decl: &super::NominalDecl,
) -> Result<(super::Env, super::NominalOps), super::NominalError> {
    let (env, new) = attempt!(append_conversion(env, decl, false));
    let (env, into) = attempt!(append_conversion(env, decl, true));
    Ok((
        env,
        super::NominalOps {
            new: Some(new),
            into: Some(into),
            left: None,
            right: None,
            matcher: None,
        },
    ))
}

fn append_conversion(
    env: super::Env,
    decl: &super::NominalDecl,
    into: bool,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    let Some(scheme) = schemes::conversion(&env, decl, into) else {
        return Err(super::NominalError::InvalidRepresentation);
    };
    let kind = if into {
        super::Behavior::NominalInto(decl.id)
    } else {
        super::Behavior::NominalNew(decl.id)
    };
    append(env, scheme, kind, decl.id.module)
}

fn variant_ops(
    env: super::Env,
    decl: &super::NominalDecl,
) -> Result<(super::Env, super::NominalOps), super::NominalError> {
    let (env, left) = attempt!(append_arm(env, decl, false));
    let (env, right) = attempt!(append_arm(env, decl, true));
    let (env, matcher) = attempt!(append_matcher(env, decl));
    Ok((
        env,
        super::NominalOps {
            new: None,
            into: None,
            left: Some(left),
            right: Some(right),
            matcher: Some(matcher),
        },
    ))
}

fn append_arm(
    env: super::Env,
    decl: &super::NominalDecl,
    right: bool,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    let Some(scheme) = schemes::arm(&env, decl, right) else {
        return Err(super::NominalError::InvalidRepresentation);
    };
    let kind = if right {
        super::Behavior::NominalRight(decl.id)
    } else {
        super::Behavior::NominalLeft(decl.id)
    };
    append(env, scheme, kind, decl.id.module)
}

fn append_matcher(
    env: super::Env,
    decl: &super::NominalDecl,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    let Some(scheme) = schemes::matcher(&env, decl) else {
        return Err(super::NominalError::InvalidRepresentation);
    };
    append(
        env,
        scheme,
        super::Behavior::NominalMatch(decl.id),
        decl.id.module,
    )
}

pub(super) fn register_bound_emit(
    env: super::Env,
    registration: super::BoundEmitRegistration,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    let registration = attempt!(checked_bound_registration(&env, registration));
    let (mut env, def) = attempt!(append(
        env,
        schemes::emit_scheme(),
        super::Behavior::BoundEmit(registration.adapter_slot),
        registration.owner,
    ));
    env.bound_adapters.push(bound_adapter(def, registration));
    env.declared_modules = true;
    Ok((env, def))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
pub(super) fn register_bound_clock(
    env: super::Env,
    registration: super::BoundClockRegistration,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    let has_identity = !registration.adapter_identity.is_empty();
    let has_input = !registration.input.is_empty();
    let is_i64_returning = registration.output.as_slice() == [crate::types::Ty::I64];
    let has_test_clock_effect = registration.effects.as_slice() == [super::TEST_CLOCK];
    let is_registered = env
        .bound_adapters
        .iter()
        .any(|row| row.adapter_slot == registration.adapter_slot);
    let is_clock_shape = has_identity && !has_input;
    let is_clock_signature = is_i64_returning && has_test_clock_effect;
    if !(is_clock_shape && is_clock_signature) || is_registered {
        return Err(super::NominalError::InvalidRepresentation);
    }
    let (mut env, def) = attempt!(append(
        env,
        schemes::clock_scheme(),
        super::Behavior::BoundClock(registration.adapter_slot),
        registration.owner,
    ));
    env.bound_adapters.push(super::BoundAdapter {
        definition: def,
        adapter_identity: registration.adapter_identity,
        adapter_slot: registration.adapter_slot,
        input: registration.input,
        output: registration.output,
        effects: registration.effects,
    });
    if !env.effects.contains(&super::TEST_CLOCK) {
        env.effects.push(super::TEST_CLOCK);
    }
    env.declared_modules = true;
    Ok((env, def))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::compound_condition,
    reason = "Owner: noble-kernel; registration admits only the exact reserved code-export identity, Text Text input, empty output, single effect and an unused adapter slot, and rejects any other registration as a typed NominalError; the operands are side-effect free."
)]
pub(super) fn register_bound_code_export(
    env: super::Env,
    registration: super::BoundCodeExportRegistration,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    if registration.adapter_identity != super::TEST_CODE_EXPORT_ADAPTER_IDENTITY
        || registration.input.as_slice() != [crate::types::Ty::Text, crate::types::Ty::Text]
        || !registration.output.is_empty()
        || registration.effects.as_slice() != [super::TEST_CODE_EXPORT]
        || env
            .bound_adapters
            .iter()
            .any(|row| row.adapter_slot == registration.adapter_slot)
    {
        return Err(super::NominalError::InvalidRepresentation);
    }
    let (mut env, def) = attempt!(append(
        env,
        schemes::code_export_scheme(),
        super::Behavior::BoundCodeExport(registration.adapter_slot),
        registration.owner,
    ));
    env.bound_adapters.push(super::BoundAdapter {
        definition: def,
        adapter_identity: registration.adapter_identity,
        adapter_slot: registration.adapter_slot,
        input: registration.input,
        output: registration.output,
        effects: registration.effects,
    });
    if !env.effects.contains(&super::TEST_CODE_EXPORT) {
        env.effects.push(super::TEST_CODE_EXPORT);
    }
    env.declared_modules = true;
    Ok((env, def))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::compound_condition,
    reason = "Owner: noble-kernel; registration admits only the exact reserved send identity, one Text input, empty output, single effect and an unused adapter slot, and rejects any other registration as a typed NominalError; the operands are side-effect free."
)]
pub(super) fn register_bound_send(
    env: super::Env,
    registration: super::BoundSendRegistration,
) -> Result<(super::Env, super::Definition), super::NominalError> {
    if registration.adapter_identity != super::TEST_SEND_ADAPTER_IDENTITY
        || registration.input.as_slice() != [crate::types::Ty::Text]
        || !registration.output.is_empty()
        || registration.effects.as_slice() != [super::TEST_SEND]
        || env
            .bound_adapters
            .iter()
            .any(|row| row.adapter_slot == registration.adapter_slot)
    {
        return Err(super::NominalError::InvalidRepresentation);
    }
    let (mut env, def) = attempt!(append(
        env,
        schemes::send_scheme(),
        super::Behavior::BoundSend(registration.adapter_slot),
        registration.owner,
    ));
    env.bound_adapters.push(super::BoundAdapter {
        definition: def,
        adapter_identity: registration.adapter_identity,
        adapter_slot: registration.adapter_slot,
        input: registration.input,
        output: registration.output,
        effects: registration.effects,
    });
    if !env.effects.contains(&super::TEST_SEND) {
        env.effects.push(super::TEST_SEND);
    }
    env.declared_modules = true;
    Ok((env, def))
}

fn checked_bound_registration(
    env: &super::Env,
    registration: super::BoundEmitRegistration,
) -> Result<super::BoundEmitRegistration, super::NominalError> {
    if registration.adapter_identity.is_empty()
        || registration.input.as_slice() != [crate::types::Ty::Text]
    {
        return Err(super::NominalError::InvalidRepresentation);
    }
    if !registration.output.is_empty() || registration.effects.as_slice() != [super::TEST_EMIT] {
        return Err(super::NominalError::InvalidRepresentation);
    }
    let mut index = 0;
    while index < env.bound_adapters.len() {
        if env.bound_adapters[index].adapter_slot == registration.adapter_slot {
            return Err(super::NominalError::InvalidRepresentation);
        }
        index += 1;
    }
    Ok(registration)
}

fn bound_adapter(
    definition: super::Definition,
    row: super::BoundEmitRegistration,
) -> super::BoundAdapter {
    super::BoundAdapter {
        definition,
        adapter_identity: row.adapter_identity,
        adapter_slot: row.adapter_slot,
        input: row.input,
        output: row.output,
        effects: row.effects,
    }
}
