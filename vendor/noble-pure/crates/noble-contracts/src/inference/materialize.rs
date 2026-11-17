mod rendering;

#[octet::sealed_enum]
enum Step {
    Visit(u32),
    Finish(super::Term, noble_kernel::types::EffSet),
}

#[octet::sealed_enum]
pub(super) enum Material {
    Value(noble_kernel::types::Ty, Extent),
    Stack(alloc::vec::Vec<noble_kernel::types::Ty>, Extent),
}

/// Two independent measures of one materialized value or stack.
///
/// `constructors` is the logical language type size checked against
/// `TYPE_CAP`. `nodes` counts every owned `Ty` node of the Rust
/// representation, including each generic instance's retained arguments
/// *and* its separately owned canonical descriptor. Nested generic instances
/// therefore grow exponentially in `nodes` while `constructors` grows
/// linearly, so the two are checked against separate caps.
#[derive(Clone, Copy)]
pub(super) struct Extent {
    pub(super) constructors: u32,
    pub(super) nodes: u32,
}

impl Extent {
    const LEAF: Self = Self {
        constructors: 1,
        nodes: 1,
    };
    const EMPTY: Self = Self {
        constructors: 0,
        nodes: 0,
    };
}

pub(super) struct State {
    steps: alloc::vec::Vec<Step>,
    pub(super) values: alloc::vec::Vec<Material>,
}

impl State {
    pub(super) fn pop(&mut self, span: crate::Span) -> Result<Material, crate::Diagnostic> {
        match self.values.pop() {
            Some(value) => Ok(value),
            None => Err(crate::internal(span)),
        }
    }
}

