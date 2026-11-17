//! Finite concrete host-effect constraints shared by source type inference.
//! Solving removes impossible bits from the greatest positive-union solution;
//! it never hides a concrete host request.

#[cfg(test)]
mod oracle;
mod solve;

#[derive(Clone, Copy)]
pub(super) enum Effect {
    Hole,
    Constant(u64),
    Union(u32, u32),
    Rigid(u32),
}

impl super::Arena {
    /// Preserve unknown editor effects even when the current finite host
    /// universe is empty; a hole never certifies purity.
    pub(crate) fn editor_effect(
        &self,
        id: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(alloc::vec::Vec<u32>, bool), crate::Diagnostic> {
        let mut pending = alloc::vec![id];
        let mut bits = 0u64;
        let mut is_unknown = false;
        while let Some(id) = pending.pop() {
            attempt!(meter.charge(1, span));
            match self.effects.get(attempt!(crate::offset(id, span))) {
                Some(Effect::Constant(value)) => bits |= value,
                Some(Effect::Hole | Effect::Rigid(_)) => is_unknown = true,
                Some(Effect::Union(left, right)) => {
                    pending.push(*left);
                    pending.push(*right);
                }
                None => return Err(crate::internal(span)),
            }
        }
        // The finite host-effect universe has one output per set bit.
        let count = attempt!(usize::try_from(bits.count_ones()).map_err(|_| crate::internal(span)));
        let mut known = alloc::vec::Vec::with_capacity(count);
        for bit in 0..64 {
            if bits & (1u64 << bit) != 0 {
                known.push(bit);
            }
        }
        Ok((known, is_unknown))
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; node charging and appending effect/bound entries allocate or return owned diagnostics."
    )]
    fn add_effect(
        &mut self,
        effect: Effect,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        attempt!(meter.node(span));
        let id = attempt!(crate::index(self.effects.len(), span));
        #[expect(
            tigerstyle::fragile_exhaustive_enum_match,
            reason = "Owner: noble-maintainers; every closed effect constructor needs its exact initial bound; a new constructor must fail compilation here."
        )]
        let bound = match effect {
            Effect::Constant(bits) => bits,
            Effect::Hole => self.effect_universe,
            Effect::Union(_, _) => self.effect_universe,
            Effect::Rigid(_) => self.effect_universe,
        };
        self.effects.push(effect);
        self.effect_bounds.push(bound);
        Ok(id)
    }

    pub fn effect_hole(
        &mut self,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.add_effect(Effect::Hole, span, meter)
    }

    pub fn rigid_effect(
        &mut self,
        identity: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        if identity >= 64 {
            return Err(crate::invalid(span, "rigid effect binder count exceeded"));
        }
        self.add_effect(Effect::Rigid(identity), span, meter)
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape."
    )]
    pub(crate) fn is_rigid_effect(&self, id: u32, identity: u32) -> bool {
        usize::try_from(id)
            .ok()
            .and_then(|index| self.effects.get(index))
            .is_some_and(|effect| matches!(effect, Effect::Rigid(index) if *index == identity))
    }

    pub fn effect_empty(
        &mut self,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.add_effect(Effect::Constant(0), span, meter)
    }

    pub fn effect_constant(
        &mut self,
        effects: &noble_kernel::types::EffSet,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.effect_constant_with_type_effects(effects, span, meter, false)
    }

    pub(crate) fn program_type_effect_constant(
        &mut self,
        effects: &noble_kernel::types::EffSet,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.effect_constant_with_type_effects(effects, span, meter, true)
    }

    fn effect_constant_with_type_effects(
        &mut self,
        effects: &noble_kernel::types::EffSet,
        span: crate::Span,
        meter: &mut crate::Meter,
        program_type: bool,
    ) -> Result<u32, crate::Diagnostic> {
        let mut bits = 0;
        let effects = effects.as_slice();
        let mut at = 0usize;
        let mut failure = None;
        while let Some(effect) = effects.get(at).copied() {
            if let Err(problem) = meter.charge(1, span) {
                failure = Some(problem);
                break;
            }
            match effect_bit(effect) {
                Some(bit)
                    if bit
                        & (self.effect_universe
                            | if program_type {
                                self.program_type_effects
                            } else {
                                0
                            })
                        != 0 =>
                {
                    bits |= bit
                }
                Some(_) | None => {
                    failure = Some(crate::Diagnostic::new(
                        crate::DiagnosticKind::Unsupported,
                        span,
                        "unknown source host effect",
                    ));
                    break;
                }
            }
            at += 1;
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        self.add_effect(Effect::Constant(bits), span, meter)
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-maintainers; both operands are effect IDs from this arena and union is symmetric, so swapping them cannot change the inferred effect set."
    )]
    pub fn effect_union(
        &mut self,
        left: u32,
        right: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        if left == right {
            Ok(left)
        } else {
            self.add_effect(Effect::Union(left, right), span, meter)
        }
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional coordinates at each checked call site; a newtype would not change the call shape."
    )]
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-contracts; the union mutates the solver tables and meter through non-const method calls."
    )]
    fn program_type_effect_union(
        &mut self,
        left: u32,
        right: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        if left == right {
            return Ok(left);
        }
        let upper =
            attempt!(self.effect_bound(left, span)) | attempt!(self.effect_bound(right, span));
        let id = attempt!(self.add_effect(Effect::Union(left, right), span, meter));
        match self
            .effect_bounds
            .get_mut(attempt!(crate::offset(id, span)))
        {
            Some(bound) => *bound = upper,
            None => return Err(crate::internal(span)),
        }
        Ok(id)
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
    )]
    /// An annotated source interface cannot hide an effect of its body.
    pub fn equate_effects(
        &mut self,
        body: u32,
        declared: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.node(span));
        self.effect_equations.push((body, declared));
        Ok(())
    }

    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
    )]
    pub fn require_rigid_effect(
        &mut self,
        body: u32,
        declared: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.node(span));
        self.rigid_effect_goals.push((body, declared));
        Ok(())
    }

    pub fn effect_pattern(
        &mut self,
        slots: &[noble_kernel::shapes::EffectSlot],
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.effect_pattern_with_type_effects(slots, variables, span, meter, false)
    }

    pub(crate) fn program_type_effect_pattern(
        &mut self,
        slots: &[noble_kernel::shapes::EffectSlot],
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        self.effect_pattern_with_type_effects(slots, variables, span, meter, true)
    }

    fn effect_pattern_with_type_effects(
        &mut self,
        slots: &[noble_kernel::shapes::EffectSlot],
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
        program_type: bool,
    ) -> Result<u32, crate::Diagnostic> {
        let mut effect = attempt!(self.effect_empty(span, meter));
        let mut at = 0usize;
        let mut failure = None;
        while at < slots.len() {
            let result = match self.effect_slot(slots.get(at), variables, span, meter, program_type)
            {
                Ok(next) if program_type => {
                    self.program_type_effect_union(effect, next, span, meter)
                }
                Ok(next) => self.effect_union(effect, next, span, meter),
                Err(problem) => Err(problem),
            };
            match result {
                Ok(next) => effect = next,
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
            at += 1;
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(effect),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; each pattern slot charges work and either allocates a concrete effect or checks a quantified effect witness, returning owned diagnostics on invalid state."
    )]
    fn effect_slot(
        &mut self,
        slot: Option<&noble_kernel::shapes::EffectSlot>,
        variables: &[super::Variable],
        span: crate::Span,
        meter: &mut crate::Meter,
        program_type: bool,
    ) -> Result<u32, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match slot {
            Some(noble_kernel::shapes::EffectSlot::Effect(id)) => {
                let bit = match effect_bit(*id) {
                    Some(bit)
                        if bit
                            & (self.effect_universe
                                | if program_type {
                                    self.program_type_effects
                                } else {
                                    0
                                })
                            != 0 =>
                    {
                        bit
                    }
                    Some(_) | None => {
                        return Err(crate::invalid(span, "unknown source host effect"))
                    }
                };
                self.add_effect(Effect::Constant(bit), span, meter)
            }
            Some(noble_kernel::shapes::EffectSlot::Var(id)) => {
                match attempt!(super::variable_at(variables, id.0, span)) {
                    super::Variable::EffectValue(id) => Ok(id),
                    super::Variable::Value(_)
                    | super::Variable::Stack(_)
                    | super::Variable::Effect => Err(crate::internal(span)),
                }
            }
            None => Err(crate::internal(span)),
        }
    }

    pub fn program(
        &mut self,
        interface: super::Program,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        attempt!(meter.node(span));
        let id = attempt!(self.add(
            super::Term::Program(interface.input, interface.output),
            span,
            meter
        ));
        self.program_effects.push((id, interface.effect));
        Ok(id)
    }

    pub(crate) fn program_effect(
        &self,
        program: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        let mut at = 0usize;
        let mut found = None;
        let mut failure = None;
        while let Some((id, effect)) = self.program_effects.get(at).copied() {
            if let Err(problem) = meter.charge(1, span) {
                failure = Some(problem);
                break;
            }
            if id == program {
                found = Some(effect);
                break;
            }
            at += 1;
        }
        if let Some(problem) = failure {
            return Err(problem);
        }
        match found {
            Some(effect) => Ok(effect),
            None => Err(crate::internal(span)),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; checked arena offsets and missing effect IDs return allocating diagnostics."
    )]
    fn effect_bound(&self, id: u32, span: crate::Span) -> Result<u64, crate::Diagnostic> {
        match self.effect_bounds.get(attempt!(crate::offset(id, span))) {
            Some(bits) => Ok(*bits),
            None => Err(crate::internal(span)),
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; checked effect-table access can return owned invalid-index diagnostics."
    )]
    fn narrow_effect(
        &mut self,
        id: u32,
        bits: u64,
        span: crate::Span,
    ) -> Result<bool, crate::Diagnostic> {
        match self
            .effect_bounds
            .get_mut(attempt!(crate::offset(id, span)))
        {
            Some(old) => {
                let new = *old & bits;
                let has_changed = new != *old;
                *old = new;
                Ok(has_changed)
            }
            None => Err(crate::internal(span)),
        }
    }

    pub fn solve_effects(
        &mut self,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(solve::rigid(self, span, meter));
        solve::run(self, span, meter)
    }

    pub fn effect_value(
        &self,
        id: u32,
        span: crate::Span,
    ) -> Result<noble_kernel::types::EffSet, crate::Diagnostic> {
        solve::value(self, id, span)
    }

    /// Close first-class interfaces only after open generic validity is checked.
    pub fn close(
        &mut self,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        solve::close(self, span, meter)
    }
}

const fn effect_bit(effect: noble_kernel::types::EffId) -> Option<u64> {
    1u64.checked_shl(effect.0)
}
