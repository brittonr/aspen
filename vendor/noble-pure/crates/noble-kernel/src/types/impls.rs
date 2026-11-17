//! Hand-written structural impls for the self-recursive type tree.
//!
//! `Ty` recurses through `alloc` containers, so a derived `Clone` or `Debug`
//! body hands this type's own trait instance to `Vec::clone` and to the
//! formatting machinery. Aeneas emits a trait instance after the method that
//! feeds it, so such a body forward-references a declaration Lean has not seen
//! yet. These bodies call the concrete methods instead: an element clone goes
//! through the local `Clone` impl and nested formatting goes through the local
//! `Debug` impl, so the translated module stays acyclic.
/// Deep-copy one type stack, one element at a time.
///
/// The extraction treats this helper as an assumption: Aeneas would
/// functionalize its loop inside the clone instance.s mutual block, and the
/// Lean backend cannot prove that shape monotone. Cloning stays an explicit,
/// disclosed copy step.
#[charon::opaque]
fn clone_stack(stack: &[crate::types::Ty]) -> alloc::vec::Vec<crate::types::Ty> {
    let mut out: alloc::vec::Vec<crate::types::Ty> = alloc::vec::Vec::with_capacity(stack.len());
    let mut index = 0;
    while index < stack.len() {
        out.push(stack[index].clone());
        index += 1;
    }
    out
}

/// Deep-copy one nominal shape by cloning only its `Ty` payloads.
///
/// `Ty`'s own `Clone` must not go through `NominalShape`'s derived `Clone`: the two
/// impls call each other, and Aeneas rejects mutually recursive definitions (and its
/// Lean backend additionally refuses termination clauses for that shape). Cloning the
/// payloads here keeps the call graph one-directional: `NominalShape::clone` may call
/// `Ty::clone`, never the reverse.
fn clone_shape(shape: &crate::types::NominalShape) -> crate::types::NominalShape {
    match shape {
        crate::types::NominalShape::Opaque(representation) => {
            crate::types::NominalShape::Opaque(alloc::boxed::Box::new((**representation).clone()))
        }
        crate::types::NominalShape::Variant(left, right) => crate::types::NominalShape::Variant(
            alloc::boxed::Box::new((**left).clone()),
            alloc::boxed::Box::new((**right).clone()),
        ),
    }
}

/// Deep-copy one type.
impl Clone for crate::types::Ty {
    fn clone(&self) -> crate::types::Ty {
        match self {
            crate::types::Ty::Unit => crate::types::Ty::Unit,
            crate::types::Ty::Bool => crate::types::Ty::Bool,
            crate::types::Ty::I64 => crate::types::Ty::I64,
            crate::types::Ty::Text => crate::types::Ty::Text,
            crate::types::Ty::Syntax => crate::types::Ty::Syntax,
            crate::types::Ty::Contract => crate::types::Ty::Contract,
            crate::types::Ty::Evidence => crate::types::Ty::Evidence,
            crate::types::Ty::Certified => crate::types::Ty::Certified,
            crate::types::Ty::Pair(left, right) => crate::types::Ty::Pair(
                alloc::boxed::Box::new((**left).clone()),
                alloc::boxed::Box::new((**right).clone()),
            ),
            crate::types::Ty::Sum(left, right) => crate::types::Ty::Sum(
                alloc::boxed::Box::new((**left).clone()),
                alloc::boxed::Box::new((**right).clone()),
            ),
            crate::types::Ty::List(item) => {
                crate::types::Ty::List(alloc::boxed::Box::new((**item).clone()))
            }
            crate::types::Ty::Program(stack_in, stack_out, effects) => crate::types::Ty::Program(
                alloc::boxed::Box::new(clone_stack(stack_in.as_slice())),
                alloc::boxed::Box::new(clone_stack(stack_out.as_slice())),
                effects.clone(),
            ),
            crate::types::Ty::LiveRef(stack_in, stack_out, effects) => crate::types::Ty::LiveRef(
                alloc::boxed::Box::new(clone_stack(stack_in.as_slice())),
                alloc::boxed::Box::new(clone_stack(stack_out.as_slice())),
                effects.clone(),
            ),
            crate::types::Ty::Resource(kind) => crate::types::Ty::Resource(*kind),
            crate::types::Ty::Nominal(id, shape) => {
                crate::types::Ty::Nominal(*id, alloc::boxed::Box::new(clone_shape(shape)))
            }
            crate::types::Ty::GenericNominal(id, args, shape) => crate::types::Ty::GenericNominal(
                *id,
                alloc::boxed::Box::new([args[0].clone(), args[1].clone()]),
                alloc::boxed::Box::new(clone_shape(shape)),
            ),
        }
    }
}

