struct Traversal {
    pending: alloc::vec::Vec<noble_kernel::types::Ty>,
}

pub(super) fn check(
    ty: &noble_kernel::types::Ty,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    // Prepared operand types have at most TYPE_CAP constructors. Owned nodes
    // keep mutable-stack borrows out of the extracted loop's backedge.
    let mut traversal = Traversal {
        pending: alloc::vec::Vec::with_capacity(1),
    };
    traversal.pending.push(ty.clone());
    let mut failure = None;
    while let Some(ty) = traversal.pending.pop() {
        if let Err(error) = step(ty, &mut traversal, span, meter) {
            failure = Some(error);
            break;
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; structural traversal consumes allocating Ty values, grows a worklist, and reports unsupported leaf types."
)]
fn step(
    ty: noble_kernel::types::Ty,
    traversal: &mut Traversal,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    match ty {
        noble_kernel::types::Ty::Unit | noble_kernel::types::Ty::Bool | noble_kernel::types::Ty::I64 | noble_kernel::types::Ty::Text => {},
        noble_kernel::types::Ty::Pair(a, b) | noble_kernel::types::Ty::Sum(a, b) => { traversal.pending.push(*a); traversal.pending.push(*b); }
        noble_kernel::types::Ty::List(item) => traversal.pending.push(*item),
        noble_kernel::types::Ty::Program(_, _, _)
        | noble_kernel::types::Ty::LiveRef(_, _, _)
        | noble_kernel::types::Ty::Syntax
        | noble_kernel::types::Ty::Contract
        | noble_kernel::types::Ty::Evidence
        | noble_kernel::types::Ty::Certified
        | noble_kernel::types::Ty::Nominal(_, _)
        | noble_kernel::types::Ty::GenericNominal(_, _, _)
        | noble_kernel::types::Ty::Resource(_) => return Err(crate::Diagnostic::new(crate::DiagnosticKind::Unsupported, span, "eq operands and maps input/result must be scalar or structural data, without nested Program, Syntax, companion, or resource types")),
    }
    Ok(())
}