impl super::Arena {
    /// Concrete suffix guaranteed by a solved stack, independent of an
    /// unresolved prefix. This reports constraints, not a complete witness.
    pub(crate) fn editor_stack_suffix(
        &self,
        mut id: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<alloc::string::String>, crate::Diagnostic> {
        let mut suffix = alloc::vec::Vec::new();
        #[expect(
            tigerstyle::numeric_units,
            reason = "Owner: noble-contracts; the constant is a count of source elements, and the name states its unit at every use site through the surrounding type and call"
        )]
        // Every pushed entry follows its own successful unit work charge.
        let work_limit = attempt!(crate::offset(meter.limits.work, span));
        loop {
            attempt!(meter.charge(1, span));
            id = attempt!(self.root(id, span, meter));
            match attempt!(self.get(id, span)) {
                super::Term::Push(stack, value) => {
                    let described = attempt!(self.describe_stack(value, span, meter));
                    if suffix.len() >= work_limit {
                        return Err(crate::internal(span));
                    }
                    suffix.push(described);
                    id = stack;
                }
                super::Term::Hole(super::Sort::Stack)
                | super::Term::Empty
                | super::Term::RigidStack(_) => break,
                _ => return Err(crate::internal(span)),
            }
        }
        suffix.reverse();
        Ok(suffix)
    }

    pub(crate) fn describe_stack(
        &self,
        id: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::string::String, crate::Diagnostic> {
        rendering::describe(self, id, span, meter)
    }

    pub fn stack_value(
        &self,
        id: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<noble_kernel::types::Ty>, crate::Diagnostic> {
        match attempt!(self.materialize(id, span, meter)) {
            Material::Stack(stack, _) => Ok(stack),
            Material::Value(_, _) => Err(crate::internal(span)),
        }
    }

    pub fn instantiation(
        &self,
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<noble_kernel::words::Inst, crate::Diagnostic> {
        let mut bindings = alloc::vec::Vec::with_capacity(variables.len());
        let mut at = 0usize;
        let mut failure = None;
        while at < variables.len() {
            match self.binding(variables.get(at), span, meter) {
                Ok(binding) => {
                    bindings.push(binding);
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
            None => Ok(noble_kernel::words::Inst { bindings }),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; witness materialization allocates structural Ty values and returns owned diagnostics."
    )]
    fn binding(
        &self,
        variable: Option<&super::Variable>,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<noble_kernel::words::Binding, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match variable {
            Some(super::Variable::Effect) => Ok(noble_kernel::words::Binding::Effect(
                noble_kernel::types::EffSet::empty(),
            )),
            Some(super::Variable::EffectValue(id)) => Ok(noble_kernel::words::Binding::Effect(
                attempt!(self.effect_value(*id, span)),
            )),
            Some(super::Variable::Value(id)) => {
                match attempt!(self.materialize(*id, span, meter)) {
                    Material::Value(ty, _) => Ok(noble_kernel::words::Binding::Value(ty)),
                    Material::Stack(_, _) => Err(crate::internal(span)),
                }
            }
            Some(super::Variable::Stack(id)) => {
                match attempt!(self.materialize(*id, span, meter)) {
                    Material::Stack(stack, _) => Ok(noble_kernel::words::Binding::Stack(stack)),
                    Material::Value(_, _) => Err(crate::internal(span)),
                }
            }
            None => Err(crate::internal(span)),
        }
    }

    pub(super) fn materialize(
        &self,
        root: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Material, crate::Diagnostic> {
        let mut initial = State {
            steps: alloc::vec::Vec::with_capacity(1),
            values: alloc::vec::Vec::new(),
        };
        initial.steps.push(Step::Visit(root));
        let mut outcome = Ok(initial);
        while let Ok(mut state) = outcome {
            match state.steps.pop() {
                Some(step) => outcome = self.read_step(step, state, span, meter),
                None => {
                    outcome = Ok(state);
                    break;
                }
            }
        }
        let mut state = attempt!(outcome);
        if state.values.len() != 1 {
            return Err(crate::internal(span));
        }
        state.pop(span)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; materialization steps mutate allocating worklists and propagate non-const metering errors."
    )]
    fn read_step(
        &self,
        step: Step,
        state: State,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<State, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match step {
            Step::Visit(id) => self.visit(id, state, span, meter),
            Step::Finish(term, effects) => {
                state.finish(term, effects, &self.generic_nominals, span, meter)
            }
        }
    }

    fn visit(
        &self,
        id: u32,
        mut state: State,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<State, crate::Diagnostic> {
        let id = attempt!(self.root(id, span, meter));
        let term = attempt!(self.get(id, span));
        let material = match term {
            super::Term::Hole(_) => {
                return Err(crate::invalid(
                    span,
                    "ambiguous witness; add an explicit typed block or word binding",
                ));
            }
            super::Term::RigidValue(_) | super::Term::RigidStack(_) => {
                return Err(crate::invalid(
                    span,
                    "declaration-level universal cannot become a runtime value",
                ));
            }
            super::Term::Link(_) => return Err(crate::internal(span)),
            super::Term::Unit => Material::Value(noble_kernel::types::Ty::Unit, Extent::LEAF),
            super::Term::Bool => Material::Value(noble_kernel::types::Ty::Bool, Extent::LEAF),
            super::Term::I64 => Material::Value(noble_kernel::types::Ty::I64, Extent::LEAF),
            super::Term::Text => Material::Value(noble_kernel::types::Ty::Text, Extent::LEAF),
            super::Term::Syntax => Material::Value(noble_kernel::types::Ty::Syntax, Extent::LEAF),
            super::Term::Contract => {
                Material::Value(noble_kernel::types::Ty::Contract, Extent::LEAF)
            }
            super::Term::Evidence => {
                Material::Value(noble_kernel::types::Ty::Evidence, Extent::LEAF)
            }
            super::Term::Certified => {
                Material::Value(noble_kernel::types::Ty::Certified, Extent::LEAF)
            }
            super::Term::Resource(kind) => {
                Material::Value(noble_kernel::types::Ty::Resource(kind), Extent::LEAF)
            }
            super::Term::Nominal(id) => {
                let mut shape = None;
                let mut at = 0usize;
                while at < self.nominals.len() {
                    let (known, candidate) = &self.nominals[at];
                    if *known == id {
                        shape = Some(candidate);
                        break;
                    }
                    at += 1;
                }
                let shape = match shape {
                    Some(shape) => shape,
                    None => return Err(crate::internal(span)),
                };
                // The registered descriptor is copied into every occurrence;
                // reserve its owned nodes before cloning it.
                let descriptor = attempt!(super::finish::reserve_descriptor(shape, span, meter));
                Material::Value(
                    noble_kernel::types::Ty::Nominal(id, alloc::boxed::Box::new(shape.clone())),
                    Extent {
                        constructors: 1,
                        nodes: attempt!(super::finish::type_nodes(descriptor, 1, span)),
                    },
                )
            }
            super::Term::Empty => Material::Stack(alloc::vec::Vec::new(), Extent::EMPTY),
            super::Term::Pair(a, b)
            | super::Term::Sum(a, b)
            | super::Term::Program(a, b)
            | super::Term::Push(a, b) => {
                let effects = if self.effectful && matches!(term, super::Term::Program(_, _)) {
                    let effect = attempt!(self.program_effect(id, span, meter));
                    attempt!(self.effect_value(effect, span))
                } else {
                    noble_kernel::types::EffSet::empty()
                };
                state.steps.push(Step::Finish(term, effects));
                state.steps.push(Step::Visit(b));
                state.steps.push(Step::Visit(a));
                return Ok(state);
            }
            super::Term::GenericNominal(_, a, b) => {
                state
                    .steps
                    .push(Step::Finish(term, noble_kernel::types::EffSet::empty()));
                state.steps.push(Step::Visit(b));
                state.steps.push(Step::Visit(a));
                return Ok(state);
            }
            super::Term::List(item) => {
                state
                    .steps
                    .push(Step::Finish(term, noble_kernel::types::EffSet::empty()));
                state.steps.push(Step::Visit(item));
                return Ok(state);
            }
        };
        state.values.push(material);
        Ok(state)
    }

    /// Diagnostic rendering has a separate fixed output bound and consumes the
    /// existing work budget. A rendering refusal never replaces the type error.
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; expected and actual are ordered IDs in the same term arena, supplied directly from the rejected stack equation only to render its two sides."
    )]
    pub fn join_message(
        &self,
        expected: u32,
        actual: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::string::String, crate::Diagnostic> {
        let (expected, actual) = attempt!(self.join_stacks(expected, actual, span, meter));
        let mut message = alloc::string::String::from("stack/program join: expected ");
        message.push_str(&expected);
        message.push_str("; actual ");
        message.push_str(&actual);
        Ok(message)
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; expected and actual are ordered IDs in the same term arena from the rejected stack equation."
    )]
    pub fn join_stacks(
        &self,
        expected: u32,
        actual: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(alloc::string::String, alloc::string::String), crate::Diagnostic> {
        let expected = attempt!(rendering::describe(self, expected, span, meter));
        let actual = attempt!(rendering::describe(self, actual, span, meter));
        Ok((expected, actual))
    }

    /// Where a rejected stack join's two sides first differ, walking both from the top: the number
    /// of positions on which they render the same type, then each side's rendering at the first
    /// position where they do not. A side with no element there contributes its remaining stack
    /// instead. `None` when every position and both remaining stacks render alike, so the mismatch
    /// is not positional. The walk is charged to the meter and stops at the stack height cap.
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; expected and actual are ordered IDs in the same term arena from the rejected stack equation."
    )]
    pub fn stack_difference(
        &self,
        expected: u32,
        actual: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Option<(u32, alloc::string::String, alloc::string::String)>, crate::Diagnostic>
    {
        let mut expected = attempt!(self.root(expected, span, meter));
        let mut actual = attempt!(self.root(actual, span, meter));
        let mut shared = 0_u32;
        while shared < super::STACK_CAP {
            attempt!(meter.charge(1, span));
            let expected_top =
                if let super::Term::Push(rest, top) = attempt!(self.get(expected, span)) {
                    Some((rest, top))
                } else {
                    None
                };
            let actual_top = if let super::Term::Push(rest, top) = attempt!(self.get(actual, span))
            {
                Some((rest, top))
            } else {
                None
            };
            let expected_text = attempt!(rendering::describe(
                self,
                expected_top.map_or(expected, |(_, top)| top),
                span,
                meter
            ));
            let actual_text = attempt!(rendering::describe(
                self,
                actual_top.map_or(actual, |(_, top)| top),
                span,
                meter
            ));
            if expected_text != actual_text {
                return Ok(Some((shared, expected_text, actual_text)));
            }
            let (Some((expected_rest, _)), Some((actual_rest, _))) = (expected_top, actual_top)
            else {
                return Ok(None);
            };
            expected = attempt!(self.root(expected_rest, span, meter));
            actual = attempt!(self.root(actual_rest, span, meter));
            shared = shared.saturating_add(1);
        }
        Ok(None)
    }
}
