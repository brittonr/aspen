//! One-segment helpers for substitution: a single segment must hold exactly
//! one type to become a `pair`, `sum`, or `list` payload.

pub(super) fn single(
    segment: alloc::vec::Vec<crate::types::Ty>,
) -> Result<crate::types::Ty, crate::words::InstError> {
    let mut items = segment;
    if items.len() == 1 {
        match items.pop() {
            Some(ty) => Ok(ty),
            None => Err(crate::words::InstError::OversizedType),
        }
    } else {
        Err(crate::words::InstError::OversizedType)
    }
}

/// Complete one `list` pattern's payload from its single finished segment.
pub(super) fn list_segment(
    only: alloc::vec::Vec<crate::types::Ty>,
) -> Result<alloc::vec::Vec<crate::types::Ty>, crate::words::InstError> {
    let inner = attempt!(single(only));
    Ok(alloc::vec![crate::types::Ty::List(alloc::boxed::Box::new(
        inner,
    ))])
}

pub(super) fn pair_segment(
    left: alloc::vec::Vec<crate::types::Ty>,
    right: alloc::vec::Vec<crate::types::Ty>,
    is_sum: bool,
) -> Result<alloc::vec::Vec<crate::types::Ty>, crate::words::InstError> {
    let left_ty = attempt!(single(left));
    let right_ty = attempt!(single(right));
    let built = if is_sum {
        crate::types::Ty::Sum(
            alloc::boxed::Box::new(left_ty),
            alloc::boxed::Box::new(right_ty),
        )
    } else {
        crate::types::Ty::Pair(
            alloc::boxed::Box::new(left_ty),
            alloc::boxed::Box::new(right_ty),
        )
    };
    Ok(alloc::vec![built])
}
