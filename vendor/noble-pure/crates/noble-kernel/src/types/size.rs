//! The declared type-size walk: an explicit work stack with a local bound.
//!
//! The walk is spelled as a state machine (`Walk` threaded through
//! `walk_step`) so no loop body returns early.
//!
//! The work stack holds borrowed nodes. A walk that owned its queue had to
//! deep-copy every subtree to reach its children, which made one size read
//! quadratic in the tree depth and let a candidate inside its declared limits
//! spend unbounded work. Nothing here clones a node.

/// One size walk's threaded state.
pub(super) struct Walk<'a> {
    /// Nodes still to visit, with their expansion markers.
    pub(super) todo: alloc::vec::Vec<(&'a crate::types::Ty, bool)>,
    /// Sizes already computed, in visit order.
    pub(super) sizes: alloc::vec::Vec<u32>,
}

/// One size-walk step's outcome.
pub(super) enum Step {
    /// More nodes remain.
    Continue,
    /// Every node was visited.
    Done,
    /// A local bound or arithmetic guard failed the walk closed.
    Failed,
}

/// Run one size-walk step, returning the updated state and its outcome.
pub(super) fn walk_step(mut walk: Walk<'_>) -> (Walk<'_>, Step) {
    match walk.todo.pop() {
        None => (walk, Step::Done),
        Some((node, expanded)) => {
            if walk.todo.len() >= super::WORK_CAP {
                return (walk, Step::Failed);
            }
            if expanded {
                let (next_sizes, total) = take_sizes(walk.sizes, count_children(node));
                walk.sizes = next_sizes;
                match total {
                    Some(total) => {
                        walk.sizes.push(total);
                        (walk, Step::Continue)
                    }
                    None => (walk, Step::Failed),
                }
            } else {
                let (todo, sizes) = queue_children(node, walk.todo, walk.sizes);
                walk.todo = todo;
                walk.sizes = sizes;
                (walk, Step::Continue)
            }
        }
    }
}

/// Number of recorded child sizes an expanded node consumes.
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; each nonzero-sized Ty vector length is at most isize::MAX, so their sum fits usize; reassess if Program storage or the Ty representation changes."
)]
const fn count_children(node: &crate::types::Ty) -> usize {
    match node {
        crate::types::Ty::Pair(_, _) | crate::types::Ty::Sum(_, _) => 2,
        crate::types::Ty::Nominal(_, shape) => match &**shape {
            crate::types::NominalShape::Opaque(_) => 1,
            crate::types::NominalShape::Variant(_, _) => 2,
        },
        crate::types::Ty::GenericNominal(_, _, shape) => match &**shape {
            crate::types::NominalShape::Variant(_, _) => 2,
            crate::types::NominalShape::Opaque(_) => 1,
        },
        crate::types::Ty::List(_) => 1,
        crate::types::Ty::Program(stack_in, stack_out, _)
        | crate::types::Ty::LiveRef(stack_in, stack_out, _) => {
            (**stack_in).len() + (**stack_out).len()
        }
        crate::types::Ty::Unit
        | crate::types::Ty::Bool
        | crate::types::Ty::I64
        | crate::types::Ty::Text
        | crate::types::Ty::Syntax
        | crate::types::Ty::Contract
        | crate::types::Ty::Evidence
        | crate::types::Ty::Certified
        | crate::types::Ty::Resource(_) => 0,
    }
}

/// Add `count` recorded child sizes to the node's own unit, saturating; the
/// total is `None` when fewer sizes were recorded. The size stack returns so
/// the caller keeps its state whichever way the step went.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; take_sizes preserves the declared saturating size measure and returns None for missing child sizes without discarding the remaining stack; missing entries are a fail-closed outcome, not a panic."
)]
fn take_sizes(
    mut sizes: alloc::vec::Vec<u32>,
    count: usize,
) -> (alloc::vec::Vec<u32>, Option<u32>) {
    let mut total = 1u32;
    let mut step = 0;
    let mut is_missing = false;
    while step < count {
        match sizes.pop() {
            Some(size) => total = total.saturating_add(size),
            None => {
                is_missing = true;
                break;
            }
        }
        step += 1;
    }
    if is_missing {
        (sizes, None)
    } else {
        (sizes, Some(total))
    }
}

/// Queue one unexpanded node: its expansion marker and its children in order,
/// or its unit size when it holds no children.
///
/// Each child is queued by reference: the tree under walk is only read, so the
/// walk neither copies a subtree nor needs a copy to reach a later sibling.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; queue_children records one for leaves and queues the exact constructor children with length-bounded indexing; walk_step handles the work bound and missing-size failures without assertions."
)]
fn queue_children<'a>(
    node: &'a crate::types::Ty,
    mut todo: alloc::vec::Vec<(&'a crate::types::Ty, bool)>,
    mut sizes: alloc::vec::Vec<u32>,
) -> (
    alloc::vec::Vec<(&'a crate::types::Ty, bool)>,
    alloc::vec::Vec<u32>,
) {
    match node {
        crate::types::Ty::Pair(left, right) | crate::types::Ty::Sum(left, right) => {
            todo.push((node, true));
            todo.push((left, false));
            todo.push((right, false));
        }
        crate::types::Ty::List(item) => {
            todo.push((node, true));
            todo.push((item, false));
        }
        crate::types::Ty::Program(stack_in, stack_out, _)
        | crate::types::Ty::LiveRef(stack_in, stack_out, _) => {
            todo.push((node, true));
            let mut index = 0;
            while index < stack_in.len() {
                todo.push((&stack_in[index], false));
                index += 1;
            }
            index = 0;
            while index < stack_out.len() {
                todo.push((&stack_out[index], false));
                index += 1;
            }
        }
        crate::types::Ty::Nominal(_, shape) => {
            todo.push((node, true));
            match &**shape {
                crate::types::NominalShape::Opaque(representation) => {
                    todo.push((representation, false));
                }
                crate::types::NominalShape::Variant(left, right) => {
                    todo.push((left, false));
                    todo.push((right, false));
                }
            }
        }
        crate::types::Ty::GenericNominal(_, _, shape) => {
            todo.push((node, true));
            match &**shape {
                crate::types::NominalShape::Variant(left, right) => {
                    todo.push((left, false));
                    todo.push((right, false));
                }
                crate::types::NominalShape::Opaque(representation) => {
                    todo.push((representation, false));
                }
            }
        }
        crate::types::Ty::Unit
        | crate::types::Ty::Bool
        | crate::types::Ty::I64
        | crate::types::Ty::Text
        | crate::types::Ty::Syntax
        | crate::types::Ty::Contract
        | crate::types::Ty::Evidence
        | crate::types::Ty::Certified
        | crate::types::Ty::Resource(_) => sizes.push(1),
    }
    (todo, sizes)
}
