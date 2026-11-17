/// Check open interfaces structurally, without picking values for quantified
/// holes. Primitive eligibility is replayed at each fresh instantiation, then
/// checked independently by the kernel for every executable specialization.
pub(super) fn open(
    state: &super::State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let mut at = 0usize;
    let mut failure = None;
    while at < state.bodies.len() {
        if let Err(problem) = body_terms(&state.arena, &state.bodies[at], meter) {
            failure = Some(problem);
            break;
        }
        at = at.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

fn body_terms(
    arena: &crate::inference::Arena,
    body: &super::Body,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(term(arena, body.input, body.span, meter));
    attempt!(term(arena, body.output, body.span, meter));
    let mut at = 0usize;
    let mut failure = None;
    while at < body.nodes.len() {
        if let Err(problem) = variables(arena, &body.nodes[at], meter) {
            failure = Some(problem);
            break;
        }
        at = at.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

fn variables(
    arena: &crate::inference::Arena,
    draft: &super::Draft,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let mut at = 0usize;
    let mut failure = None;
    while at < draft.variables.len() {
        if let Err(problem) = variable_term(arena, draft.variables[at], draft.span, meter) {
            failure = Some(problem);
            break;
        }
        at = at.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; each witness consumes work and structural traversal can allocate or reject with an owned diagnostic."
)]
fn variable_term(
    arena: &crate::inference::Arena,
    variable: crate::inference::Variable,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; each closed witness kind must be structurally checked or explicitly classified as an effect witness; new kinds require this validation rule to be reviewed."
    )]
    match variable {
        crate::inference::Variable::Value(id) | crate::inference::Variable::Stack(id) => {
            term(arena, id, span, meter)
        }
        crate::inference::Variable::Effect | crate::inference::Variable::EffectValue(_) => Ok(()),
    }
}

struct Visit {
    id: u32,
    depth: u32,
    height: u32,
    group: Option<usize>,
}

struct Traversal {
    pending: alloc::vec::Vec<Visit>,
    sizes: alloc::vec::Vec<u32>,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; every explicit continuation checks work, depth, term IDs, stack height, and constructor-group size through diagnostics, retaining the first refusal without panicking."
)]
fn term(
    arena: &crate::inference::Arena,
    root: u32,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let mut walk = Traversal {
        pending: alloc::vec::Vec::with_capacity(1),
        sizes: alloc::vec::Vec::new(),
    };
    walk.pending.push(Visit {
        id: root,
        depth: 0,
        height: 0,
        group: None,
    });
    let mut failure = None;
    while let Some(visit) = walk.pending.pop() {
        if let Err(problem) = walk.visit(arena, visit, span, meter) {
            failure = Some(problem);
            break;
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

impl Traversal {
    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; a charged visit resolves arena links, checks bounded constructors, and grows owned continuations; these operations can allocate diagnostics and worklists."
    )]
    fn visit(
        &mut self,
        arena: &crate::inference::Arena,
        visit: Visit,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        attempt!(meter.depth(visit.depth, span));
        let id = attempt!(arena.root(visit.id, span, meter));
        let term = attempt!(arena.get(id, span));
        let group = attempt!(self.value_group(term, visit.group, span, meter));
        // A charged visit schedules at most two children, in the original LIFO order.
        match term {
            crate::inference::Term::Push(stack, value) => {
                if visit.height >= crate::inference::STACK_CAP {
                    return Err(crate::source::exhausted(
                        span,
                        "open stack height limit exceeded",
                    ));
                }
                self.pending.reserve(2);
                self.pending.push(Visit {
                    id: stack,
                    depth: visit.depth,
                    height: visit.height.saturating_add(1),
                    group,
                });
                self.pending.push(Visit {
                    id: value,
                    depth: visit.depth,
                    height: 0,
                    group,
                });
            }
            crate::inference::Term::Pair(a, b)
            | crate::inference::Term::Sum(a, b)
            | crate::inference::Term::Program(a, b) => {
                self.pending.reserve(2);
                self.pending.push(Visit {
                    id: a,
                    depth: visit.depth.saturating_add(1),
                    height: 0,
                    group,
                });
                self.pending.push(Visit {
                    id: b,
                    depth: visit.depth.saturating_add(1),
                    height: 0,
                    group,
                });
            }
            crate::inference::Term::List(item) => {
                self.pending.reserve(1);
                self.pending.push(Visit {
                    id: item,
                    depth: visit.depth.saturating_add(1),
                    height: 0,
                    group,
                });
            }
            crate::inference::Term::GenericNominal(_, a, b) => {
                self.pending.push(Visit {
                    id: a,
                    depth: visit.depth.saturating_add(1),
                    height: 0,
                    group,
                });
                self.pending.push(Visit {
                    id: b,
                    depth: visit.depth.saturating_add(1),
                    height: 0,
                    group,
                });
            }
            crate::inference::Term::Hole(_)
            | crate::inference::Term::RigidValue(_)
            | crate::inference::Term::RigidStack(_)
            | crate::inference::Term::Unit
            | crate::inference::Term::Bool
            | crate::inference::Term::I64
            | crate::inference::Term::Text
            | crate::inference::Term::Syntax
            | crate::inference::Term::Contract
            | crate::inference::Term::Evidence
            | crate::inference::Term::Certified
            | crate::inference::Term::Resource(_)
            | crate::inference::Term::Nominal(_)
            | crate::inference::Term::Empty => {}
            crate::inference::Term::Link(_) => return Err(crate::internal(span)),
        }
        Ok(())
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; each new size group consumes a node before reserving and appending an owned counter, and invalid groups or oversized types return allocating diagnostics."
    )]
    fn value_group(
        &mut self,
        term: crate::inference::Term,
        group: Option<usize>,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<Option<usize>, crate::Diagnostic> {
        if matches!(
            term,
            crate::inference::Term::Push(_, _)
                | crate::inference::Term::Empty
                | crate::inference::Term::RigidStack(_)
                | crate::inference::Term::Hole(crate::inference::Sort::Stack)
        ) {
            return Ok(group);
        }
        let group = match group {
            Some(group) => group,
            None => {
                attempt!(meter.node(span));
                let group = self.sizes.len();
                self.sizes.reserve(1);
                self.sizes.push(0u32);
                group
            }
        };
        match self.sizes.get_mut(group) {
            Some(size) => {
                *size = size.saturating_add(1);
                if *size > crate::syntax::TYPE_CAP {
                    return Err(crate::source::exhausted(
                        span,
                        "open value type exceeds the 256-constructor limit",
                    ));
                }
            }
            None => return Err(crate::internal(span)),
        }
        Ok(Some(group))
    }
}
