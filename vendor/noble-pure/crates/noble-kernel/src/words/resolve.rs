//! Iterative resolution of reference bindings in one instantiation witness
//! (B-CHECK-05).
//!
//! A `Ref` binding states a type equation: this variable's witness is
//! another variable's witness. Chains must be finite and well founded — a
//! chain that returns to a variable already on its own path is cyclic and
//! rejects. Every hop charges the declared work limit before it is taken,
//! and the walk threads its output and meter as returned state, so no loop
//! body returns early.

/// The state one binding resolution threads through.
struct Walk {
    /// Resolved bindings, one per input binding, in order.
    out: alloc::vec::Vec<crate::words::Binding>,
    /// Work units spent on hops so far.
    spent: u32,
}

/// The declared kind of one binding position, when it exists.
fn kind_at(
    kinds: &[crate::words::VariableKind],
    index: usize,
) -> Option<crate::words::VariableKind> {
    kinds.get(index).copied()
}

/// The index of one variable's binding position, when it fits.
fn slot_of(var: crate::words::Variable) -> Option<usize> {
    usize::try_from(var.0).ok()
}

/// Whether one binding carries exactly this declared kind.
const fn carries_kind(binding: &crate::words::Binding, kind: crate::words::VariableKind) -> bool {
    match (binding, kind) {
        (crate::words::Binding::Stack(_), crate::words::VariableKind::Stack)
        | (crate::words::Binding::Value(_), crate::words::VariableKind::Value)
        | (crate::words::Binding::Effect(_), crate::words::VariableKind::Effect) => true,
        (crate::words::Binding::Ref(_), _)
        | (crate::words::Binding::Stack(_), _)
        | (crate::words::Binding::Value(_), _)
        | (crate::words::Binding::Effect(_), _) => false,
    }
}

/// Resolve every reference binding of one instantiation (B-CHECK-05).
///
/// Returns the witness with every reference replaced by its terminal
/// binding and the work spent following chains. A cyclic chain rejects;
/// a chain that would exceed `fuel` hops rejects fail-closed.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; bindings preserves input order and threads step_binding's typed reference, kind, cycle and fuel failures; invalid witnesses must remain rejection outcomes rather than assertion panics."
)]
pub fn bindings(
    kinds: &[crate::words::VariableKind],
    inst: &crate::words::Inst,
    fuel: u32,
) -> Result<(crate::words::Inst, u32), crate::words::InstError> {
    let mut walk = Walk {
        out: alloc::vec::Vec::with_capacity(crate::capacity::at_least(inst.bindings.len(), 4)),
        spent: 0,
    };
    let mut failure: Option<crate::words::InstError> = None;
    let mut index: usize = 0;
    while index < inst.bindings.len() {
        let (next, step) = step_binding(kinds, inst, index, walk, fuel);
        walk = next;
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
        None => Ok((crate::words::Inst { bindings: walk.out }, walk.spent)),
    }
}

/// Resolve one binding: a direct binding is kept, a reference follows its
/// chain to a terminal binding of the same declared kind.
///
/// A chain longer than the witness itself must revisit a binding position
/// (pigeonhole), so the walk reports a cycle without extra state.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; bindings calls step_binding only with index < inst.bindings.len(), and reference resolution validates terminal membership/kind through typed failures; no additional assertion is needed."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; step_binding clones direct bindings into its Vec or invokes the runtime reference walk; these owned allocation/drop operations cannot be const."
)]
fn step_binding(
    kinds: &[crate::words::VariableKind],
    inst: &crate::words::Inst,
    index: usize,
    mut walk: Walk,
    fuel: u32,
) -> (Walk, Result<(), crate::words::InstError>) {
    let declared = kind_at(kinds, index);
    match &inst.bindings[index] {
        crate::words::Binding::Ref(var) => {
            let (next, found) = follow_chain(inst, *var, walk, fuel);
            walk = next;
            match found {
                Err(problem) => (walk, Err(problem)),
                Ok(terminal) => match declared {
                    None => (walk, Err(crate::words::InstError::UnknownVariable)),
                    Some(kind) => finish_binding(inst, terminal, kind, walk),
                },
            }
        }
        binding => {
            walk.out.push(binding.clone());
            (walk, Ok(()))
        }
    }
}

/// Follow one reference chain to its terminal binding position.
///
/// Each hop is charged before it is taken; the hop count passing the
/// witness length names a cycle without a visited set.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; follow_chain checks the witness-length cycle bound and fuel before checked slot lookup, returning InstError for invalid references; asserting well-founded input would defeat this validator."
)]
fn follow_chain(
    inst: &crate::words::Inst,
    from: crate::words::Variable,
    mut walk: Walk,
    fuel: u32,
) -> (Walk, Result<usize, crate::words::InstError>) {
    let mut hops: usize = 0;
    let mut current = from;
    let mut terminal: Option<usize> = None;
    let mut failure: Option<crate::words::InstError> = None;
    // A missing binding position ends the walk at once: it is an unknown
    // variable, not a cycle, and charges only the hop that found it.
    while terminal.is_none() && failure.is_none() {
        hops += 1;
        if hops > inst.bindings.len() {
            failure = Some(crate::words::InstError::CyclicWitness);
            break;
        }
        if walk.spent >= fuel {
            failure = Some(crate::words::InstError::WalkExhausted);
            break;
        }
        walk.spent += 1;
        // The lookup is two explicit branches: a combinator closure over
        // the borrowed arena is a shape the extraction's borrow pass
        // cannot end.
        let found = match slot_of(current) {
            Some(slot) => inst.bindings.get(slot),
            None => None,
        };
        match found {
            None => failure = Some(crate::words::InstError::UnknownVariable),
            Some(crate::words::Binding::Ref(next)) => current = *next,
            Some(crate::words::Binding::Stack(_)) => terminal = slot_of(current),
            Some(crate::words::Binding::Value(_)) => terminal = slot_of(current),
            Some(crate::words::Binding::Effect(_)) => terminal = slot_of(current),
        }
    }
    match failure {
        Some(problem) => (walk, Err(problem)),
        None => match terminal {
            Some(slot) => (walk, Ok(slot)),
            None => (walk, Err(crate::words::InstError::CyclicWitness)),
        },
    }
}

/// Keep one resolved terminal binding when its kind matches the referring
/// variable's declared kind.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; finish_binding clones the checked terminal binding into the owned Vec output; reassess only if binding copies and output storage become const-capable."
)]
fn finish_binding(
    inst: &crate::words::Inst,
    terminal: usize,
    kind: crate::words::VariableKind,
    mut walk: Walk,
) -> (Walk, Result<(), crate::words::InstError>) {
    let binding = match inst.bindings.get(terminal) {
        Some(binding) => binding,
        None => return (walk, Err(crate::words::InstError::UnknownVariable)),
    };
    if carries_kind(binding, kind) {
        walk.out.push(binding.clone());
        (walk, Ok(()))
    } else {
        (walk, Err(crate::words::InstError::KindMismatch))
    }
}
