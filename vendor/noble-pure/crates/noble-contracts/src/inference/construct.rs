impl<'a> super::build::State<'a> {
    pub(super) fn build_ty(
        mut self,
        arena: &mut super::Arena,
        ty: &'a noble_kernel::types::Ty,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Self, crate::Diagnostic> {
        let term = match ty {
            noble_kernel::types::Ty::Unit => super::Term::Unit,
            noble_kernel::types::Ty::Bool => super::Term::Bool,
            noble_kernel::types::Ty::I64 => super::Term::I64,
            noble_kernel::types::Ty::Text => super::Term::Text,
            noble_kernel::types::Ty::Syntax => super::Term::Syntax,
            noble_kernel::types::Ty::Contract => super::Term::Contract,
            noble_kernel::types::Ty::Evidence => super::Term::Evidence,
            noble_kernel::types::Ty::Certified => super::Term::Certified,
            noble_kernel::types::Ty::Nominal(id, shape) => {
                if !arena
                    .nominals
                    .iter()
                    .any(|(known, checked)| known == id && checked == shape.as_ref())
                {
                    return Err(crate::invalid(
                        span,
                        "unregistered nominal type or changed representation",
                    ));
                }
                super::Term::Nominal(*id)
            }
            noble_kernel::types::Ty::GenericNominal(id, arguments, shape) => {
                let Some(declaration) = arena.generic_nominals.iter().find(|decl| decl.id == *id)
                else {
                    return Err(crate::invalid(span, "unregistered generic nominal family"));
                };
                let expected = noble_kernel::types::NominalShape::Variant(
                    alloc::boxed::Box::new(
                        arguments[usize::from(declaration.payload_params[0])].clone(),
                    ),
                    alloc::boxed::Box::new(
                        arguments[usize::from(declaration.payload_params[1])].clone(),
                    ),
                );
                if **shape != expected {
                    return Err(crate::invalid(
                        span,
                        "changed generic nominal representation",
                    ));
                }
                self.steps.push(super::build::Step::GenericNominal(*id));
                self.steps.push(super::build::Step::Ty(&arguments[1]));
                self.steps.push(super::build::Step::Ty(&arguments[0]));
                return Ok(self);
            }
            noble_kernel::types::Ty::Pair(a, b) | noble_kernel::types::Ty::Sum(a, b) => {
                self.steps
                    .push(if matches!(ty, noble_kernel::types::Ty::Pair(_, _)) {
                        super::build::Step::Pair
                    } else {
                        super::build::Step::Sum
                    });
                self.steps.push(super::build::Step::Ty(b));
                self.steps.push(super::build::Step::Ty(a));
                return Ok(self);
            }
            noble_kernel::types::Ty::List(item) => {
                self.steps.push(super::build::Step::List);
                self.steps.push(super::build::Step::Ty(item));
                return Ok(self);
            }
            noble_kernel::types::Ty::Program(inputs, outputs, effects) => {
                if !arena.effectful && !effects.is_empty() {
                    return Err(crate::Diagnostic::new(
                        crate::DiagnosticKind::Unsupported,
                        span,
                        "effectful Program is outside the pure fragment",
                    ));
                }
                let effect = if arena.effectful {
                    Some(attempt!(
                        arena.program_type_effect_constant(effects, span, meter)
                    ))
                } else {
                    None
                };
                self.steps.push(super::build::Step::Program(effect));
                self.steps.push(super::build::Step::StackTy(outputs));
                self.steps.push(super::build::Step::StackTy(inputs));
                return Ok(self);
            }
            noble_kernel::types::Ty::LiveRef(..) => {
                return Err(crate::invalid(
                    span,
                    "LiveRef is not a value; it belongs only to a live root input sidecar",
                ));
            }
            noble_kernel::types::Ty::Resource(kind) => {
                if arena.resources {
                    self.values.push(attempt!(arena.add(
                        super::Term::Resource(*kind),
                        span,
                        meter
                    )));
                    return Ok(self);
                }
                return Err(crate::Diagnostic::new(
                    crate::DiagnosticKind::Unsupported,
                    span,
                    "host resources are outside the pure fragment",
                ));
            }
        };
        self.values.push(attempt!(arena.add(term, span, meter)));
        Ok(self)
    }

    pub(super) fn build_pattern(
        mut self,
        arena: &mut super::Arena,
        pattern: &'a noble_kernel::shapes::Pattern,
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Self, crate::Diagnostic> {
        let term = match pattern {
            noble_kernel::shapes::Pattern::Unit => super::Term::Unit,
            noble_kernel::shapes::Pattern::Bool => super::Term::Bool,
            noble_kernel::shapes::Pattern::I64 => super::Term::I64,
            noble_kernel::shapes::Pattern::Text => super::Term::Text,
            noble_kernel::shapes::Pattern::Syntax => super::Term::Syntax,
            noble_kernel::shapes::Pattern::Contract => super::Term::Contract,
            noble_kernel::shapes::Pattern::Evidence => super::Term::Evidence,
            noble_kernel::shapes::Pattern::Certified => super::Term::Certified,
            noble_kernel::shapes::Pattern::Nominal(id, shape) => {
                if !arena
                    .nominals
                    .iter()
                    .any(|(known, checked)| known == id && checked == shape.as_ref())
                {
                    return Err(crate::invalid(
                        span,
                        "unregistered nominal pattern or changed representation",
                    ));
                }
                super::Term::Nominal(*id)
            }
            noble_kernel::shapes::Pattern::GenericNominal(id, arguments, payload_params) => {
                if !arena.generic_nominals.iter().any(|declaration| {
                    declaration.id == *id && declaration.payload_params == *payload_params
                }) {
                    return Err(crate::invalid(span, "unregistered generic nominal pattern"));
                }
                self.steps.push(super::build::Step::GenericNominal(*id));
                self.steps.push(super::build::Step::Pattern(&arguments[1]));
                self.steps.push(super::build::Step::Pattern(&arguments[0]));
                return Ok(self);
            }
            noble_kernel::shapes::Pattern::Pair(a, b)
            | noble_kernel::shapes::Pattern::Sum(a, b) => {
                self.steps.push(
                    if matches!(pattern, noble_kernel::shapes::Pattern::Pair(_, _)) {
                        super::build::Step::Pair
                    } else {
                        super::build::Step::Sum
                    },
                );
                self.steps.push(super::build::Step::Pattern(b));
                self.steps.push(super::build::Step::Pattern(a));
                return Ok(self);
            }
            noble_kernel::shapes::Pattern::List(item) => {
                self.steps.push(super::build::Step::List);
                self.steps.push(super::build::Step::Pattern(item));
                return Ok(self);
            }
            noble_kernel::shapes::Pattern::Program(inputs, outputs, effects) => {
                let effect = if arena.effectful {
                    Some(attempt!(arena.program_type_effect_pattern(
                        effects, variables, span, meter
                    )))
                } else {
                    attempt!(super::pure_effects(effects, variables, span, meter));
                    None
                };
                self.steps.push(super::build::Step::Program(effect));
                self.steps.push(super::build::Step::StackPattern(outputs));
                self.steps.push(super::build::Step::StackPattern(inputs));
                return Ok(self);
            }
            noble_kernel::shapes::Pattern::LiveRef(..) => {
                return Err(crate::invalid(
                    span,
                    "LiveRef cannot be instantiated as a value or captured in a Program",
                ));
            }
            noble_kernel::shapes::Pattern::Var(variable) => {
                match super::variable_at(variables, variable.0, span) {
                    Ok(super::Variable::Value(id)) => self.values.push(id),
                    Ok(
                        super::Variable::Stack(_)
                        | super::Variable::Effect
                        | super::Variable::EffectValue(_),
                    )
                    | Err(_) => {
                        return Err(crate::internal(span));
                    }
                }
                return Ok(self);
            }
            noble_kernel::shapes::Pattern::Resource(kind) => {
                if !arena.resources {
                    return Err(crate::invalid(
                        span,
                        "resource pattern requires a component environment",
                    ));
                }
                super::Term::Resource(*kind)
            }
            noble_kernel::shapes::Pattern::StackVar(_) => return Err(crate::internal(span)),
        };
        self.values.push(attempt!(arena.add(term, span, meter)));
        Ok(self)
    }
}