/// Queue one `Program` pair's element checks; the flag fails closed.
fn push_type_program<'a>(
    mut work: alloc::vec::Vec<(&'a crate::types::Ty, &'a crate::types::Ty)>,
    first_in: &'a [crate::types::Ty],
    first_out: &'a [crate::types::Ty],
    second_in: &'a [crate::types::Ty],
    second_out: &'a [crate::types::Ty],
) -> (
    alloc::vec::Vec<(&'a crate::types::Ty, &'a crate::types::Ty)>,
    bool,
) {
    let is_comparable = first_in.len() == second_in.len()
        && first_out.len() == second_out.len()
        && work.len() < super::WORK_CAP;
    let mut index = 0;
    while index < first_in.len() && is_comparable {
        work.push((&first_in[index], &second_in[index]));
        index += 1;
    }
    index = 0;
    while index < first_out.len() && is_comparable {
        work.push((&first_out[index], &second_out[index]));
        index += 1;
    }
    (work, is_comparable)
}

/// Structural equality of two types, decided by one explicit pairwise walk.
///
/// The work stack holds borrowed node pairs and visits each compared node
/// once: equality never copies a subtree, so comparing two deep values costs
/// the nodes they share, not the square of their depth. The walk fails closed
/// past the local work bound.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; ty_eq returns false for constructor, stack-length, effect or local-work mismatches; unequal types and exhausted comparisons must remain boolean results rather than panic paths."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-kernel; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
fn ty_eq(left: &crate::types::Ty, right: &crate::types::Ty) -> bool {
    let mut work: alloc::vec::Vec<(&crate::types::Ty, &crate::types::Ty)> =
        alloc::vec::Vec::with_capacity(8);
    work.push((left, right));
    while !work.is_empty() {
        if work.len() >= super::WORK_CAP {
            return false;
        }
        let Some((first, second)) = work.pop() else {
            break;
        };
        match (first, second) {
            (crate::types::Ty::Unit, crate::types::Ty::Unit)
            | (crate::types::Ty::Bool, crate::types::Ty::Bool)
            | (crate::types::Ty::I64, crate::types::Ty::I64)
            | (crate::types::Ty::Text, crate::types::Ty::Text)
            | (crate::types::Ty::Syntax, crate::types::Ty::Syntax)
            | (crate::types::Ty::Contract, crate::types::Ty::Contract)
            | (crate::types::Ty::Evidence, crate::types::Ty::Evidence)
            | (crate::types::Ty::Certified, crate::types::Ty::Certified) => {}
            (crate::types::Ty::Resource(first_kind), crate::types::Ty::Resource(second_kind))
                if first_kind == second_kind => {}
            (
                crate::types::Ty::Nominal(first_id, first_shape),
                crate::types::Ty::Nominal(second_id, second_shape),
            ) if first_id == second_id => match (&**first_shape, &**second_shape) {
                (
                    crate::types::NominalShape::Opaque(first),
                    crate::types::NominalShape::Opaque(second),
                ) => work.push((first, second)),
                (
                    crate::types::NominalShape::Variant(first_left, first_right),
                    crate::types::NominalShape::Variant(second_left, second_right),
                ) => {
                    work.push((first_right, second_right));
                    work.push((first_left, second_left));
                }
                _ => return false,
            },
            (
                crate::types::Ty::GenericNominal(first_id, first_args, first_shape),
                crate::types::Ty::GenericNominal(second_id, second_args, second_shape),
            ) if first_id == second_id => {
                work.push((&first_args[0], &second_args[0]));
                work.push((&first_args[1], &second_args[1]));
                match (&**first_shape, &**second_shape) {
                    (
                        crate::types::NominalShape::Variant(first_left, first_right),
                        crate::types::NominalShape::Variant(second_left, second_right),
                    ) => {
                        work.push((first_right, second_right));
                        work.push((first_left, second_left));
                    }
                    _ => return false,
                }
            }
            (
                crate::types::Ty::Pair(first_head, first_tail),
                crate::types::Ty::Pair(second_head, second_tail),
            )
            | (
                crate::types::Ty::Sum(first_head, first_tail),
                crate::types::Ty::Sum(second_head, second_tail),
            ) => {
                work.push((first_head, second_head));
                work.push((first_tail, second_tail));
            }
            (crate::types::Ty::List(first_item), crate::types::Ty::List(second_item)) => {
                work.push((first_item, second_item));
            }
            (
                crate::types::Ty::Program(a_in, a_out, a_eff),
                crate::types::Ty::Program(b_in, b_out, b_eff),
            )
            | (
                crate::types::Ty::LiveRef(a_in, a_out, a_eff),
                crate::types::Ty::LiveRef(b_in, b_out, b_eff),
            ) if a_eff == b_eff => {
                let (next, is_program_equal) = push_type_program(work, a_in, a_out, b_in, b_out);
                work = next;
                if !is_program_equal {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

impl PartialEq for crate::types::Ty {
    fn eq(&self, other: &crate::types::Ty) -> bool {
        ty_eq(self, other)
    }
}

/// Render one type stack in the list form `[a, b]`.
///
/// The extraction treats this helper as an assumption: formatting is
/// observability only, and Aeneas would functionalize its loop inside the
/// debug instance's mutual block, which the Lean backend cannot prove
/// monotone.
#[charon::opaque]
#[allow(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers. Formatting writes only the explicit caller-owned Formatter required by core::fmt; text layout is outside the semantic refinement boundary."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; debug_stack uses length-bounded indexing and propagates the caller's first fmt::Error; arbitrary type stacks and formatter failures must not trigger assertions."
)]
fn debug_stack(stack: &[crate::types::Ty], f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    attempt!(core::fmt::Formatter::write_str(f, "["));
    let mut index = 0;
    let mut failure: Option<core::fmt::Error> = None;
    while index < stack.len() {
        let step = if index == 0 {
            core::fmt::Debug::fmt(&stack[index], f)
        } else {
            match core::fmt::Formatter::write_str(f, ", ") {
                Ok(()) => core::fmt::Debug::fmt(&stack[index], f),
                Err(problem) => Err(problem),
            }
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
        None => core::fmt::Formatter::write_str(f, "]"),
    }
}

/// Render one nominal shape, closing it, so `Ty`'s `Debug` never calls
/// `NominalShape`'s derived `Debug` (mutually recursive pair, see `clone_shape`).
#[allow(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers. Formatting writes only the explicit caller-owned Formatter required by core::fmt; text layout is outside the semantic refinement boundary."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; debug_shape calls non-const core::fmt::Formatter::write_str and Debug::fmt, so a const helper cannot express this formatting boundary."
)]
fn debug_shape(
    shape: &crate::types::NominalShape,
    f: &mut core::fmt::Formatter<'_>,
) -> core::fmt::Result {
    match shape {
        crate::types::NominalShape::Opaque(representation) => {
            attempt!(core::fmt::Formatter::write_str(f, "Opaque("));
            attempt!(core::fmt::Debug::fmt(&**representation, f));
        }
        crate::types::NominalShape::Variant(left, right) => {
            attempt!(core::fmt::Formatter::write_str(f, "Variant("));
            attempt!(core::fmt::Debug::fmt(&**left, f));
            attempt!(core::fmt::Formatter::write_str(f, ", "));
            attempt!(core::fmt::Debug::fmt(&**right, f));
        }
    }
    core::fmt::Formatter::write_str(f, ")")
}

/// Render one type in its constructor form.
impl core::fmt::Debug for crate::types::Ty {
    #[allow(
        tigerstyle::mutating_input_in_pure,
        reason = "Owner: noble-maintainers. The standard Debug trait requires a mutable caller-owned Formatter; this performs no ambient observation or semantic state mutation."
    )]
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; this implements the non-const Debug::fmt trait method and calls runtime Formatter writes; reassess only if core::fmt gains a stable const trait contract."
    )]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            crate::types::Ty::Unit => core::fmt::Formatter::write_str(f, "Unit"),
            crate::types::Ty::Bool => core::fmt::Formatter::write_str(f, "Bool"),
            crate::types::Ty::I64 => core::fmt::Formatter::write_str(f, "I64"),
            crate::types::Ty::Text => core::fmt::Formatter::write_str(f, "Text"),
            crate::types::Ty::Syntax => core::fmt::Formatter::write_str(f, "Syntax"),
            crate::types::Ty::Contract => core::fmt::Formatter::write_str(f, "Contract"),
            crate::types::Ty::Evidence => core::fmt::Formatter::write_str(f, "Evidence"),
            crate::types::Ty::Certified => core::fmt::Formatter::write_str(f, "Certified"),
            crate::types::Ty::Pair(left, right) => {
                attempt!(core::fmt::Formatter::write_str(f, "Pair("));
                attempt!(core::fmt::Debug::fmt(&**left, f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(core::fmt::Debug::fmt(&**right, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::Sum(left, right) => {
                attempt!(core::fmt::Formatter::write_str(f, "Sum("));
                attempt!(core::fmt::Debug::fmt(&**left, f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(core::fmt::Debug::fmt(&**right, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::List(item) => {
                attempt!(core::fmt::Formatter::write_str(f, "List("));
                attempt!(core::fmt::Debug::fmt(&**item, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::Program(stack_in, stack_out, effects) => {
                attempt!(core::fmt::Formatter::write_str(f, "Program("));
                attempt!(debug_stack(stack_in.as_slice(), f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(debug_stack(stack_out.as_slice(), f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(core::fmt::Debug::fmt(effects, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::LiveRef(stack_in, stack_out, effects) => {
                attempt!(core::fmt::Formatter::write_str(f, "LiveRef("));
                attempt!(debug_stack(stack_in.as_slice(), f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(debug_stack(stack_out.as_slice(), f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(core::fmt::Debug::fmt(effects, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::Resource(kind) => {
                attempt!(core::fmt::Formatter::write_str(f, "Resource("));
                attempt!(core::fmt::Debug::fmt(kind, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::Nominal(id, shape) => {
                attempt!(core::fmt::Formatter::write_str(f, "Nominal("));
                attempt!(core::fmt::Debug::fmt(id, f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(debug_shape(shape, f));
                core::fmt::Formatter::write_str(f, ")")
            }
            crate::types::Ty::GenericNominal(id, args, shape) => {
                attempt!(core::fmt::Formatter::write_str(f, "GenericNominal("));
                attempt!(core::fmt::Debug::fmt(id, f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(debug_stack(args.as_slice(), f));
                attempt!(core::fmt::Formatter::write_str(f, ", "));
                attempt!(debug_shape(shape, f));
                core::fmt::Formatter::write_str(f, ")")
            }
        }
    }
}

#[cfg(test)]
impl Eq for crate::types::Ty {}
