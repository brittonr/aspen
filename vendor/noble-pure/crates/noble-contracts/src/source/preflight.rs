#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; input admission mutates only the preparation-owned meter and fresh type-path/continuation counters bounded by constructor, stack, depth, node, and work limits; supplied types remain borrowed unchanged and no host program executes."
)]

pub(super) mod paths;

pub(super) fn check(
    inputs: &[noble_kernel::types::Ty],
    session: &super::Session,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    if inputs.len() > attempt!(crate::offset(crate::inference::STACK_CAP, span)) {
        return Err(super::exhausted(span, "source input stack limit exceeded"));
    }
    let mut at = 0usize;
    let mut failure = None;
    while at < inputs.len() {
        if let Err(problem) = value(&inputs[at], session, span, meter) {
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

#[derive(Clone, Copy)]
pub(super) enum PathStep {
    Root,
    Left,
    Right,
    Item,
    GenericArgument(usize),
    Input(usize),
    Output(usize),
}

struct Visit {
    step: PathStep,
    depth: u32,
    borrow_allowed: bool,
}

struct Traversal {
    pending: alloc::vec::Vec<Visit>,
    path: alloc::vec::Vec<PathStep>,
    type_nodes: usize,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the owned continuation traversal checks constructor, work and depth bounds before following each borrowed input node, retaining refusals rather than asserting on caller types."
)]
fn value(
    input: &noble_kernel::types::Ty,
    session: &super::Session,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let mut walk = Traversal {
        pending: alloc::vec::Vec::with_capacity(1),
        path: alloc::vec::Vec::new(),
        type_nodes: 1,
    };
    attempt!(meter.node(span));
    walk.pending.push(Visit {
        step: PathStep::Root,
        depth: 0,
        borrow_allowed: true,
    });
    let mut failure = None;
    while let Some(entry) = walk.pending.pop() {
        if let Err(problem) = walk.visit(entry, input, session, span, meter) {
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
        reason = "Owner: noble-maintainers; charged input traversal checks runtime limits and reserves owned continuation capacity, with owned diagnostics on rejection."
    )]
    fn visit(
        &mut self,
        entry: Visit,
        root: &noble_kernel::types::Ty,
        session: &super::Session,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let Visit {
            step,
            depth,
            borrow_allowed,
        } = entry;
        attempt!(meter.charge(1, span));
        attempt!(meter.depth(depth, span));
        // LIFO siblings share the prefix above their depth. Both the path and
        // pending selectors are bounded by the admitted constructor count.
        self.path.truncate(attempt!(crate::offset(depth, span)));
        self.path.push(step);
        let ty = attempt!(paths::locate(root, &self.path, span));
        let children = match ty {
            noble_kernel::types::Ty::Pair(_, _) | noble_kernel::types::Ty::Sum(_, _) => 2,
            noble_kernel::types::Ty::List(_) => 1,
            noble_kernel::types::Ty::Nominal(_, _) => 0,
            noble_kernel::types::Ty::GenericNominal(_, _, _) => 0,
            noble_kernel::types::Ty::Program(input, output, _)
            | noble_kernel::types::Ty::LiveRef(input, output, _) => {
                input.len().saturating_add(output.len())
            }
            noble_kernel::types::Ty::Unit
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Text
            | noble_kernel::types::Ty::Syntax
            | noble_kernel::types::Ty::Contract
            | noble_kernel::types::Ty::Evidence
            | noble_kernel::types::Ty::Certified
            | noble_kernel::types::Ty::Resource(_) => 0,
        };
        self.type_nodes = self.type_nodes.saturating_add(children);
        if self.type_nodes > attempt!(crate::offset(crate::syntax::TYPE_CAP, span)) {
            return Err(super::exhausted(
                span,
                "source input exceeds the 256-constructor type limit",
            ));
        }
        self.pending.reserve(children);
        self.expand(ty, depth, borrow_allowed, session, span, meter)
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; structural expansion charges the mutable runtime meter, grows the owned work vector and constructs owned type diagnostics."
    )]
    #[expect(
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
    )]
    fn expand(
        &mut self,
        ty: &noble_kernel::types::Ty,
        depth: u32,
        borrow_allowed: bool,
        session: &super::Session,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        let next_depth = depth.saturating_add(1);
        match ty {
            noble_kernel::types::Ty::Resource(kind) => {
                if session.live_slots.is_some() {
                    return Err(crate::invalid(
                        span,
                        "live resource requires a registered versioned nominal schema",
                    ));
                }
                let is_known = if let Some(context) = &session.declared {
                    context.environment.resource_kinds.contains(kind)
                } else {
                    match &session.bindings {
                        Some(bindings) => {
                            depth == 0 && bindings.environment.resource_kinds.contains(kind)
                        }
                        None => false,
                    }
                };
                if is_known {
                    Ok(())
                } else {
                    Err(crate::invalid(
                        span,
                        "resource input is not a declared top-level component owner",
                    ))
                }
            }
            noble_kernel::types::Ty::Nominal(id, shape) => {
                let environment = session
                    .declared
                    .as_ref()
                    .map(|context| &context.environment)
                    .or_else(|| {
                        session
                            .live_slots
                            .as_ref()
                            .map(|profile| &profile.environment)
                    });
                if environment
                    .is_some_and(|env| env.nominal(*id).is_some_and(|decl| decl.shape == **shape))
                {
                    Ok(())
                } else {
                    Err(crate::invalid(
                        span,
                        "unregistered or changed nominal input schema",
                    ))
                }
            }
            noble_kernel::types::Ty::GenericNominal(_, _, _) => {
                if session
                    .declared
                    .as_ref()
                    .map(|context| &context.environment)
                    .or_else(|| {
                        session
                            .live_slots
                            .as_ref()
                            .map(|profile| &profile.environment)
                    })
                    .is_some_and(|env| env.valid_generic_instance(ty))
                {
                    Ok(())
                } else {
                    Err(crate::invalid(
                        span,
                        "unregistered or changed generic nominal input schema",
                    ))
                }
            }
            noble_kernel::types::Ty::Pair(_, _) | noble_kernel::types::Ty::Sum(_, _) => {
                attempt!(meter.node(span));
                attempt!(meter.node(span));
                self.pending.push(Visit {
                    step: PathStep::Left,
                    depth: next_depth,
                    borrow_allowed: false,
                });
                self.pending.push(Visit {
                    step: PathStep::Right,
                    depth: next_depth,
                    borrow_allowed: false,
                });
                Ok(())
            }
            noble_kernel::types::Ty::List(_) => {
                attempt!(meter.node(span));
                self.pending.push(Visit {
                    step: PathStep::Item,
                    depth: next_depth,
                    borrow_allowed: false,
                });
                Ok(())
            }
            noble_kernel::types::Ty::Program(input, output, effects) => {
                let cap = attempt!(crate::offset(crate::inference::STACK_CAP, span));
                if input.len() > cap || output.len() > cap {
                    return Err(super::exhausted(
                        span,
                        "source program interface stack limit exceeded",
                    ));
                }
                attempt!(host_effects(
                    effects,
                    session.program_type_effect_universe(),
                    span,
                    meter
                ));
                attempt!(self.schedule(input.len(), false, false, next_depth, span, meter));
                self.schedule(output.len(), true, false, next_depth, span, meter)
            }
            noble_kernel::types::Ty::LiveRef(input, output, effects) => {
                if !borrow_allowed || session.live_slots.is_none() {
                    return Err(crate::invalid(
                        span,
                        "LiveRef is only an invocation-input borrow or forwarded input",
                    ));
                }
                let cap = attempt!(crate::offset(crate::inference::STACK_CAP, span));
                if input.len() > cap || output.len() > cap {
                    return Err(super::exhausted(
                        span,
                        "live target interface stack limit exceeded",
                    ));
                }
                attempt!(host_effects(
                    effects,
                    session.effect_universe(),
                    span,
                    meter
                ));
                attempt!(self.schedule(input.len(), false, true, next_depth, span, meter));
                self.schedule(output.len(), true, false, next_depth, span, meter)
            }
            noble_kernel::types::Ty::Unit
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Text
            | noble_kernel::types::Ty::Syntax
            | noble_kernel::types::Ty::Contract
            | noble_kernel::types::Ty::Evidence
            | noble_kernel::types::Ty::Certified => Ok(()),
        }
    }

    #[expect(
        tigerstyle::too_many_parameters,
        reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
    )]
    #[expect(
        tigerstyle::ambiguous_params,
        reason = "Owner: noble-contracts; the same-typed parameters are positional by contract and named at every call site"
    )]
    fn schedule(
        &mut self,
        count: usize,
        output: bool,
        borrow_allowed: bool,
        depth: u32,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<(), crate::Diagnostic> {
        self.pending.reserve(count);
        let mut at = 0usize;
        let mut failure = None;
        while at < count {
            if let Err(problem) = meter.node(span) {
                failure = Some(problem);
                break;
            }
            let step = if output {
                PathStep::Output(at)
            } else {
                PathStep::Input(at)
            };
            self.pending.push(Visit {
                step,
                depth,
                borrow_allowed,
            });
            at = at.saturating_add(1);
        }
        match failure {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; every latent effect consumes work and must belong to the session's explicitly enabled test or generated-component effect universe; invalid effects and exhaustion return diagnostics."
)]
fn host_effects(
    effects: &noble_kernel::types::EffSet,
    universe: u64,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let effects = effects.as_slice();
    let mut at = 0usize;
    let mut failure = None;
    while at < effects.len() {
        let effect = effects[at];
        if let Err(problem) = meter.charge(1, span) {
            failure = Some(problem);
            break;
        }
        if 1u64.checked_shl(effect.0).unwrap_or(0) & universe == 0 {
            failure = Some(crate::Diagnostic::new(
                crate::DiagnosticKind::Unsupported,
                span,
                "unknown input program host effect",
            ));
            break;
        }
        at = at.saturating_add(1);
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}
