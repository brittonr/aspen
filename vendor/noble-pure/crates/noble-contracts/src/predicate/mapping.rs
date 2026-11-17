#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; Program purity, singleton interfaces, structural eligibility, and argument/result types are explicit fallible checks."
)]
pub(super) fn apply(
    operands: (
        super::application::Operand<'_>,
        super::application::Operand<'_>,
        super::application::Operand<'_>,
    ),
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    let (program, argument, result) = operands;
    let (inputs, outputs, effects) = match &program.expr.ty {
        noble_kernel::types::Ty::Program(inputs, outputs, effects) => (inputs, outputs, effects),
        noble_kernel::types::Ty::LiveRef(..)
        | noble_kernel::types::Ty::Unit
        | noble_kernel::types::Ty::Bool
        | noble_kernel::types::Ty::I64
        | noble_kernel::types::Ty::Text
        | noble_kernel::types::Ty::Pair(_, _)
        | noble_kernel::types::Ty::Sum(_, _)
        | noble_kernel::types::Ty::List(_)
        | noble_kernel::types::Ty::Syntax
        | noble_kernel::types::Ty::Contract
        | noble_kernel::types::Ty::Evidence
        | noble_kernel::types::Ty::Certified
        | noble_kernel::types::Ty::Nominal(_, _)
        | noble_kernel::types::Ty::GenericNominal(_, _, _)
        | noble_kernel::types::Ty::Resource(_) => {
            return Err(crate::invalid(
                span,
                "maps first argument must have a Program type",
            ))
        }
    };
    if !effects.is_empty() {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            span,
            "maps requires a pure Program",
        ));
    }
    if inputs.len() != 1 || outputs.len() != 1 {
        return Err(crate::invalid(
            span,
            "maps requires exactly one input value and one output value",
        ));
    }
    let input = match inputs.first() {
        Some(ty) => ty,
        None => return Err(crate::internal(span)),
    };
    let output = match outputs.first() {
        Some(ty) => ty,
        None => return Err(crate::internal(span)),
    };
    attempt!(super::structural::check(input, span, meter));
    attempt!(super::structural::check(output, span, meter));
    attempt!(super::same(input, &argument.expr.ty, span, meter));
    attempt!(super::same(output, &result.expr.ty, span, meter));
    Ok(crate::Expr {
        kind: crate::ExprKind::Maps(program.id, argument.id, result.id),
        ty: noble_kernel::types::Ty::Bool,
        span,
        // These are pure Boolean fields. Eager Boolean operators avoid the
        // pinned translator's unsupported join of the three operand borrows.
        total: program.expr.total & argument.expr.total & result.expr.total,
        uses_output: program.expr.uses_output | argument.expr.uses_output | result.expr.uses_output,
    })
}
