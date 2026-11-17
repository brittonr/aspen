//! Pure helpers for the acceptance machine: charges, limits, joins, and
//! diagnostics. No helper recurses or mutates its inputs.

pub(crate) mod effects;
pub(crate) mod instantiate;
mod multiset;

/// The request and environment a check runs against.
pub(crate) struct Ctx<'a> {
    pub(crate) request: &'a crate::untrusted::Request,
    pub(crate) env: &'a crate::contracts::Env,
    /// Structural checking only; the source-bound Wasm entry separately
    /// authenticates the owner and every lexical occurrence.
    pub(crate) signed_owner: Option<crate::contracts::Definition>,
}

/// The failing site: a node and, for invocations, its definition.
#[derive(Clone, Copy)]
pub(crate) struct Site {
    pub(crate) node: Option<crate::untrusted::NodeId>,
    pub(crate) def: Option<crate::contracts::Definition>,
}

pub(crate) const fn site(
    node: Option<crate::untrusted::NodeId>,
    def: Option<crate::contracts::Definition>,
) -> Site {
    Site { node, def }
}

/// Charge work, failing closed before the declared limit is exceeded.
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; work and cost intentionally use the same work-unit domain, with checked subtraction defining remaining budget and Work exhaustion; wrappers would only restate this scalar contract."
)]
pub(crate) const fn charge(work: u32, cost: u32) -> Result<u32, super::Fail> {
    match work.checked_sub(cost) {
        Some(remaining) => Ok(remaining),
        None => Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    }
}

pub(crate) fn scheme_cost(scheme: &crate::words::Scheme) -> Result<u32, super::Fail> {
    let count = match u32::try_from(scheme.stack_in.len().saturating_add(scheme.stack_out.len())) {
        Ok(count) => count,
        Err(_) => return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    };
    Ok(count.saturating_add(1))
}

pub(crate) fn join_cost(interface: &crate::untrusted::Interface) -> Result<u32, super::Fail> {
    let entries = interface
        .stack_in
        .len()
        .saturating_add(interface.stack_out.len());
    let count = match u32::try_from(entries) {
        Ok(count) => count,
        Err(_) => return Err(super::Fail::Exhausted(crate::untrusted::LimitKind::Work)),
    };
    Ok(count.saturating_add(1))
}

/// Validate one stack against the declared height and type-size limits.
pub(crate) fn limits_of(stack: &[crate::types::Ty], ctx: &Ctx) -> Result<(), super::Fail> {
    limits_of_with(stack, ctx, false)
}

/// Entry input is the only stack permitted to carry borrowed references,
/// which are subsequently moved into the frame's ordered sidecar.
pub(crate) fn limits_of_entry(stack: &[crate::types::Ty], ctx: &Ctx) -> Result<(), super::Fail> {
    limits_of_with(stack, ctx, true)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; limits_of_with rejects unrepresentable or excessive stack height before checking each type size, preserving typed StackHeight/TypeSize exhaustion instead of assertions."
)]
fn limits_of_with(
    stack: &[crate::types::Ty],
    ctx: &Ctx,
    allow_borrowed: bool,
) -> Result<(), super::Fail> {
    let height = match u64::try_from(stack.len()) {
        Ok(height) => height,
        Err(_) => {
            return Err(super::Fail::Exhausted(
                crate::untrusted::LimitKind::StackHeight,
            ))
        }
    };
    if height > u64::from(ctx.request.limits.stack_height) {
        return Err(super::Fail::Exhausted(
            crate::untrusted::LimitKind::StackHeight,
        ));
    }
    let mut is_valid = true;
    let mut index = 0;
    while index < stack.len() {
        let is_next =
            if allow_borrowed && matches!(&stack[index], crate::types::Ty::LiveRef(_, _, _)) {
                ctx.env.valid_live_ref(&stack[index])
            } else {
                ctx.env.valid_type(&stack[index], 512)
            };
        if !is_next {
            is_valid = false;
            break;
        }
        index += 1;
    }
    if !is_valid {
        return Err(invalid_without_stacks(
            site(None, None),
            crate::untrusted::Constraint::InvalidType,
        ));
    }
    match crate::words::bounds::check_sizes(stack, ctx.request.limits.type_size) {
        Ok(()) => Ok(()),
        Err(_) => Err(super::Fail::Exhausted(
            crate::untrusted::LimitKind::TypeSize,
        )),
    }
}

/// Check each stack entry without carrying a borrowed entry across the loop.
pub(crate) fn valid_stack_types(stack: &[crate::types::Ty], env: &crate::contracts::Env) -> bool {
    let mut index = 0;
    let mut is_every_type_valid = true;
    while index < stack.len() {
        if !valid_stack_type_at(stack, index, env) {
            is_every_type_valid = false;
            break;
        }
        index += 1;
    }
    is_every_type_valid
}

fn valid_stack_type_at(
    stack: &[crate::types::Ty],
    index: usize,
    env: &crate::contracts::Env,
) -> bool {
    env.valid_type(&stack[index], 512)
}

/// The failing definition of a node, when it is an invocation.
pub(crate) fn definition_of(
    candidate: &crate::untrusted::Candidate,
    node: crate::untrusted::NodeId,
) -> Option<crate::contracts::Definition> {
    let index = match usize::try_from(node.0) {
        Ok(index) => index,
        Err(_) => return None,
    };
    match candidate.nodes.get(index) {
        Some(
            crate::untrusted::Node::Invocation { def, .. }
            | crate::untrusted::Node::SignedSelf { def, .. },
        ) => Some(*def),
        Some(crate::untrusted::Node::Literal { .. })
        | Some(crate::untrusted::Node::SlotInvoke { .. })
        | Some(crate::untrusted::Node::Quotation { .. })
        | None => None,
    }
}

