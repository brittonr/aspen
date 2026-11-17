macro_rules! attempt_segment {
    ($walk:expr, $segment:expr) => {
        match crate::words::segments::single($segment) {
            Ok(ty) => ty,
            Err(problem) => return ($walk, Err(problem)),
        }
    };
}

/// Complete a pair, sum, or list from its finished segments.
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; required children and constructor kind fail through InstError, preserving right-then-left pops. Every new Pattern must require a reviewed completion rule, not a fallback."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-kernel; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
pub(super) fn apply(node: crate::shapes::Pattern, mut walk: super::Walk) -> super::StepState {
    if let crate::shapes::Pattern::GenericNominal(id, _, mapping) = node {
        let (Some(second), Some(first)) = (walk.segments.pop(), walk.segments.pop()) else {
            return (walk, Err(crate::words::InstError::OversizedType));
        };
        if !matches!(mapping, [0, 1] | [1, 0]) {
            return (walk, Err(crate::words::InstError::KindMismatch));
        }
        let first = attempt_segment!(walk, first);
        let second = attempt_segment!(walk, second);
        // Both arguments are copied into the descriptor, so one level doubles
        // the node count. Bound the result before it is materialized: nested
        // generic patterns otherwise grow exponentially inside the pattern
        // bound and only fail long after the declared type-size limit.
        let (Some(first_nodes), Some(second_nodes)) =
            (first.physical_nodes(), second.physical_nodes())
        else {
            return (walk, Err(crate::words::InstError::OversizedType));
        };
        let doubled = match first_nodes
            .checked_add(second_nodes)
            .and_then(|nodes| nodes.checked_mul(2))
            .and_then(|nodes| nodes.checked_add(1))
        {
            Some(doubled) => doubled,
            None => return (walk, Err(crate::words::InstError::OversizedType)),
        };
        if doubled > crate::types::PHYSICAL_NODE_CAP {
            return (walk, Err(crate::words::InstError::OversizedType));
        }
        let args = [first, second];
        let shape = crate::types::NominalShape::Variant(
            alloc::boxed::Box::new(args[usize::from(mapping[0])].clone()),
            alloc::boxed::Box::new(args[usize::from(mapping[1])].clone()),
        );
        walk.segments
            .push(alloc::vec![crate::types::Ty::GenericNominal(
                id,
                alloc::boxed::Box::new(args),
                alloc::boxed::Box::new(shape),
            )]);
        return (walk, Ok(()));
    }
    // A list completes from its single segment; pair and sum from two.
    if let crate::shapes::Pattern::List(_) = node {
        return match walk
            .segments
            .pop()
            .map(crate::words::segments::list_segment)
        {
            Some(Ok(segment)) => {
                walk.segments.push(segment);
                (walk, Ok(()))
            }
            Some(Err(problem)) => (walk, Err(problem)),
            None => (walk, Err(crate::words::InstError::OversizedType)),
        };
    }
    // The first pop is the top: the right child's finished segment.
    let popped = (walk.segments.pop(), walk.segments.pop());
    let (left, right) = match popped {
        (Some(right), Some(left)) => (left, right),
        _ => return (walk, Err(crate::words::InstError::OversizedType)),
    };
    let built = match node {
        crate::shapes::Pattern::Pair(_, _) => {
            crate::words::segments::pair_segment(left, right, false)
        }
        crate::shapes::Pattern::Sum(_, _) => {
            crate::words::segments::pair_segment(left, right, true)
        }
        crate::shapes::Pattern::Var(_)
        | crate::shapes::Pattern::StackVar(_)
        | crate::shapes::Pattern::List(_)
        | crate::shapes::Pattern::Program(_, _, _)
        | crate::shapes::Pattern::LiveRef(_, _, _)
        | crate::shapes::Pattern::Unit
        | crate::shapes::Pattern::Bool
        | crate::shapes::Pattern::I64
        | crate::shapes::Pattern::Text
        | crate::shapes::Pattern::Syntax
        | crate::shapes::Pattern::Contract
        | crate::shapes::Pattern::Evidence
        | crate::shapes::Pattern::Certified
        | crate::shapes::Pattern::Resource(_) => Err(crate::words::InstError::KindMismatch),
        crate::shapes::Pattern::Nominal(_, _) => Err(crate::words::InstError::KindMismatch),
        crate::shapes::Pattern::GenericNominal(_, _, _) => {
            Err(crate::words::InstError::KindMismatch)
        }
    };
    match built {
        Ok(segment) => {
            walk.segments.push(segment);
            (walk, Ok(()))
        }
        Err(problem) => (walk, Err(problem)),
    }
}
