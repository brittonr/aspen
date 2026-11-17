impl super::Arena {
    #[expect(
        clippy::vec_init_then_push,
        reason = "Owner: noble-maintainers; explicit owned push avoids the pinned Aeneas erased-region failure in vec! array conversion; reassess when the translator supports it."
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; variable and target are named IDs in one arena. This private directed binding is called only after root/sort checks in unify_step."
    )]
    fn bind(
        &mut self,
        variable: u32,
        target: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let mut initial = alloc::vec::Vec::with_capacity(1);
        initial.push(target);
        let mut outcome = Ok(initial);
        while let Ok(mut pending) = outcome {
            match pending.pop() {
                Some(id) => outcome = self.occurs_step(variable, id, pending, span, meter),
                None => {
                    outcome = Ok(pending);
                    break;
                }
            }
        }
        attempt!(outcome);
        match self.terms.get_mut(attempt!(crate::offset(variable, span))) {
            Some(term) => {
                *term = super::Term::Link(target);
                Ok(())
            }
            None => Err(crate::internal(span)),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; variable and visited ID inhabit the same arena for the occurs check. Traversal grows a Vec and reports recursive equations through diagnostics."
    )]
    fn occurs_step(
        &self,
        variable: u32,
        id: u32,
        mut pending: alloc::vec::Vec<u32>,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<u32>, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        let id = attempt!(self.root(id, span, meter));
        if id == variable {
            return Err(crate::invalid(
                span,
                "recursive type or stack equation is not supported",
            ));
        }
        match attempt!(self.get(id, span)) {
            super::Term::Pair(a, b)
            | super::Term::Sum(a, b)
            | super::Term::Program(a, b)
            | super::Term::Push(a, b) => {
                pending.push(a);
                pending.push(b);
            }
            super::Term::GenericNominal(_, a, b) => {
                pending.push(a);
                pending.push(b);
            }
            super::Term::List(item) => pending.push(item),
            super::Term::Hole(_)
            | super::Term::RigidValue(_)
            | super::Term::RigidStack(_)
            | super::Term::Link(_)
            | super::Term::Unit
            | super::Term::Bool
            | super::Term::I64
            | super::Term::Text
            | super::Term::Syntax
            | super::Term::Contract
            | super::Term::Evidence
            | super::Term::Certified
            | super::Term::Resource(_)
            | super::Term::Nominal(_)
            | super::Term::Empty => {}
        }
        Ok(pending)
    }

    #[expect(
        clippy::vec_init_then_push,
        reason = "Owner: noble-maintainers; explicit owned push avoids the pinned Aeneas erased-region failure in vec! array conversion; reassess when the translator supports it."
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; left and right are same-domain arena IDs in a symmetric type equation; swapping them does not alter type acceptance."
    )]
    pub fn unify(
        &mut self,
        left: u32,
        right: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let mut initial = alloc::vec::Vec::with_capacity(1);
        initial.push((left, right));
        let mut outcome = Ok(initial);
        while let Ok(mut pending) = outcome {
            match pending.pop() {
                Some((left, right)) => {
                    outcome = self.unify_step(left, right, pending, span, meter);
                }
                None => {
                    outcome = Ok(pending);
                    break;
                }
            }
        }
        attempt!(outcome);
        Ok(())
    }

    #[expect(
        tigerstyle::missing_const_fn,
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; left and right are symmetric equation operands. This step grows a worklist, binds holes, and constructs typed mismatch errors."
    )]
    fn unify_step(
        &mut self,
        left: u32,
        right: u32,
        mut pending: alloc::vec::Vec<(u32, u32)>,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<alloc::vec::Vec<(u32, u32)>, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        let left = attempt!(self.root(left, span, meter));
        let right = attempt!(self.root(right, span, meter));
        if left == right {
            return Ok(pending);
        }
        let a = attempt!(self.get(left, span));
        let b = attempt!(self.get(right, span));
        if self.effectful
            && matches!(a, super::Term::Program(_, _))
            && matches!(b, super::Term::Program(_, _))
        {
            let a_effect = attempt!(self.program_effect(left, span, meter));
            let b_effect = attempt!(self.program_effect(right, span, meter));
            attempt!(meter.node(span));
            self.effect_equations.push((a_effect, b_effect));
        }
        if sort(a) != sort(b) {
            return Err(crate::invalid(
                span,
                "value and stack witness kinds do not match",
            ));
        }
        match (a, b) {
            (super::Term::Hole(_), _) => attempt!(self.bind(left, right, span, meter)),
            (_, super::Term::Hole(_)) => attempt!(self.bind(right, left, span, meter)),
            (super::Term::Unit, super::Term::Unit)
            | (super::Term::Bool, super::Term::Bool)
            | (super::Term::I64, super::Term::I64)
            | (super::Term::Text, super::Term::Text)
            | (super::Term::Syntax, super::Term::Syntax)
            | (super::Term::Contract, super::Term::Contract)
            | (super::Term::Evidence, super::Term::Evidence)
            | (super::Term::Certified, super::Term::Certified)
            | (super::Term::Empty, super::Term::Empty) => {}
            (super::Term::Resource(a), super::Term::Resource(b)) if a == b => {}
            (super::Term::Nominal(a), super::Term::Nominal(b)) if a == b => {}
            (super::Term::RigidValue(a), super::Term::RigidValue(b))
            | (super::Term::RigidStack(a), super::Term::RigidStack(b))
                if a == b => {}
            (super::Term::GenericNominal(id_a, a, b), super::Term::GenericNominal(id_c, c, d))
                if id_a == id_c =>
            {
                pending.push((a, c));
                pending.push((b, d));
            }
            (super::Term::Pair(a, b), super::Term::Pair(c, d))
            | (super::Term::Sum(a, b), super::Term::Sum(c, d))
            | (super::Term::Program(a, b), super::Term::Program(c, d))
            | (super::Term::Push(a, b), super::Term::Push(c, d)) => {
                pending.push((a, c));
                pending.push((b, d));
            }
            (super::Term::List(a), super::Term::List(b)) => pending.push((a, b)),
            (
                super::Term::Link(_)
                | super::Term::RigidValue(_)
                | super::Term::RigidStack(_)
                | super::Term::Unit
                | super::Term::Bool
                | super::Term::I64
                | super::Term::Text
                | super::Term::Syntax
                | super::Term::Contract
                | super::Term::Evidence
                | super::Term::Certified
                | super::Term::Resource(_)
                | super::Term::Nominal(_)
                | super::Term::GenericNominal(_, _, _)
                | super::Term::Pair(_, _)
                | super::Term::Sum(_, _)
                | super::Term::List(_)
                | super::Term::Program(_, _)
                | super::Term::Empty
                | super::Term::Push(_, _),
                _,
            ) => {
                return Err(crate::invalid(
                    span,
                    "program type or stack witness cannot be resolved",
                ))
            }
        }
        Ok(pending)
    }
}

const fn sort(term: super::Term) -> super::Sort {
    match term {
        super::Term::Hole(sort) => sort,
        super::Term::RigidStack(_) | super::Term::Empty | super::Term::Push(_, _) => {
            super::Sort::Stack
        }
        super::Term::Link(_)
        | super::Term::Unit
        | super::Term::Bool
        | super::Term::I64
        | super::Term::Text
        | super::Term::Syntax
        | super::Term::Contract
        | super::Term::Evidence
        | super::Term::Certified
        | super::Term::Resource(_)
        | super::Term::Nominal(_)
        | super::Term::RigidValue(_)
        | super::Term::GenericNominal(_, _, _)
        | super::Term::Pair(_, _)
        | super::Term::Sum(_, _)
        | super::Term::List(_)
        | super::Term::Program(_, _) => super::Sort::Value,
    }
}
