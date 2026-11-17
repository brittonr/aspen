impl super::super::Arena {
    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-maintainers; this step allocates arena/worklist entries. The immediately preceding nonempty check proves len - 1 cannot underflow."
    )]
    pub(super) fn build_stack_ty<'a>(
        &mut self,
        stack: &'a [noble_kernel::types::Ty],
        mut state: super::State<'a>,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<super::State<'a>, crate::Diagnostic> {
        state
            .values
            .push(attempt!(self.add(super::super::Term::Empty, span, meter)));
        if !stack.is_empty() {
            state
                .steps
                .push(super::Step::StackTyParts(stack, stack.len() - 1));
        }
        Ok(state)
    }

    #[expect(
        tigerstyle::raw_arithmetic_overflow,
        reason = "Owner: noble-maintainers; this step allocates arena/worklist entries. len > start and start >= 0 prove len - 1 cannot underflow."
    )]
    pub(super) fn build_stack_pattern<'a>(
        &mut self,
        stack: &'a [noble_kernel::shapes::Pattern],
        mut state: super::State<'a>,
        variables: &[super::super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<super::State<'a>, crate::Diagnostic> {
        let mut start = 0usize;
        let initial = match stack.first() {
            Some(noble_kernel::shapes::Pattern::StackVar(variable)) => {
                start = 1;
                match attempt!(super::super::variable_at(variables, variable.0, span)) {
                    super::super::Variable::Stack(id) => id,
                    super::super::Variable::Value(_)
                    | super::super::Variable::Effect
                    | super::super::Variable::EffectValue(_) => {
                        return Err(crate::internal(span));
                    }
                }
            }
            Some(
                noble_kernel::shapes::Pattern::Unit
                | noble_kernel::shapes::Pattern::Bool
                | noble_kernel::shapes::Pattern::I64
                | noble_kernel::shapes::Pattern::Text
                | noble_kernel::shapes::Pattern::Syntax
                | noble_kernel::shapes::Pattern::Contract
                | noble_kernel::shapes::Pattern::Evidence
                | noble_kernel::shapes::Pattern::Certified
                | noble_kernel::shapes::Pattern::Pair(_, _)
                | noble_kernel::shapes::Pattern::Sum(_, _)
                | noble_kernel::shapes::Pattern::List(_)
                | noble_kernel::shapes::Pattern::Program(_, _, _)
                | noble_kernel::shapes::Pattern::LiveRef(_, _, _)
                | noble_kernel::shapes::Pattern::Var(_)
                | noble_kernel::shapes::Pattern::Resource(_)
                | noble_kernel::shapes::Pattern::Nominal(_, _)
                | noble_kernel::shapes::Pattern::GenericNominal(_, _, _),
            )
            | None => attempt!(self.add(super::super::Term::Empty, span, meter)),
        };
        state.values.push(initial);
        if stack.len() > start {
            state.steps.push(super::Step::StackPatternParts(
                stack,
                start,
                stack.len() - 1,
            ));
        }
        Ok(state)
    }
}

#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; scheduling grows a Vec and is non-const. The local at > 0 guard proves at - 1 cannot underflow."
)]
pub(super) fn ty_step<'a>(
    stack: &'a [noble_kernel::types::Ty],
    at: usize,
    mut state: super::State<'a>,
    span: crate::Span,
) -> Result<super::State<'a>, crate::Diagnostic> {
    let ty = match stack.get(at) {
        Some(ty) => ty,
        None => return Err(crate::internal(span)),
    };
    state.steps.push(super::Step::Push);
    state.steps.push(super::Step::Ty(ty));
    // Scan first, then evaluate: every scan keeps its original charge and order.
    // The continuation and its borrowed elements move together through build.
    if at > 0 {
        state.steps.push(super::Step::StackTyParts(stack, at - 1));
    }
    Ok(state)
}

#[expect(
    tigerstyle::raw_arithmetic_overflow,
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; scheduling grows a Vec. start and at are related bounds of the same reverse scan; at > start proves at - 1 is safe and keeps the prefix excluded."
)]
pub(super) fn pattern_step<'a>(
    stack: &'a [noble_kernel::shapes::Pattern],
    start: usize,
    at: usize,
    mut state: super::State<'a>,
    span: crate::Span,
) -> Result<super::State<'a>, crate::Diagnostic> {
    let pattern = match stack.get(at) {
        Some(pattern) => pattern,
        None => return Err(crate::internal(span)),
    };
    state.steps.push(super::Step::Push);
    state.steps.push(super::Step::Pattern(pattern));
    if at > start {
        state
            .steps
            .push(super::Step::StackPatternParts(stack, start, at - 1));
    }
    Ok(state)
}
