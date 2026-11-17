#[expect(
    tigerstyle::borrowed_argument_types,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each frame step is work-charged and child frames require depth/node checks before reserving and pushing continuations; the owned growing worklist cannot be a fixed slice."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
pub(super) fn visit(
    mut frame: super::Frame,
    parents: &mut alloc::vec::Vec<super::Frame>,
    state: &mut super::State,
    scope: &super::Scope<'_>,
    editor_analysis: bool,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, frame.span));
    let tree = attempt!(scope.tree(frame.tree, frame.span));
    let items = attempt!(super::items(tree, frame.items));
    let id = match items.get(frame.at) {
        Some(id) => *id,
        None => return super::completion::complete(frame, parents, state, meter),
    };
    let node = attempt!(tree.node(id));
    frame.at = frame.at.saturating_add(1);
    match &node.kind {
        crate::source::Kind::Literal(_) | crate::source::Kind::Text(_) => {
            attempt!(super::operations::literal(node, &mut frame, state, meter));
        }
        crate::source::Kind::Call(crate::source::Target::Builtin(id)) => {
            attempt!(super::operations::builtin(
                super::operations::Call {
                    id: *id,
                    span: node.span
                },
                &mut frame,
                state,
                scope.environment,
                meter
            ));
        }
        crate::source::Kind::Call(crate::source::Target::SelfOwner) => {
            attempt!(super::operations::signed_self(
                node.span, &mut frame, state, scope, meter
            ));
        }
        crate::source::Kind::Call(crate::source::Target::SlotInvoke(ordinal)) => {
            if !matches!(frame.origin, super::Origin::Root) {
                return Err(crate::invalid(
                    node.span,
                    "slot.invoke cannot capture a borrowed reference in a quotation or definition",
                ));
            }
            attempt!(super::operations::slot_invoke(
                *ordinal, node.span, &mut frame, state, meter,
            ));
        }
        crate::source::Kind::Call(crate::source::Target::Named(_))
        | crate::source::Kind::Quotation(_) => {
            attempt!(meter.depth(
                attempt!(crate::index(parents.len().saturating_add(1), node.span)),
                node.span
            ));
            let child = attempt!(child((id, node), &frame, state, scope, meter));
            attempt!(meter.node(node.span));
            parents.reserve(2);
            parents.push(frame);
            parents.push(child);
            return Ok(());
        }
        crate::source::Kind::EditorHole => {
            if !editor_analysis {
                return Err(crate::invalid(
                    node.span,
                    "editor hole cannot enter source admission",
                ));
            }
            let output = attempt!(state.arena.add(
                crate::inference::Term::Hole(crate::inference::Sort::Stack),
                node.span,
                meter
            ));
            let effect = attempt!(state.arena.effect_hole(node.span, meter));
            attempt!(meter.node(node.span));
            state.holes.push(super::Hole {
                input: frame.stack,
                output,
                effect,
                span: node.span,
            });
            frame.stack = output;
            frame.effect =
                attempt!(state
                    .arena
                    .effect_union(frame.effect, effect, node.span, meter));
        }
        crate::source::Kind::Word(_) => return Err(crate::internal(node.span)),
    }
    parents.push(frame);
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; named lookup checks immutable namespace identity before constructing a fresh metered frame; invalid node kinds and missing definitions return owned diagnostics instead of assertions."
)]
fn child(
    location: (u32, &crate::source::Node),
    frame: &super::Frame,
    state: &mut super::State,
    scope: &super::Scope<'_>,
    meter: &mut crate::Meter,
) -> Result<super::Frame, crate::Diagnostic> {
    let (node_id, node) = location;
    match &node.kind {
        crate::source::Kind::Call(crate::source::Target::Named(id)) => {
            let definition = match scope
                .session
                .definitions
                .get(attempt!(crate::offset(*id, node.span)))
            {
                Some(definition) => definition,
                None => return Err(crate::internal(node.span)),
            };
            named(
                super::operations::Call {
                    id: *id,
                    span: node.span,
                },
                definition,
                frame,
                state,
                meter,
            )
        }
        crate::source::Kind::Quotation(_) => quotation(node_id, frame, state, node.span, meter),
        _ => Err(crate::internal(node.span)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; every named use allocates a fresh body and effect term after node charging, while visit has already checked expansion depth; no declaration instance is shared as a monomorphic value."
)]
fn named(
    call: super::operations::Call,
    definition: &crate::source::Named,
    frame: &super::Frame,
    state: &mut super::State,
    meter: &mut crate::Meter,
) -> Result<super::Frame, crate::Diagnostic> {
    // References only point to earlier installed bodies. Depth/work bounds
    // cover expansion, and every use receives a fresh body and effect term.
    let span = call.span;
    let input = frame.stack;
    let effect = attempt!(state.arena.effect_empty(span, meter));
    let (signature, owner_variables) = match definition.signature.as_ref() {
        Some(scheme) => {
            let (output, effects, variables) = attempt!(super::apply_signature(
                &mut state.arena,
                scheme,
                input,
                false,
                span,
                meter,
            ));
            (Some((output, effects, false)), Some(variables))
        }
        None => (None, None),
    };
    attempt!(meter.node(span));
    let body = state.bodies.len();
    state.bodies.push(super::Body {
        nodes: alloc::vec::Vec::new(),
        root: alloc::vec::Vec::new(),
        input,
        output: input,
        effect,
        identity: Some(definition.identity),
        owner: definition.owner,
        owner_slot: Some(call.id),
        owner_variables,
        span,
    });
    Ok(super::Frame {
        tree: super::TreeKey::Named(call.id),
        items: super::BodyKey::Root,
        at: 0,
        input,
        stack: input,
        effect,
        body,
        sequence: alloc::vec::Vec::new(),
        origin: super::Origin::Named,
        span,
        caller: frame.caller.or(Some(span)),
        signature,
    })
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; visit checks nesting depth before allocating this quotation's stack hole and effect term under the shared meter; completion installs their single monomorphic Program interface."
)]
fn quotation(
    node: u32,
    frame: &super::Frame,
    state: &mut super::State,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<super::Frame, crate::Diagnostic> {
    let input = attempt!(state.arena.add(
        crate::inference::Term::Hole(crate::inference::Sort::Stack),
        span,
        meter
    ));
    let effect = attempt!(state.arena.effect_empty(span, meter));
    Ok(super::Frame {
        tree: frame.tree,
        items: super::BodyKey::Quotation(node),
        at: 0,
        input,
        stack: input,
        effect,
        body: frame.body,
        sequence: alloc::vec::Vec::new(),
        origin: super::Origin::Quotation(frame.stack),
        span,
        caller: frame.caller,
        signature: None,
    })
}
