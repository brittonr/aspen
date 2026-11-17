pub(super) struct Form {
    pub op: super::Op,
    pub annotation: Option<noble_kernel::types::Ty>,
    pub span: crate::Span,
}

#[derive(Clone, Copy)]
pub(super) struct Operand<'a> {
    pub id: u32,
    pub expr: &'a crate::Expr,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; annotations and operand IDs are checked through typed diagnostics before unary or binary elaboration."
)]
pub(super) fn resolve(
    form: Form,
    operands: (Option<u32>, Option<u32>, Option<u32>),
    expressions: &[crate::Expr],
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    if matches!(form.op, super::Op::Nil) {
        let item = match form.annotation {
            Some(item) => item,
            None => return Err(crate::internal(form.span)),
        };
        return Ok(crate::Expr {
            kind: crate::ExprKind::Nil,
            ty: noble_kernel::types::Ty::List(alloc::boxed::Box::new(item)),
            span: form.span,
            total: true,
            uses_output: false,
        });
    }
    let (first, second, third) = operands;
    let id = attempt!(super::required(first, form.span));
    let left = Operand {
        id,
        expr: attempt!(super::get(expressions, id, form.span)),
    };
    if super::arity(form.op) == 1 {
        return super::unary::apply(form, left, meter);
    }
    let id = attempt!(super::required(second, form.span));
    let right = Operand {
        id,
        expr: attempt!(super::get(expressions, id, form.span)),
    };
    super::binary::apply(form, (left, right), third, expressions, meter)
}
