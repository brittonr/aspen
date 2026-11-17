enum Step {
    Visit(crate::source::preflight::PathStep, usize),
    Pair,
    Sum,
    List,
    Program(usize),
    GenericNominal(noble_kernel::types::NominalTypeId, [u8; 2]),
}

struct Traversal {
    pending: alloc::vec::Vec<Step>,
    path: alloc::vec::Vec<crate::source::preflight::PathStep>,
    values: alloc::vec::Vec<noble_kernel::shapes::Pattern>,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; malformed or exhausted traversal states return diagnostics at the conversion boundary rather than asserting over externally supplied types."
)]
pub(super) fn convert(
    root: &noble_kernel::types::Ty,
    environment: &noble_kernel::contracts::Env,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::shapes::Pattern, crate::Diagnostic> {
    let mut walk = Traversal {
        pending: alloc::vec::Vec::with_capacity(1),
        path: alloc::vec::Vec::new(),
        values: alloc::vec::Vec::new(),
    };
    walk.pending
        .push(Step::Visit(crate::source::preflight::PathStep::Root, 0));
    let mut failure = None;
    while let Some(step) = walk.pending.pop() {
        if let Err(problem) = walk.step(step, root, environment, span, meter) {
            failure = Some(problem);
            break;
        }
    }
    if let Some(problem) = failure {
        return Err(problem);
    }
    if walk.values.len() != 1 {
        return Err(crate::internal(span));
    }
    walk.take(span)
}

impl Traversal {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; each charged continuation grows owned path and result vectors or constructs an allocating diagnostic, which cannot run in const evaluation."
    )]
    fn step(
        &mut self,
        step: Step,
        root: &noble_kernel::types::Ty,
        environment: &noble_kernel::contracts::Env,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.node(span));
        let value = match step {
            Step::Visit(selector, depth) => {
                self.path.truncate(depth);
                self.path.push(selector);
                let ty = attempt!(crate::source::preflight::paths::locate(
                    root, &self.path, span
                ));
                match attempt!(self.visit(ty, environment, depth, span)) {
                    Some(value) => value,
                    None => return Ok(()),
                }
            }
            step => attempt!(self.finish(step, root, span, meter)),
        };
        self.values.reserve(1);
        self.values.push(value);
        Ok(())
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; each charged type visit appends owned child selectors rather than cloning recursive types; the bounded worklist allocates."
    )]
    fn visit(
        &mut self,
        ty: &noble_kernel::types::Ty,
        environment: &noble_kernel::contracts::Env,
        depth: usize,
        span: crate::Span,
    ) -> Result<Option<noble_kernel::shapes::Pattern>, crate::Diagnostic> {
        let next_depth = depth.saturating_add(1);
        let value = match ty {
            noble_kernel::types::Ty::Unit => noble_kernel::shapes::Pattern::Unit,
            noble_kernel::types::Ty::Bool => noble_kernel::shapes::Pattern::Bool,
            noble_kernel::types::Ty::I64 => noble_kernel::shapes::Pattern::I64,
            noble_kernel::types::Ty::Text => noble_kernel::shapes::Pattern::Text,
            noble_kernel::types::Ty::Syntax => noble_kernel::shapes::Pattern::Syntax,
            noble_kernel::types::Ty::Contract => noble_kernel::shapes::Pattern::Contract,
            noble_kernel::types::Ty::Evidence => noble_kernel::shapes::Pattern::Evidence,
            noble_kernel::types::Ty::Certified => noble_kernel::shapes::Pattern::Certified,
            noble_kernel::types::Ty::Resource(kind) => {
                noble_kernel::shapes::Pattern::Resource(*kind)
            }
            noble_kernel::types::Ty::Nominal(id, shape) => {
                noble_kernel::shapes::Pattern::Nominal(*id, shape.clone())
            }
            noble_kernel::types::Ty::GenericNominal(id, _, _) => {
                let Some(decl) = environment.generic_variant(*id) else {
                    return Err(crate::invalid(span, "unknown generic nominal family"));
                };
                if !environment.valid_generic_instance(ty) {
                    return Err(crate::invalid(span, "invalid generic nominal instance"));
                }
                self.pending.reserve(3);
                self.pending
                    .push(Step::GenericNominal(*id, decl.payload_params));
                self.pending.push(Step::Visit(
                    crate::source::preflight::PathStep::GenericArgument(1),
                    next_depth,
                ));
                self.pending.push(Step::Visit(
                    crate::source::preflight::PathStep::GenericArgument(0),
                    next_depth,
                ));
                return Ok(None);
            }
            noble_kernel::types::Ty::Pair(_, _) | noble_kernel::types::Ty::Sum(_, _) => {
                self.pending.reserve(3);
                self.pending
                    .push(if matches!(ty, noble_kernel::types::Ty::Pair(_, _)) {
                        Step::Pair
                    } else {
                        Step::Sum
                    });
                self.pending.push(Step::Visit(
                    crate::source::preflight::PathStep::Right,
                    next_depth,
                ));
                self.pending.push(Step::Visit(
                    crate::source::preflight::PathStep::Left,
                    next_depth,
                ));
                return Ok(None);
            }
            noble_kernel::types::Ty::List(_) => {
                self.pending.reserve(2);
                self.pending.push(Step::List);
                self.pending.push(Step::Visit(
                    crate::source::preflight::PathStep::Item,
                    next_depth,
                ));
                return Ok(None);
            }
            noble_kernel::types::Ty::Program(input, output, _) => {
                self.pending.reserve(1);
                self.pending.push(Step::Program(depth));
                self.schedule(output.len(), true);
                self.schedule(input.len(), false);
                return Ok(None);
            }
            noble_kernel::types::Ty::LiveRef(..) => {
                return Err(crate::invalid(
                    span,
                    "borrowed live reference cannot enter a reusable definition scheme",
                ));
            }
        };
        Ok(Some(value))
    }

    fn schedule(&mut self, count: usize, is_output: bool) {
        let depth = self.path.len();
        self.pending.reserve(count);
        let mut at = count;
        while at > 0 {
            at -= 1;
            let selector = if is_output {
                crate::source::preflight::PathStep::Output(at)
            } else {
                crate::source::preflight::PathStep::Input(at)
            };
            self.pending.push(Step::Visit(selector, depth));
        }
    }

    fn finish(
        &mut self,
        step: Step,
        root: &noble_kernel::types::Ty,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<noble_kernel::shapes::Pattern, crate::Diagnostic> {
        #[expect(
            tigerstyle::fragile_exhaustive_enum_match,
            reason = "Owner: noble-maintainers; every constructor continuation must consume its exact operands and Visit is invalid here; new closed variants require an explicit construction rule."
        )]
        match step {
            Step::Pair | Step::Sum => {
                let b = attempt!(self.take(span));
                let a = attempt!(self.take(span));
                if matches!(step, Step::Pair) {
                    Ok(noble_kernel::shapes::Pattern::Pair(
                        alloc::boxed::Box::new(a),
                        alloc::boxed::Box::new(b),
                    ))
                } else {
                    Ok(noble_kernel::shapes::Pattern::Sum(
                        alloc::boxed::Box::new(a),
                        alloc::boxed::Box::new(b),
                    ))
                }
            }
            Step::List => Ok(noble_kernel::shapes::Pattern::List(alloc::boxed::Box::new(
                attempt!(self.take(span)),
            ))),
            Step::GenericNominal(id, payload_params) => {
                let second = attempt!(self.take(span));
                let first = attempt!(self.take(span));
                Ok(noble_kernel::shapes::Pattern::GenericNominal(
                    id,
                    alloc::boxed::Box::new([first, second]),
                    payload_params,
                ))
            }
            Step::Program(depth) => self.program(root, depth, span, meter),
            Step::Visit(_, _) => Err(crate::internal(span)),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; finishing a program transfers owned operands with Vec::split_off and allocates its exact effect slots; neither ownership transition is const-compatible."
    )]
    fn program(
        &mut self,
        root: &noble_kernel::types::Ty,
        depth: usize,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<noble_kernel::shapes::Pattern, crate::Diagnostic> {
        self.path.truncate(depth.saturating_add(1));
        let (input, output, effects) = match attempt!(crate::source::preflight::paths::locate(
            root, &self.path, span
        )) {
            noble_kernel::types::Ty::Program(input, output, effects) => (input, output, effects),
            _ => return Err(crate::internal(span)),
        };
        if self.values.len() < input.len().saturating_add(output.len()) {
            return Err(crate::internal(span));
        }
        let outputs = self
            .values
            .split_off(self.values.len().saturating_sub(output.len()));
        let inputs = self
            .values
            .split_off(self.values.len().saturating_sub(input.len()));
        Ok(noble_kernel::shapes::Pattern::program(
            inputs,
            outputs,
            attempt!(super::effect_slots(effects, span, meter)),
        ))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; pop transfers an owned recursive pattern out of the private result stack, and a missing operand returns an allocating diagnostic."
    )]
    fn take(
        &mut self,
        span: crate::Span,
    ) -> Result<noble_kernel::shapes::Pattern, crate::Diagnostic> {
        match self.values.pop() {
            Some(value) => Ok(value),
            None => Err(crate::internal(span)),
        }
    }
}
