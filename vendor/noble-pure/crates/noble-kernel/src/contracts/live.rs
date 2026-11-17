//! Host-registered nominal resources and borrowed live-slot signatures.

fn resource_kind(decl: &super::NominalDecl) -> Option<crate::types::ResourceKind> {
    let crate::types::NominalShape::Opaque(representation) = &decl.shape else {
        return None;
    };
    let crate::types::Ty::Resource(kind) = &**representation else {
        return None;
    };
    Some(*kind)
}

pub(super) fn register_resource(
    mut env: super::Env,
    decl: super::NominalDecl,
) -> Result<super::Env, super::NominalError> {
    let kind = resource_kind(&decl).ok_or(super::NominalError::InvalidRepresentation)?;
    let is_live = env.live_slots;
    let is_exported = decl.exported;
    let is_unpublished = decl.public == [false, false];
    let is_fixture_kind = kind == super::FIXTURE_RESOURCE;
    let is_registered = env.resource_kinds.contains(&kind);
    let is_registrable = is_live && is_exported && is_unpublished;
    if !is_registrable || is_fixture_kind || is_registered {
        return Err(super::NominalError::InvalidRepresentation);
    }
    if env.nominal(decl.id).is_some() || env.generic_variant(decl.id).is_some() {
        return Err(super::NominalError::DuplicateIdentity);
    }
    env.resource_kinds.push(kind);
    env.live_resource_nominals.push(decl.id);
    env.nominals.push(decl);
    Ok(env)
}

/// Check the catalog itself, not just its registration path: `super::Env` fields
/// are public and may have been altered after host registration.
pub(super) fn catalog_valid(env: &super::Env) -> bool {
    if !env.live_slots {
        return env.live_resource_nominals.is_empty();
    }
    if env.resource_kinds.len() != env.live_resource_nominals.len().saturating_add(1)
        || env.resource_kinds.first() != Some(&super::FIXTURE_RESOURCE)
    {
        return false;
    }
    let mut index = 0;
    let mut is_valid = true;
    while index < env.live_resource_nominals.len() && is_valid {
        is_valid = catalog_row_valid(env, index);
        index += 1;
    }
    is_valid
}

/// Validate a resource-catalog row against its declaration and kind.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; catalog validation uses non-const Env::nominal lookup and resource-kind comparison against the host-registered declaration."
)]
fn catalog_row_matches(env: &super::Env, index: usize) -> bool {
    let Some(decl) = env.nominal(env.live_resource_nominals[index]) else {
        return false;
    };
    let kind = env.resource_kinds[index.saturating_add(1)];
    decl.exported && decl.public == [false, false] && resource_kind(decl) == Some(kind)
}

/// Validate a resource-catalog row against every earlier row.
fn catalog_row_is_unique(env: &super::Env, index: usize) -> bool {
    let kind = env.resource_kinds[index.saturating_add(1)];
    let nominal = env.live_resource_nominals[index];
    let mut prior = 0;
    let mut is_unique = true;
    while prior < index && is_unique {
        if env.resource_kinds[prior.saturating_add(1)] == kind
            || env.live_resource_nominals[prior] == nominal
        {
            is_unique = false;
        }
        prior += 1;
    }
    is_unique
}

/// Validate one resource-catalog row against its declaration, its kind and every
/// earlier row. The duplicate scan accumulates its verdict so Aeneas can extract
/// the bounded loop without returns or control-flow exits inside it.
fn catalog_row_valid(env: &super::Env, index: usize) -> bool {
    catalog_row_matches(env, index) && catalog_row_is_unique(env, index)
}

pub(super) fn kind_is_slot_resource(env: &super::Env, kind: crate::types::ResourceKind) -> bool {
    if !env.live_slots {
        return false;
    }
    let mut index = 1;
    while index < env.resource_kinds.len() {
        if env.resource_kinds[index] == kind {
            return true;
        }
        index += 1;
    }
    false
}

