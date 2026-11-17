mod stack;

#[octet::sealed_enum]
pub(super) enum Step<'a> {
    Ty(&'a noble_kernel::types::Ty),
    Pattern(&'a noble_kernel::shapes::Pattern),
    StackTy(&'a [noble_kernel::types::Ty]),
    StackPattern(&'a [noble_kernel::shapes::Pattern]),
    StackTyParts(&'a [noble_kernel::types::Ty], usize),
    StackPatternParts(&'a [noble_kernel::shapes::Pattern], usize, usize),
    Pair,
    Sum,
    GenericNominal(noble_kernel::types::NominalTypeId),
    List,
    Program(Option<u32>),
    Push,
}

pub(super) struct State<'a> {
    pub(super) steps: alloc::vec::Vec<Step<'a>>,
    pub(super) values: alloc::vec::Vec<u32>,
}

impl super::Arena {
    pub fn rigid_variables(
        &mut self,
        kinds: &[noble_kernel::words::VariableKind],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<super::Variable>, crate::Diagnostic> {
        let mut variables = alloc::vec::Vec::with_capacity(kinds.len());
        for kind in kinds {
            attempt!(meter.node(span));
            let identity = self.rigid_count;
            self.rigid_count = attempt!(identity
                .checked_add(1)
                .ok_or_else(|| { crate::invalid(span, "rigid signature binder limit exceeded") }));
            let variable = match kind {
                noble_kernel::words::VariableKind::Value => super::Variable::Value(attempt!(
                    self.add(super::Term::RigidValue(identity), span, meter)
                )),
                noble_kernel::words::VariableKind::Stack => super::Variable::Stack(attempt!(
                    self.add(super::Term::RigidStack(identity), span, meter)
                )),
                noble_kernel::words::VariableKind::Effect => {
                    super::Variable::EffectValue(attempt!(self.rigid_effect(identity, span, meter)))
                }
            };
            variables.push(variable);
        }
        Ok(variables)
    }

    pub fn stack(
        &mut self,
        stack: &[noble_kernel::types::Ty],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.build(Step::StackTy(stack), &[], span, meter)
    }

    pub fn ty(
        &mut self,
        ty: &noble_kernel::types::Ty,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.build(Step::Ty(ty), &[], span, meter)
    }

    pub fn pattern_stack(
        &mut self,
        stack: &[noble_kernel::shapes::Pattern],
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.build(Step::StackPattern(stack), variables, span, meter)
    }

    fn build(
        &mut self,
        root: Step<'_>,
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        let mut state = State {
            steps: alloc::vec::Vec::with_capacity(1),
            values: alloc::vec::Vec::new(),
        };
        state.steps.push(root);
        while let Some(step) = state.steps.pop() {
            state = attempt!(self.build_step(step, state, variables, span, meter));
        }
        if state.values.len() != 1 {
            return Err(crate::internal(span));
        }
        require_id(state.values.pop(), span)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; this allocating worklist interpreter must explicitly handle every Step variant, including impossible constructor cases as typed internal errors."
    )]
    fn build_step<'a>(
        &mut self,
        step: Step<'a>,
        mut state: State<'a>,
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<State<'a>, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match step {
            Step::Ty(ty) => state.build_ty(self, ty, span, meter),
            Step::Pattern(pattern) => state.build_pattern(self, pattern, variables, span, meter),
            Step::StackTy(stack) => self.build_stack_ty(stack, state, span, meter),
            Step::StackPattern(stack) => {
                self.build_stack_pattern(stack, state, variables, span, meter)
            }
            Step::StackTyParts(stack, at) => stack::ty_step(stack, at, state, span),
            Step::StackPatternParts(stack, start, at) => {
                stack::pattern_step(stack, start, at, state, span)
            }
            Step::Pair | Step::Sum | Step::GenericNominal(_) | Step::Program(_) | Step::Push => {
                let b = attempt!(require_id(state.values.pop(), span));
                let a = attempt!(require_id(state.values.pop(), span));
                if let Step::Program(Some(effect)) = step {
                    state.values.push(attempt!(self.program(
                        super::Program {
                            input: a,
                            output: b,
                            effect,
                        },
                        span,
                        meter
                    )));
                    return Ok(state);
                }
                let term = match step {
                    Step::Pair => super::Term::Pair(a, b),
                    Step::Sum => super::Term::Sum(a, b),
                    Step::GenericNominal(id) => super::Term::GenericNominal(id, a, b),
                    Step::Program(_) => super::Term::Program(a, b),
                    Step::Push => super::Term::Push(a, b),
                    Step::Ty(_)
                    | Step::Pattern(_)
                    | Step::StackTy(_)
                    | Step::StackPattern(_)
                    | Step::StackTyParts(_, _)
                    | Step::StackPatternParts(_, _, _)
                    | Step::List => return Err(crate::internal(span)),
                };
                state.values.push(attempt!(self.add(term, span, meter)));
                Ok(state)
            }
            Step::List => {
                let item = attempt!(require_id(state.values.pop(), span));
                state
                    .values
                    .push(attempt!(self.add(super::Term::List(item), span, meter)));
                Ok(state)
            }
        }
    }

    pub fn variables(
        &mut self,
        kinds: &[noble_kernel::words::VariableKind],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<super::Variable>, crate::Diagnostic> {
        let mut variables = alloc::vec::Vec::with_capacity(kinds.len());
        let mut at = 0usize;
        let mut failure = None;
        while at < kinds.len() {
            match self.variable(kinds.get(at), span, meter) {
                Ok(variable) => {
                    variables.push(variable);
                    at += 1;
                }
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(variables),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; creating a value or stack witness allocates a term in the arena."
    )]
    fn variable(
        &mut self,
        kind: Option<&noble_kernel::words::VariableKind>,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<super::Variable, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match kind {
            Some(noble_kernel::words::VariableKind::Value) => Ok(super::Variable::Value(attempt!(
                self.add(super::Term::Hole(super::Sort::Value), span, meter)
            ))),
            Some(noble_kernel::words::VariableKind::Stack) => Ok(super::Variable::Stack(attempt!(
                self.add(super::Term::Hole(super::Sort::Stack), span, meter)
            ))),
            Some(noble_kernel::words::VariableKind::Effect) => {
                if self.effectful {
                    Ok(super::Variable::EffectValue(attempt!(
                        self.effect_hole(span, meter)
                    )))
                } else {
                    Ok(super::Variable::Effect)
                }
            }
            None => Err(crate::internal(span)),
        }
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; the missing-ID path constructs an owned internal diagnostic."
)]
fn require_id(value: Option<u32>, span: crate::Span) -> Result<u32, crate::Diagnostic> {
    match value {
        Some(value) => Ok(value),
        None => Err(crate::internal(span)),
    }
}
