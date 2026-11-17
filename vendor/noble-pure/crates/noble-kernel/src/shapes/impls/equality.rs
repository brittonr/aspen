//! Structural comparison, separate from Clone and Debug boundaries.
//!
//! The comparison borrows both patterns: it never copies a subtree, so a deep
//! pattern is compared in one linear pass and a rejected pattern is not
//! duplicated first.

/// Queue one `Program` pair's element checks; the flag fails closed.
fn push_pattern_program<'a>(
    mut work: alloc::vec::Vec<(&'a crate::shapes::Pattern, &'a crate::shapes::Pattern)>,
    first_in: &'a [crate::shapes::Pattern],
    first_out: &'a [crate::shapes::Pattern],
    second_in: &'a [crate::shapes::Pattern],
    second_out: &'a [crate::shapes::Pattern],
) -> (
    alloc::vec::Vec<(&'a crate::shapes::Pattern, &'a crate::shapes::Pattern)>,
    bool,
) {
    let is_comparable = first_in.len() == second_in.len()
        && first_out.len() == second_out.len()
        && work.len() < super::super::WORK_CAP;
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

pub(super) fn pattern_eq(left: &crate::shapes::Pattern, right: &crate::shapes::Pattern) -> bool {
    let mut work: alloc::vec::Vec<(&crate::shapes::Pattern, &crate::shapes::Pattern)> =
        alloc::vec::Vec::with_capacity(8);
    work.push((left, right));
    let mut is_equal = true;
    while !work.is_empty() {
        if work.len() >= super::super::WORK_CAP {
            is_equal = false;
            break;
        }
        if let Some((first, second)) = work.pop() {
            let (next, is_step_equal) = compare(first, second, work);
            work = next;
            is_equal = is_equal && is_step_equal;
        }
    }
    is_equal
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; structural comparison returns false on unequal constructors, lengths or work exhaustion; ordinary unequal inputs must not panic."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; the comparison pushes borrowed node pairs into an owned Vec, whose push and drop are not const on the pinned compiler."
)]
fn compare<'a>(
    first: &'a crate::shapes::Pattern,
    second: &'a crate::shapes::Pattern,
    mut work: alloc::vec::Vec<(&'a crate::shapes::Pattern, &'a crate::shapes::Pattern)>,
) -> (
    alloc::vec::Vec<(&'a crate::shapes::Pattern, &'a crate::shapes::Pattern)>,
    bool,
) {
    let is_equal = match (first, second) {
        (crate::shapes::Pattern::Unit, crate::shapes::Pattern::Unit) => true,
        (crate::shapes::Pattern::Bool, crate::shapes::Pattern::Bool) => true,
        (crate::shapes::Pattern::I64, crate::shapes::Pattern::I64) => true,
        (crate::shapes::Pattern::Text, crate::shapes::Pattern::Text) => true,
        (crate::shapes::Pattern::Syntax, crate::shapes::Pattern::Syntax) => true,
        (crate::shapes::Pattern::Contract, crate::shapes::Pattern::Contract) => true,
        (crate::shapes::Pattern::Evidence, crate::shapes::Pattern::Evidence) => true,
        (crate::shapes::Pattern::Certified, crate::shapes::Pattern::Certified) => true,
        (
            crate::shapes::Pattern::Resource(first_kind),
            crate::shapes::Pattern::Resource(second_kind),
        ) => first_kind == second_kind,
        (
            crate::shapes::Pattern::Nominal(first_id, first_shape),
            crate::shapes::Pattern::Nominal(second_id, second_shape),
        ) => first_id == second_id && first_shape == second_shape,
        (
            crate::shapes::Pattern::GenericNominal(first_id, first_args, first_mapping),
            crate::shapes::Pattern::GenericNominal(second_id, second_args, second_mapping),
        ) if first_id == second_id && first_mapping == second_mapping => {
            work.push((&first_args[0], &second_args[0]));
            work.push((&first_args[1], &second_args[1]));
            true
        }
        (crate::shapes::Pattern::Var(first_var), crate::shapes::Pattern::Var(second_var)) => {
            first_var == second_var
        }
        (
            crate::shapes::Pattern::StackVar(first_var),
            crate::shapes::Pattern::StackVar(second_var),
        ) => first_var == second_var,
        (
            crate::shapes::Pattern::Pair(first_head, first_tail),
            crate::shapes::Pattern::Pair(second_head, second_tail),
        )
        | (
            crate::shapes::Pattern::Sum(first_head, first_tail),
            crate::shapes::Pattern::Sum(second_head, second_tail),
        ) => {
            work.push((first_head, second_head));
            work.push((first_tail, second_tail));
            true
        }
        (crate::shapes::Pattern::List(first_item), crate::shapes::Pattern::List(second_item)) => {
            work.push((first_item, second_item));
            true
        }
        (
            crate::shapes::Pattern::Program(a_in, a_out, a_eff),
            crate::shapes::Pattern::Program(b_in, b_out, b_eff),
        )
        | (
            crate::shapes::Pattern::LiveRef(a_in, a_out, a_eff),
            crate::shapes::Pattern::LiveRef(b_in, b_out, b_eff),
        ) => {
            let (next, is_program_equal) = push_pattern_program(work, a_in, a_out, b_in, b_out);
            work = next;
            is_program_equal && a_eff == b_eff
        }
        _ => false,
    };
    (work, is_equal)
}
