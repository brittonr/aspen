#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; only the bounded private traversal builds these structural paths, and every child selection returns a diagnostic on an invalid path without recursion or input assertions."
)]
pub(in crate::source) fn locate<'a>(
    root: &'a noble_kernel::types::Ty,
    path: &[super::PathStep],
    span: crate::Span,
) -> Result<&'a noble_kernel::types::Ty, crate::Diagnostic> {
    let mut ty = root;
    let mut at = 0usize;
    let mut failure = None;
    while at < path.len() {
        match child(ty, path[at], span) {
            Ok(next) => {
                ty = next;
                at = at.saturating_add(1);
            }
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(ty),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; borrowed child selection uses non-const Box dereferencing and constructs owned diagnostics on mismatched paths."
)]
fn child(
    ty: &noble_kernel::types::Ty,
    step: super::PathStep,
    span: crate::Span,
) -> Result<&noble_kernel::types::Ty, crate::Diagnostic> {
    match (step, ty) {
        (super::PathStep::Root, _) => Ok(ty),
        (
            super::PathStep::Left,
            noble_kernel::types::Ty::Pair(left, _) | noble_kernel::types::Ty::Sum(left, _),
        ) => Ok(left),
        (
            super::PathStep::Right,
            noble_kernel::types::Ty::Pair(_, right) | noble_kernel::types::Ty::Sum(_, right),
        ) => Ok(right),
        (super::PathStep::Item, noble_kernel::types::Ty::List(item)) => Ok(item),
        (
            super::PathStep::GenericArgument(index),
            noble_kernel::types::Ty::GenericNominal(_, arguments, _),
        ) => arguments.get(index).ok_or_else(|| crate::internal(span)),
        (
            super::PathStep::Input(index),
            noble_kernel::types::Ty::Program(input, _, _)
            | noble_kernel::types::Ty::LiveRef(input, _, _),
        ) => match input.get(index) {
            Some(child) => Ok(child),
            None => Err(crate::internal(span)),
        },
        (
            super::PathStep::Output(index),
            noble_kernel::types::Ty::Program(_, output, _)
            | noble_kernel::types::Ty::LiveRef(_, output, _),
        ) => match output.get(index) {
            Some(child) => Ok(child),
            None => Err(crate::internal(span)),
        },
        _ => Err(crate::internal(span)),
    }
}
