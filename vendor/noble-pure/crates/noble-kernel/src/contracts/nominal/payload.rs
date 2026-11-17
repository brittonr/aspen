//! Bounded construction and checking of nominal payload patterns.

/// One pending type visit or its postorder constructor.
struct Frame<'a> {
    ty: &'a crate::types::Ty,
    is_complete: bool,
}

struct Build<'a> {
    work: alloc::vec::Vec<Frame<'a>>,
    built: alloc::vec::Vec<crate::shapes::Pattern>,
    entered: usize,
}

impl<'a> Build<'a> {
    fn new(ty: &'a crate::types::Ty) -> Self {
        let mut work = alloc::vec::Vec::with_capacity(1);
        let built = alloc::vec::Vec::with_capacity(8);
        work.push(Frame {
            ty,
            is_complete: false,
        });
        Self {
            work,
            built,
            entered: 0,
        }
    }
}

/// Construct a payload pattern, including immutable programs whose
/// interfaces may mention resources without owning those resources.
pub(super) fn pattern(
    env: &super::super::Env,
    ty: &crate::types::Ty,
) -> Option<crate::shapes::Pattern> {
    let mut state = Some(Build::new(ty));
    let mut result = None;
    while let Some(mut current) = state {
        state = match current.work.pop() {
            Some(frame) if current.work.len() < 512 => advance(env, frame, current),
            Some(_) => None,
            None => {
                if current.built.len() == 1 {
                    result = current.built.pop();
                }
                None
            }
        };
    }
    result
}

fn advance<'a>(
    env: &super::super::Env,
    frame: Frame<'a>,
    mut state: Build<'a>,
) -> Option<Build<'a>> {
    if frame.is_complete {
        return complete(env, frame.ty, state);
    }
    if state.entered >= 512 {
        return None;
    }
    state.entered += 1;
    enter(frame.ty, state)
}

#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-kernel; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::nested_conditionals,
    reason = "one-pass dispatcher over the type constructors; every arm returns, so the chain is flat in control flow"
)]
fn enter<'a>(ty: &'a crate::types::Ty, mut state: Build<'a>) -> Option<Build<'a>> {
    debug_assert!(state.entered > 0 && state.entered <= 512);
    debug_assert!(state.built.len() <= state.entered);
    if let crate::types::Ty::Pair(left, right) | crate::types::Ty::Sum(left, right) = ty {
        state.work.push(Frame {
            ty,
            is_complete: true,
        });
        state.work.push(Frame {
            ty: right,
            is_complete: false,
        });
        state.work.push(Frame {
            ty: left,
            is_complete: false,
        });
        return Some(state);
    } else if let crate::types::Ty::List(item) = ty {
        state.work.push(Frame {
            ty,
            is_complete: true,
        });
        state.work.push(Frame {
            ty: item,
            is_complete: false,
        });
        return Some(state);
    } else if let crate::types::Ty::Program(input, output, _) = ty {
        state.work.push(Frame {
            ty,
            is_complete: true,
        });
        let mut index = output.len();
        while index > 0 {
            index -= 1;
            state.work.push(Frame {
                ty: &output[index],
                is_complete: false,
            });
        }
        index = input.len();
        while index > 0 {
            index -= 1;
            state.work.push(Frame {
                ty: &input[index],
                is_complete: false,
            });
        }
        return Some(state);
    } else if let crate::types::Ty::GenericNominal(_, args, _) = ty {
        state.work.push(Frame {
            ty,
            is_complete: true,
        });
        state.work.push(Frame {
            ty: &args[1],
            is_complete: false,
        });
        state.work.push(Frame {
            ty: &args[0],
            is_complete: false,
        });
        return Some(state);
    }
    match leaf_pattern(ty) {
        Some(pattern) => {
            state.built.push(pattern);
            Some(state)
        }
        None => None,
    }
}

