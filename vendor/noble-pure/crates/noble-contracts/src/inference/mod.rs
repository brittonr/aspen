#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; type/effect inference mutates only preparation-owned, metered term and constraint arenas, traversal scratch, and bounded diagnostic buffers; borrowed source types and published session state remain unchanged."
)]

mod build;
mod construct;
mod effects;
mod finish;
mod materialize;
mod unify;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sort {
    Value,
    Stack,
}

#[derive(Clone, Copy)]
#[charon::variants_suffix("Term")]
#[octet::sealed_enum]
pub(crate) enum Term {
    Hole(Sort),
    RigidValue(u32),
    RigidStack(u32),
    Link(u32),
    Unit,
    Bool,
    I64,
    Text,
    Syntax,
    Contract,
    Evidence,
    Certified,
    Resource(noble_kernel::types::ResourceKind),
    Nominal(noble_kernel::types::NominalTypeId),
    GenericNominal(noble_kernel::types::NominalTypeId, u32, u32),
    Pair(u32, u32),
    Sum(u32, u32),
    List(u32),
    Program(u32, u32),
    Empty,
    Push(u32, u32),
}

#[derive(Clone, Copy)]
pub(crate) enum Variable {
    Value(u32),
    Stack(u32),
    Effect,
    EffectValue(u32),
}

pub(crate) struct Program {
    pub input: u32,
    pub output: u32,
    pub effect: u32,
}

pub(crate) struct Arena {
    terms: alloc::vec::Vec<Term>,
    effectful: bool,
    effect_universe: u64,
    program_type_effects: u64,
    resources: bool,
    pub(crate) nominals: alloc::vec::Vec<(
        noble_kernel::types::NominalTypeId,
        noble_kernel::types::NominalShape,
    )>,
    pub(crate) generic_nominals: alloc::vec::Vec<noble_kernel::contracts::GenericVariantDecl>,
    rigid_count: u32,
    effects: alloc::vec::Vec<effects::Effect>,
    effect_bounds: alloc::vec::Vec<u64>,
    effect_equations: alloc::vec::Vec<(u32, u32)>,
    rigid_effect_goals: alloc::vec::Vec<(u32, u32)>,
    program_effects: alloc::vec::Vec<(u32, u32)>,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            terms: alloc::vec::Vec::new(),
            effectful: false,
            effect_universe: 0,
            program_type_effects: 0,
            resources: false,
            nominals: alloc::vec::Vec::new(),
            generic_nominals: alloc::vec::Vec::new(),
            rigid_count: 0,
            effects: alloc::vec::Vec::new(),
            effect_bounds: alloc::vec::Vec::new(),
            effect_equations: alloc::vec::Vec::new(),
            rigid_effect_goals: alloc::vec::Vec::new(),
            program_effects: alloc::vec::Vec::new(),
        }
    }

    pub fn source(effect_universe: u64, resources: bool) -> Self {
        let mut arena = Self::new();
        arena.effectful = true;
        arena.effect_universe = effect_universe;
        arena.resources = resources;
        arena
    }

    pub(crate) fn allow_program_type_effects(&mut self, effects: u64) {
        self.program_type_effects = effects;
    }

    pub fn add(
        &mut self,
        term: Term,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        attempt!(meter.node(span));
        let id = attempt!(crate::index(self.terms.len(), span));
        self.terms.push(term);
        Ok(id)
    }

    pub(crate) fn get(&self, id: u32, span: crate::Span) -> Result<Term, crate::Diagnostic> {
        match self.terms.get(attempt!(crate::offset(id, span))) {
            Some(term) => Ok(*term),
            None => Err(crate::internal(span)),
        }
    }

    pub(crate) fn root(
        &self,
        mut id: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<u32, crate::Diagnostic> {
        let mut remaining = self.terms.len().saturating_add(1);
        let mut is_found = false;
        let mut failure = None;
        while remaining > 0 {
            match self.root_step(id, span, meter) {
                Ok(Some(next)) => id = next,
                Ok(None) => {
                    is_found = true;
                    break;
                }
                Err(problem) => {
                    failure = Some(problem);
                    break;
                }
            }
            remaining -= 1;
        }
        match failure {
            Some(problem) => Err(problem),
            None => {
                if is_found {
                    Ok(id)
                } else {
                    Err(crate::internal(span))
                }
            }
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; metered link lookup can construct exhaustion or invalid-index diagnostics."
    )]
    fn root_step(
        &self,
        id: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Option<u32>, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match attempt!(self.get(id, span)) {
            Term::Link(next) => Ok(Some(next)),
            Term::Hole(_)
            | Term::RigidValue(_)
            | Term::RigidStack(_)
            | Term::Unit
            | Term::Bool
            | Term::I64
            | Term::Text
            | Term::Syntax
            | Term::Contract
            | Term::Evidence
            | Term::Certified
            | Term::Resource(_)
            | Term::Nominal(_)
            | Term::GenericNominal(_, _, _)
            | Term::Pair(_, _)
            | Term::Sum(_, _)
            | Term::List(_)
            | Term::Program(_, _)
            | Term::Empty
            | Term::Push(_, _) => Ok(None),
        }
    }
}

pub(crate) const STACK_CAP: u32 = 256;

/// Owned `Ty` nodes in one materialized value, counted separately from the
/// logical `TYPE_CAP`. A generic instance owns both its ordered arguments and
/// a descriptor copy of its payloads, so this physical count can double per
/// nesting level while the logical size grows by a constant. Every inferred
/// value is refused above this bound before its descriptor copies are cloned,
/// which also bounds the later clone, equality and size walks over it.
pub(crate) const TYPE_NODE_CAP: u32 = 4096;

pub(crate) fn variable_at(
    variables: &[Variable],
    id: u32,
    span: crate::Span,
) -> Result<Variable, crate::Diagnostic> {
    match variables.get(attempt!(crate::offset(id, span))) {
        Some(variable) => Ok(*variable),
        None => Err(crate::invalid(span, "unknown witness variable")),
    }
}

pub(crate) fn pure_effects(
    effects: &[noble_kernel::shapes::EffectSlot],
    variables: &[Variable],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let mut at = 0usize;
    let mut failure = None;
    while at < effects.len() {
        match pure_effect(effects.get(at), variables, span, meter) {
            Ok(()) => at += 1,
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; effect lookup and rejection invoke metering and owned diagnostic constructors."
)]
fn pure_effect(
    effect: Option<&noble_kernel::shapes::EffectSlot>,
    variables: &[Variable],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    match effect {
        Some(noble_kernel::shapes::EffectSlot::Var(variable)) => {
            match attempt!(variable_at(variables, variable.0, span)) {
                Variable::Effect => Ok(()),
                Variable::Value(_) | Variable::Stack(_) | Variable::EffectValue(_) => {
                    Err(crate::internal(span))
                }
            }
        }
        Some(noble_kernel::shapes::EffectSlot::Effect(_)) => Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            span,
            "host effects are outside the pure contract fragment",
        )),
        None => Err(crate::internal(span)),
    }
}
