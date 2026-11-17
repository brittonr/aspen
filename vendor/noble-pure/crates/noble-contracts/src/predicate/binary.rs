#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; operand types, structural equality, and operator arities are validated through fallible elaboration rather than panic assertions."
)]
pub(super) fn apply(
    form: super::application::Form,
    operands: (
        super::application::Operand<'_>,
        super::application::Operand<'_>,
    ),
    third: Option<u32>,
    expressions: &[crate::Expr],
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    let (left, right) = operands;
    if matches!(form.op, super::Op::Maps) {
        let id = attempt!(super::required(third, form.span));
        let result = super::application::Operand {
            id,
            expr: attempt!(super::get(expressions, id, form.span)),
        };
        return super::mapping::apply((left, right, result), form.span, meter);
    }
    let (kind, ty) = match form.op {
        super::Op::And | super::Op::Or | super::Op::Implies => {
            attempt!(boolean(form.op, operands, form.span, meter))
        }
        super::Op::Eq => {
            attempt!(super::same(&left.expr.ty, &right.expr.ty, form.span, meter));
            attempt!(super::structural::check(&left.expr.ty, form.span, meter));
            (
                crate::ExprKind::Eq(left.id, right.id),
                noble_kernel::types::Ty::Bool,
            )
        }
        super::Op::Lt | super::Op::Le | super::Op::Add | super::Op::Sub | super::Op::Mul => {
            attempt!(integer(form.op, operands, form.span, meter))
        }
        super::Op::Pair => (
            crate::ExprKind::Pair(left.id, right.id),
            noble_kernel::types::Ty::Pair(
                alloc::boxed::Box::new(left.expr.ty.clone()),
                alloc::boxed::Box::new(right.expr.ty.clone()),
            ),
        ),
        super::Op::Cons => attempt!(cons(operands, form.span, meter)),
        super::Op::Not
        | super::Op::First
        | super::Op::Second
        | super::Op::Inl
        | super::Op::Inr
        | super::Op::IsLeft
        | super::Op::Left
        | super::Op::Right
        | super::Op::Nil
        | super::Op::IsNil
        | super::Op::Head
        | super::Op::Tail
        | super::Op::Length
        | super::Op::Maps => return Err(crate::internal(form.span)),
    };
    Ok(crate::Expr {
        kind,
        ty,
        span: form.span,
        total: left.expr.total && right.expr.total,
        uses_output: left.expr.uses_output || right.expr.uses_output,
    })
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both Boolean operands are checked through non-const metering and owned type-mismatch diagnostics."
)]
fn boolean(
    op: super::Op,
    operands: (
        super::application::Operand<'_>,
        super::application::Operand<'_>,
    ),
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    let (left, right) = operands;
    attempt!(super::same(
        &left.expr.ty,
        &noble_kernel::types::Ty::Bool,
        span,
        meter
    ));
    attempt!(super::same(
        &right.expr.ty,
        &noble_kernel::types::Ty::Bool,
        span,
        meter
    ));
    let kind = match op {
        super::Op::And => crate::ExprKind::And(left.id, right.id),
        super::Op::Or => crate::ExprKind::Or(left.id, right.id),
        super::Op::Implies => crate::ExprKind::Implies(left.id, right.id),
        super::Op::Not
        | super::Op::Eq
        | super::Op::Lt
        | super::Op::Le
        | super::Op::Add
        | super::Op::Sub
        | super::Op::Mul
        | super::Op::Pair
        | super::Op::First
        | super::Op::Second
        | super::Op::Inl
        | super::Op::Inr
        | super::Op::IsLeft
        | super::Op::Left
        | super::Op::Right
        | super::Op::Nil
        | super::Op::Cons
        | super::Op::IsNil
        | super::Op::Head
        | super::Op::Tail
        | super::Op::Length
        | super::Op::Maps => return Err(crate::internal(span)),
    };
    Ok((kind, noble_kernel::types::Ty::Bool))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both I64 operands are checked through non-const metering and owned type-mismatch diagnostics."
)]
fn integer(
    op: super::Op,
    operands: (
        super::application::Operand<'_>,
        super::application::Operand<'_>,
    ),
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    let (left, right) = operands;
    attempt!(super::same(
        &left.expr.ty,
        &noble_kernel::types::Ty::I64,
        span,
        meter
    ));
    attempt!(super::same(
        &right.expr.ty,
        &noble_kernel::types::Ty::I64,
        span,
        meter
    ));
    let kind = match op {
        super::Op::Lt => crate::ExprKind::Lt(left.id, right.id),
        super::Op::Le => crate::ExprKind::Le(left.id, right.id),
        super::Op::Add => crate::ExprKind::Add(left.id, right.id),
        super::Op::Sub => crate::ExprKind::Sub(left.id, right.id),
        super::Op::Mul => crate::ExprKind::Mul(left.id, right.id),
        super::Op::Not
        | super::Op::And
        | super::Op::Or
        | super::Op::Implies
        | super::Op::Eq
        | super::Op::Pair
        | super::Op::First
        | super::Op::Second
        | super::Op::Inl
        | super::Op::Inr
        | super::Op::IsLeft
        | super::Op::Left
        | super::Op::Right
        | super::Op::Nil
        | super::Op::Cons
        | super::Op::IsNil
        | super::Op::Head
        | super::Op::Tail
        | super::Op::Length
        | super::Op::Maps => return Err(crate::internal(span)),
    };
    let ty = if matches!(op, super::Op::Lt | super::Op::Le) {
        noble_kernel::types::Ty::Bool
    } else {
        noble_kernel::types::Ty::I64
    };
    Ok((kind, ty))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; cons checks the list constructor and element equality through diagnostics, then clones an allocating structural type."
)]
fn cons(
    operands: (
        super::application::Operand<'_>,
        super::application::Operand<'_>,
    ),
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    let (left, right) = operands;
    match &right.expr.ty {
        noble_kernel::types::Ty::List(item) => {
            attempt!(super::same(&left.expr.ty, item, span, meter));
            Ok((
                crate::ExprKind::Cons(left.id, right.id),
                right.expr.ty.clone(),
            ))
        }
        noble_kernel::types::Ty::Unit
        | noble_kernel::types::Ty::Bool
        | noble_kernel::types::Ty::I64
        | noble_kernel::types::Ty::Text
        | noble_kernel::types::Ty::Pair(_, _)
        | noble_kernel::types::Ty::Sum(_, _)
        | noble_kernel::types::Ty::Program(_, _, _)
        | noble_kernel::types::Ty::LiveRef(_, _, _)
        | noble_kernel::types::Ty::Syntax
        | noble_kernel::types::Ty::Contract
        | noble_kernel::types::Ty::Evidence
        | noble_kernel::types::Ty::Certified
        | noble_kernel::types::Ty::Nominal(_, _)
        | noble_kernel::types::Ty::GenericNominal(_, _, _)
        | noble_kernel::types::Ty::Resource(_) => Err(crate::invalid(
            span,
            "cons requires an item and a list of that item type",
        )),
    }
}