fn leaf_pattern(ty: &crate::types::Ty) -> Option<crate::shapes::Pattern> {
    match ty {
        crate::types::Ty::Unit => Some(crate::shapes::Pattern::Unit),
        crate::types::Ty::Bool => Some(crate::shapes::Pattern::Bool),
        crate::types::Ty::I64 => Some(crate::shapes::Pattern::I64),
        crate::types::Ty::Text => Some(crate::shapes::Pattern::Text),
        crate::types::Ty::Syntax => Some(crate::shapes::Pattern::Syntax),
        crate::types::Ty::Contract => Some(crate::shapes::Pattern::Contract),
        crate::types::Ty::Evidence => Some(crate::shapes::Pattern::Evidence),
        crate::types::Ty::Certified => Some(crate::shapes::Pattern::Certified),
        crate::types::Ty::Resource(kind) => Some(crate::shapes::Pattern::Resource(*kind)),
        crate::types::Ty::Nominal(id, shape) => Some(crate::shapes::Pattern::Nominal(
            *id,
            alloc::boxed::Box::new((**shape).clone()),
        )),
        _ => None,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; literal const complete fails on non-const complete_pair/complete_list (E0015) and owned Build destruction (E0493) under both pinned compilers; reassess when those operations become const-capable."
)]
fn complete<'a>(
    env: &super::super::Env,
    ty: &crate::types::Ty,
    state: Build<'a>,
) -> Option<Build<'a>> {
    match ty {
        crate::types::Ty::Pair(_, _) => complete_pair(false, state),
        crate::types::Ty::Sum(_, _) => complete_pair(true, state),
        crate::types::Ty::List(_) => complete_list(state),
        crate::types::Ty::Program(input, output, effects) => {
            complete_program(input.len(), output.len(), effects, state)
        }
        crate::types::Ty::GenericNominal(id, _, _) => complete_generic(env, *id, state),
        _ => None,
    }
}

fn complete_generic<'a>(
    env: &super::super::Env,
    id: crate::types::NominalTypeId,
    mut state: Build<'a>,
) -> Option<Build<'a>> {
    let second = state.built.pop()?;
    let first = state.built.pop()?;
    let decl = env.generic_variant(id)?;
    state.built.push(crate::shapes::Pattern::GenericNominal(
        id,
        alloc::boxed::Box::new([first, second]),
        decl.payload_params,
    ));
    Some(state)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "the two lengths are positional by contract and named at every call site"
)]
fn complete_program<'a>(
    input_len: usize,
    output_len: usize,
    effects: &crate::types::EffSet,
    mut state: Build<'a>,
) -> Option<Build<'a>> {
    let mut output = alloc::vec::Vec::with_capacity(output_len);
    let mut index = 0;
    while index < output_len {
        match state.built.pop() {
            Some(part) => {
                output.push(part);
                index += 1;
            }
            None => break,
        }
    }
    if index != output_len {
        return None;
    }
    output.reverse();
    let mut input = alloc::vec::Vec::with_capacity(input_len);
    index = 0;
    while index < input_len {
        match state.built.pop() {
            Some(part) => {
                input.push(part);
                index += 1;
            }
            None => break,
        }
    }
    if index != input_len {
        return None;
    }
    input.reverse();
    let mut slots = alloc::vec::Vec::with_capacity(effects.as_slice().len());
    index = 0;
    while index < effects.as_slice().len() {
        slots.push(crate::shapes::EffectSlot::Effect(effects.as_slice()[index]));
        index += 1;
    }
    state
        .built
        .push(crate::shapes::Pattern::program(input, output, slots));
    Some(state)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject Vec<Pattern>::pop (E0015) and push (E0658) required by paired postorder construction; reassess when these Vec operations become const-capable."
)]
fn complete_pair<'a>(is_sum: bool, mut state: Build<'a>) -> Option<Build<'a>> {
    match state.built.pop() {
        Some(right) => match state.built.pop() {
            Some(left) => {
                state.built.push(pair_pattern(left, right, is_sum));
                Some(state)
            }
            None => None,
        },
        None => None,
    }
}

fn complete_list<'a>(mut state: Build<'a>) -> Option<Build<'a>> {
    match state.built.pop() {
        Some(item) => {
            state
                .built
                .push(crate::shapes::Pattern::List(alloc::boxed::Box::new(item)));
            Some(state)
        }
        None => None,
    }
}

fn pair_pattern(
    left: crate::shapes::Pattern,
    right: crate::shapes::Pattern,
    is_sum: bool,
) -> crate::shapes::Pattern {
    if is_sum {
        crate::shapes::Pattern::Sum(alloc::boxed::Box::new(left), alloc::boxed::Box::new(right))
    } else {
        crate::shapes::Pattern::Pair(alloc::boxed::Box::new(left), alloc::boxed::Box::new(right))
    }
}
