#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; unary typing is checked through fallible elaborators and partial projections are represented explicitly, not asserted away."
)]
pub(super) fn apply(
    form: super::application::Form,
    operand: super::application::Operand<'_>,
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    let (kind, ty) = match form.op {
        super::Op::Not => {
            attempt!(super::same(
                &operand.expr.ty,
                &noble_kernel::types::Ty::Bool,
                form.span,
                meter
            ));
            (
                crate::ExprKind::Not(operand.id),
                noble_kernel::types::Ty::Bool,
            )
        }
        super::Op::First | super::Op::Second => attempt!(pair(form.op, operand, form.span)),
        super::Op::Inl | super::Op::Inr => {
            attempt!(inject(form.op, operand, form.annotation, form.span))
        }
        super::Op::IsLeft | super::Op::Left | super::Op::Right => {
            attempt!(sum(form.op, operand, form.span))
        }
        super::Op::IsNil | super::Op::Head | super::Op::Tail | super::Op::Length => {
            attempt!(list(form.op, operand, form.span))
        }
        super::Op::And
        | super::Op::Or
        | super::Op::Implies
        | super::Op::Eq
        | super::Op::Lt
        | super::Op::Le
        | super::Op::Add
        | super::Op::Sub
        | super::Op::Mul
        | super::Op::Pair
        | super::Op::Nil
        | super::Op::Cons
        | super::Op::Maps => return Err(crate::internal(form.span)),
    };
    let is_partial = matches!(
        form.op,
        super::Op::Left | super::Op::Right | super::Op::Head | super::Op::Tail
    );
    Ok(crate::Expr {
        kind,
        ty,
        span: form.span,
        total: operand.expr.total && !is_partial,
        uses_output: operand.expr.uses_output,
    })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; a pair projection clones an owned structural component or allocates a type diagnostic."
)]
fn pair(
    op: super::Op,
    operand: super::application::Operand<'_>,
    span: crate::Span,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    match &operand.expr.ty {
        noble_kernel::types::Ty::Pair(left, right) => {
            if matches!(op, super::Op::First) {
                Ok((crate::ExprKind::First(operand.id), (**left).clone()))
            } else {
                Ok((crate::ExprKind::Second(operand.id), (**right).clone()))
            }
        }
        noble_kernel::types::Ty::Unit
        | noble_kernel::types::Ty::Bool
        | noble_kernel::types::Ty::I64
        | noble_kernel::types::Ty::Text
        | noble_kernel::types::Ty::Sum(_, _)
        | noble_kernel::types::Ty::List(_)
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
            "pair projection requires a Pair value",
        )),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; missing annotations are explicit errors and either injection constructs the corresponding owned Sum type without a panic precondition."
)]
fn inject(
    op: super::Op,
    operand: super::application::Operand<'_>,
    annotation: Option<noble_kernel::types::Ty>,
    span: crate::Span,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    let other = match annotation {
        Some(other) => other,
        None => return Err(crate::internal(span)),
    };
    if matches!(op, super::Op::Inl) {
        Ok((
            crate::ExprKind::Inl(operand.id),
            noble_kernel::types::Ty::Sum(
                alloc::boxed::Box::new(operand.expr.ty.clone()),
                alloc::boxed::Box::new(other),
            ),
        ))
    } else {
        Ok((
            crate::ExprKind::Inr(operand.id),
            noble_kernel::types::Ty::Sum(
                alloc::boxed::Box::new(other),
                alloc::boxed::Box::new(operand.expr.ty.clone()),
            ),
        ))
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; Sum eligibility is a typed validation error; projections clone owned component types and preserve partiality at the caller."
)]
fn sum(
    op: super::Op,
    operand: super::application::Operand<'_>,
    span: crate::Span,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    let (left, right) = match &operand.expr.ty {
        noble_kernel::types::Ty::Sum(left, right) => (left, right),
        noble_kernel::types::Ty::Unit
        | noble_kernel::types::Ty::Bool
        | noble_kernel::types::Ty::I64
        | noble_kernel::types::Ty::Text
        | noble_kernel::types::Ty::Pair(_, _)
        | noble_kernel::types::Ty::List(_)
        | noble_kernel::types::Ty::Program(_, _, _)
        | noble_kernel::types::Ty::LiveRef(_, _, _)
        | noble_kernel::types::Ty::Syntax
        | noble_kernel::types::Ty::Contract
        | noble_kernel::types::Ty::Evidence
        | noble_kernel::types::Ty::Certified
        | noble_kernel::types::Ty::Nominal(_, _)
        | noble_kernel::types::Ty::GenericNominal(_, _, _)
        | noble_kernel::types::Ty::Resource(_) => {
            return Err(crate::invalid(span, "sum operation requires a Sum value"))
        }
    };
    match op {
        super::Op::IsLeft => Ok((
            crate::ExprKind::IsLeft(operand.id),
            noble_kernel::types::Ty::Bool,
        )),
        super::Op::Left => Ok((crate::ExprKind::Left(operand.id), (**left).clone())),
        super::Op::Right => Ok((crate::ExprKind::Right(operand.id), (**right).clone())),
        super::Op::Not
        | super::Op::And
        | super::Op::Or
        | super::Op::Implies
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
        | super::Op::Nil
        | super::Op::Cons
        | super::Op::IsNil
        | super::Op::Head
        | super::Op::Tail
        | super::Op::Length
        | super::Op::Maps => Err(crate::internal(span)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; List eligibility is a typed validation error; projections clone owned element/list types and preserve partiality at the caller."
)]
fn list(
    op: super::Op,
    operand: super::application::Operand<'_>,
    span: crate::Span,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    let item = match &operand.expr.ty {
        noble_kernel::types::Ty::List(item) => item,
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
        | noble_kernel::types::Ty::Resource(_) => {
            return Err(crate::invalid(span, "list operation requires a List value"))
        }
    };
    match op {
        super::Op::IsNil => Ok((
            crate::ExprKind::IsNil(operand.id),
            noble_kernel::types::Ty::Bool,
        )),
        super::Op::Head => Ok((crate::ExprKind::Head(operand.id), (**item).clone())),
        super::Op::Tail => Ok((crate::ExprKind::Tail(operand.id), operand.expr.ty.clone())),
        super::Op::Length => Ok((
            crate::ExprKind::Length(operand.id),
            noble_kernel::types::Ty::I64,
        )),
        super::Op::Not
        | super::Op::And
        | super::Op::Or
        | super::Op::Implies
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
        | super::Op::Maps => Err(crate::internal(span)),
    }
}