/// Only root-positioned borrowed references may be used as authority. Their
/// signatures are finite concrete trees; a borrowed input may itself be a
/// root-provided reference, but no result or value child may contain one.
///
/// Return the known-effect flag for one ordered effect list without returning
/// from inside a loop.
pub(super) fn effects_known(env: &super::Env, effects: &[crate::types::EffId]) -> bool {
    let mut index = 0;
    let mut is_known = true;
    while index < effects.len() && is_known {
        if !env.knows_effect(effects[index]) {
            is_known = false;
        }
        index += 1;
    }
    is_known
}

/// Validate one ordered type list at the fixed 512 nesting budget, accumulating
/// the verdict instead of returning from inside the loop.
pub(super) fn types_valid(env: &super::Env, types: &[crate::types::Ty]) -> bool {
    let mut index = 0;
    let mut is_valid = true;
    while index < types.len() && is_valid {
        is_valid = env.valid_type(&types[index], 512);
        index += 1;
    }
    is_valid
}

/// Queue one live-reference input's forwarded references without changing order.
/// The owned vector is returned so its allocation is reused by the outer walk.
fn queue_forwarded_refs<'a>(
    env: &super::Env,
    input: &'a [crate::types::Ty],
    mut pending: alloc::vec::Vec<&'a crate::types::Ty>,
) -> (alloc::vec::Vec<&'a crate::types::Ty>, bool) {
    let mut index = 0;
    let mut is_queued = true;
    while index < input.len() && is_queued {
        let value = &input[index];
        if matches!(value, crate::types::Ty::LiveRef(_, _, _)) {
            if pending.len() >= 512 {
                is_queued = false;
            } else {
                pending.push(value);
            }
        } else if !env.valid_type(value, 512) {
            is_queued = false;
        }
        index += 1;
    }
    (pending, is_queued)
}

/// Validate one reachable `LiveRef` node and queue its forwarded references.
/// The owned pending vector makes the state transition explicit while keeping
/// its allocation and stack order intact for the bounded outer walk.
pub(super) fn ref_node_valid<'a>(
    env: &super::Env,
    current: &'a crate::types::Ty,
    pending: alloc::vec::Vec<&'a crate::types::Ty>,
) -> (alloc::vec::Vec<&'a crate::types::Ty>, bool) {
    let crate::types::Ty::LiveRef(input, output, effects) = current else {
        return (pending, false);
    };
    if input.len().saturating_add(output.len()) > 512 {
        return (pending, false);
    }
    if !effects_known(env, effects.as_slice()) {
        return (pending, false);
    }
    if !types_valid(env, output) {
        return (pending, false);
    }
    queue_forwarded_refs(env, input.as_slice(), pending)
}

/// Advance one bounded live-reference step, returning the next queued node.
fn ref_step<'a>(
    env: &super::Env,
    current: &'a crate::types::Ty,
    visited: usize,
    pending: alloc::vec::Vec<&'a crate::types::Ty>,
) -> (
    alloc::vec::Vec<&'a crate::types::Ty>,
    Option<&'a crate::types::Ty>,
    bool,
) {
    if visited >= 512 {
        return (pending, None, false);
    }
    let (mut pending, is_valid) = ref_node_valid(env, current, pending);
    if is_valid {
        let next = pending.pop();
        (pending, next, true)
    } else {
        (pending, None, false)
    }
}

pub(super) fn valid_ref(env: &super::Env, ty: &crate::types::Ty) -> bool {
    if !env.live_slots {
        return false;
    }
    let mut pending: alloc::vec::Vec<&crate::types::Ty> = alloc::vec::Vec::new();
    let mut next = Some(ty);
    let mut visited = 0usize;
    let mut is_valid = true;
    while let Some(current) = next {
        (pending, next, is_valid) = ref_step(env, current, visited, pending);
        if is_valid {
            visited += 1;
        }
    }
    is_valid
}