/// Join one interface into a frame's running stack and effect bound.
pub(crate) fn join(
    frame: super::Frame,
    interface: &crate::untrusted::Interface,
    at: Site,
    ctx: &Ctx,
) -> Result<super::Frame, super::Fail> {
    let mut joined = frame;
    if let Some(constraint) = match_tail(&joined.stack, &interface.stack_in) {
        return Err(invalid(
            ctx,
            at,
            interface.stack_in.clone(),
            tail_copy(&joined.stack, interface.stack_in.len()),
            constraint,
        ));
    }
    let keep = joined.stack.len().saturating_sub(interface.stack_in.len());
    joined.stack.truncate(keep);
    joined.stack.extend_from_slice(&interface.stack_out);
    joined.effects = joined.effects.union(&interface.effects);
    attempt!(limits_of_join(
        joined.stack.len(),
        &interface.stack_out,
        ctx
    ));
    // A join replaces exactly the matched suffix and appends exactly the
    // interface's output, and the height it validated is the height it
    // returns.
    debug_assert_eq!(
        joined.stack.len(),
        keep.saturating_add(interface.stack_out.len()),
        "a join appends exactly the interface output"
    );
    debug_assert!(
        u64::try_from(joined.stack.len())
            .is_ok_and(|height| height <= u64::from(ctx.request.limits.stack_height)),
        "a joined frame stays inside the declared stack height"
    );
    Ok(joined)
}

/// Validate the entries one join appends, plus the height it produces.
///
/// A retained entry entered the frame through the join that appended it (or
/// through `limits_of_entry`), so it was already checked against the same
/// height, type-size and environment predicates. Re-walking the retained
/// prefix on every node would make one join cost the whole frame instead of
/// the interface the join charges.
pub(crate) fn limits_of_join(
    height: usize,
    appended: &[crate::types::Ty],
    ctx: &Ctx,
) -> Result<(), super::Fail> {
    let joined_height = match u64::try_from(height) {
        Ok(joined_height) => joined_height,
        Err(_) => {
            return Err(super::Fail::Exhausted(
                crate::untrusted::LimitKind::StackHeight,
            ))
        }
    };
    if joined_height > u64::from(ctx.request.limits.stack_height) {
        return Err(super::Fail::Exhausted(
            crate::untrusted::LimitKind::StackHeight,
        ));
    }
    limits_of(appended, ctx)
}

/// Whether the stack's top matches the expected segment.
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; the length guard proves stack.len() >= expected.len(), and index < expected.len() proves tail_start + index < stack.len(); reassess if either guard changes."
)]
fn match_tail(
    stack: &[crate::types::Ty],
    expected: &[crate::types::Ty],
) -> Option<crate::untrusted::Constraint> {
    if stack.len() < expected.len() {
        return Some(crate::untrusted::Constraint::StackJoin);
    }
    let tail_start = stack.len() - expected.len();
    let mut mismatch: Option<crate::untrusted::Constraint> = None;
    let mut index = 0;
    while index < expected.len() {
        let ty = &expected[index];
        if &stack[tail_start + index] != ty {
            let actual = tail_copy(stack, expected.len());
            mismatch = Some(mismatch_constraint(expected, &actual));
            break;
        }
        index += 1;
    }
    mismatch
}

/// Classify a mismatch as wrong order or wrong shape.
pub(crate) fn mismatch_constraint(
    expected: &[crate::types::Ty],
    actual: &[crate::types::Ty],
) -> crate::untrusted::Constraint {
    if expected.len() == actual.len() && multiset::same(expected, actual) {
        crate::untrusted::Constraint::StackOrder
    } else {
        crate::untrusted::Constraint::StackJoin
    }
}

#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; tail_copy subtracts needed only in the branch proving stack.len() >= needed; the other branch copies the entire shorter stack."
)]
pub(crate) fn tail_copy(
    stack: &[crate::types::Ty],
    needed: usize,
) -> alloc::vec::Vec<crate::types::Ty> {
    if stack.len() >= needed {
        stack[stack.len() - needed..].to_vec()
    } else {
        stack.to_vec()
    }
}

/// Report a failure with no stack comparison, without charging or truncating
/// a diagnostic budget: both stack segments are empty by construction.
pub(crate) const fn invalid_without_stacks(
    at: Site,
    constraint: crate::untrusted::Constraint,
) -> super::Fail {
    super::Fail::Invalid(crate::untrusted::Diagnostic {
        node: at.node,
        def: at.def,
        expected: alloc::vec::Vec::new(),
        actual: alloc::vec::Vec::new(),
        constraint,
        provenance_available: false,
        truncated: false,
    })
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; invalid truncates expected then actual entries to the caller's diagnostic budget, including zero, and returns that diagnostic; assertions would turn reporting into another failure path."
)]
pub(crate) fn invalid(
    ctx: &Ctx,
    at: Site,
    mut expected: alloc::vec::Vec<crate::types::Ty>,
    mut actual: alloc::vec::Vec<crate::types::Ty>,
    constraint: crate::untrusted::Constraint,
) -> super::Fail {
    let budget = usize::try_from(ctx.request.limits.diagnostics).unwrap_or(0);
    let mut is_truncated = false;
    if expected.len().saturating_add(actual.len()) > budget {
        is_truncated = true;
        let keep_expected = budget.saturating_sub(actual.len());
        expected.truncate(keep_expected);
        if expected.len().saturating_add(actual.len()) > budget {
            let keep_actual = budget.saturating_sub(expected.len());
            actual.truncate(keep_actual);
        }
    }
    super::Fail::Invalid(crate::untrusted::Diagnostic {
        node: at.node,
        def: at.def,
        expected,
        actual,
        constraint,
        provenance_available: false,
        truncated: is_truncated,
    })
}
