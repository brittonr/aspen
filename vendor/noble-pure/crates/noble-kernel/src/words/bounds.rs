//! Instantiation-bound checks: binding kinds, stack height, type size, and
//! effect-identity count. Every loop here exits by condition; no step returns
//! from inside a loop.

/// Check one binding against its declared kind and the declared bounds.
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; check_binding carries Scheme::check_inst's named stack-entry and type-node limits unchanged into the corresponding kind-specific checks; reassess if these bounds escape that checking chain."
)]
pub(super) fn check_binding(
    binding: &crate::words::Binding,
    kind: &crate::words::VariableKind,
    max_stack: u32,
    max_type: u32,
    max_effects: u64,
) -> Result<(), crate::words::InstError> {
    match (binding, kind) {
        (crate::words::Binding::Stack(segment), crate::words::VariableKind::Stack) => {
            check_segment(segment, max_stack, max_type)
        }
        (crate::words::Binding::Value(ty), crate::words::VariableKind::Value) => {
            check_size(ty, max_type)
        }
        (crate::words::Binding::Effect(set), crate::words::VariableKind::Effect) => {
            check_effect_count(set, max_effects)
        }
        // A reference binding's own shape carries no bound; its terminal
        // binding was bounds-checked at its own position, and the
        // resolution walk checks the kinds match before substituting.
        (crate::words::Binding::Ref(_), _) => Ok(()),
        (crate::words::Binding::Stack(_), crate::words::VariableKind::Value)
        | (crate::words::Binding::Stack(_), crate::words::VariableKind::Effect)
        | (crate::words::Binding::Value(_), crate::words::VariableKind::Stack)
        | (crate::words::Binding::Value(_), crate::words::VariableKind::Effect)
        | (crate::words::Binding::Effect(_), crate::words::VariableKind::Stack)
        | (crate::words::Binding::Effect(_), crate::words::VariableKind::Value) => {
            Err(crate::words::InstError::KindMismatch)
        }
    }
}

/// Check one segment's height and every entry's type size.
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; check_segment checks max_stack against the segment length before passing max_type to the node-size check, preserving the distinct stack/type exhaustion order."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; check_segment uses checked integer-conversion traits and invokes the runtime allocating type-size walk through check_sizes; reassess if that walk becomes const-capable."
)]
fn check_segment(
    segment: &[crate::types::Ty],
    max_stack: u32,
    max_type: u32,
) -> Result<(), crate::words::InstError> {
    let height = match u64::try_from(segment.len()) {
        Ok(height) => height,
        Err(_) => return Err(crate::words::InstError::OversizedStack),
    };
    if height > u64::from(max_stack) {
        return Err(crate::words::InstError::OversizedStack);
    }
    check_sizes(segment, max_type)
}

/// Check every type size in one segment against the declared bound.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; check_sizes bounds indexing by segment length and returns OversizedType for an excessive or unavailable size; hostile type trees must reject rather than trigger assertions."
)]
pub(crate) fn check_sizes(
    segment: &[crate::types::Ty],
    max_type: u32,
) -> Result<(), crate::words::InstError> {
    let mut index = 0;
    let mut failure: Option<crate::words::InstError> = None;
    while index < segment.len() {
        let step = match segment[index].size() {
            Some(size) => {
                if size > max_type {
                    Err(crate::words::InstError::OversizedType)
                } else {
                    Ok(())
                }
            }
            None => Err(crate::words::InstError::OversizedType),
        };
        match step {
            Ok(()) => index += 1,
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

/// Check one value binding's type size against the declared bound.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; check_size invokes Ty::size's owned Vec traversal, which allocates and cannot be const; reassess only if the size representation and walk change."
)]
fn check_size(ty: &crate::types::Ty, max_type: u32) -> Result<(), crate::words::InstError> {
    match ty.size() {
        Some(size) => {
            if size > max_type {
                Err(crate::words::InstError::OversizedType)
            } else {
                Ok(())
            }
        }
        None => Err(crate::words::InstError::OversizedType),
    }
}

/// Check one effect binding's identity count against the declared bound.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; check_effect_count calls EffSet::len, whose checked TryFrom conversion is not stable const; reassess when that conversion becomes const without changing the public API."
)]
fn check_effect_count(
    set: &crate::types::EffSet,
    max_effects: u64,
) -> Result<(), crate::words::InstError> {
    match set.len() {
        Some(count) => {
            if count > max_effects {
                Err(crate::words::InstError::OversizedEffects)
            } else {
                Ok(())
            }
        }
        None => Err(crate::words::InstError::OversizedEffects),
    }
}
