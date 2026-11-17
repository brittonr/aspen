#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; completed frames check body IDs and child completion checks parent presence; state-machine inconsistencies remain diagnostics instead of assertions."
)]
pub(super) fn complete(
    frame: super::Frame,
    parents: &mut [super::Frame],
    state: &mut super::State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    if let Some((output, effects, rigid)) = frame.signature {
        attempt!(state.arena.unify(frame.stack, output, frame.span, meter));
        if rigid {
            attempt!(state
                .arena
                .require_rigid_effect(frame.effect, effects, frame.span, meter));
        } else {
            attempt!(state
                .arena
                .equate_effects(frame.effect, effects, frame.span, meter));
        }
    }
    if !matches!(frame.origin, super::Origin::Quotation(_)) {
        let body = match state.bodies.get_mut(frame.body) {
            Some(body) => body,
            None => return Err(crate::internal(frame.span)),
        };
        body.input = frame.input;
        body.output = frame.stack;
        body.effect = frame.effect;
    }
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-maintainers; root, quotation, and named completion have distinct ownership and inference rules, so any new origin must require an explicit transition."
    )]
    match frame.origin {
        super::Origin::Root => {
            let body = match state.bodies.get_mut(frame.body) {
                Some(body) => body,
                None => return Err(crate::internal(frame.span)),
            };
            body.root = frame.sequence;
            Ok(())
        }
        super::Origin::Quotation(surrounding) => {
            quotation(frame, surrounding, parents, state, meter)
        }
        super::Origin::Named => named(frame, parents, state, meter),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; one monomorphic Program term and its draft are allocated under node/work limits, and updating the enclosing frame requires a checked parent; failures allocate diagnostics."
)]
fn quotation(
    frame: super::Frame,
    surrounding: u32,
    parents: &mut [super::Frame],
    state: &mut super::State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let program = attempt!(state.arena.program(
        crate::inference::Program {
            input: frame.input,
            output: frame.stack,
            effect: frame.effect,
        },
        frame.span,
        meter
    ));
    let output = attempt!(state.arena.add(
        crate::inference::Term::Push(surrounding, program),
        frame.span,
        meter
    ));
    let variables = alloc::vec![
        crate::inference::Variable::Stack(surrounding),
        crate::inference::Variable::Stack(frame.input),
        crate::inference::Variable::Stack(frame.stack),
        crate::inference::Variable::EffectValue(frame.effect)
    ];
    let id = attempt!(super::add(
        frame.body,
        super::Draft {
            kind: super::DraftKind::Quotation(frame.sequence),
            variables,
            text: None,
            span: frame.span
        },
        state,
        meter
    ));
    let parent = match parents.last_mut() {
        Some(parent) => parent,
        None => return Err(crate::internal(frame.span)),
    };
    parent.stack = output;
    parent.sequence.push(id);
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; both the fresh body's ID and its caller frame are checked before a metered invocation draft and effect union are installed; malformed state returns diagnostics."
)]
fn named(
    frame: super::Frame,
    parents: &mut [super::Frame],
    state: &mut super::State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let body = match state.bodies.get_mut(frame.body) {
        Some(body) => body,
        None => return Err(crate::internal(frame.span)),
    };
    body.root = frame.sequence;
    let variables = if body
        .nodes
        .iter()
        .any(|node| matches!(node.kind, super::DraftKind::SignedSelf(_)))
    {
        body.owner_variables
            .as_ref()
            .ok_or_else(|| {
                crate::invalid(
                    frame.span,
                    "signed invocation has no source owner substitution",
                )
            })?
            .clone()
    } else {
        alloc::vec::Vec::new()
    };
    let definition = noble_kernel::contracts::Definition(attempt!(crate::index(
        frame
            .body
            .saturating_sub(1)
            .saturating_add(state.definition_base),
        frame.span
    )));
    let parent = match parents.last_mut() {
        Some(parent) => parent,
        None => return Err(crate::internal(frame.span)),
    };
    let id = attempt!(super::add(
        parent.body,
        super::Draft {
            kind: super::DraftKind::Invocation(definition),
            variables,
            text: None,
            span: frame.span
        },
        state,
        meter
    ));
    parent.stack = frame.stack;
    parent.effect =
        attempt!(state
            .arena
            .effect_union(parent.effect, frame.effect, frame.span, meter));
    parent.sequence.push(id);
    Ok(())
}
