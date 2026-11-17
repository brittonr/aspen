pub(super) struct Call {
    pub id: u32,
    pub span: crate::Span,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; text-byte accumulation is checked against the configured limit before cloning, and arena terms/drafts are node-charged and indexed fallibly."
)]
pub(super) fn literal(
    node: &crate::source::Node,
    frame: &mut super::Frame,
    state: &mut super::State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let (lit, text) = match &node.kind {
        crate::source::Kind::Literal(lit) => (*lit, None),
        crate::source::Kind::Text(bytes) => {
            let count = attempt!(crate::index(bytes.len(), node.span));
            state.text_bytes = match state.text_bytes.checked_add(count) {
                Some(total) if total <= meter.limits.bytes => total,
                Some(_) | None => {
                    return Err(crate::source::exhausted(
                        node.span,
                        "specialized text payload byte limit exceeded",
                    ))
                }
            };
            attempt!(meter.charge(count, node.span));
            (noble_kernel::untrusted::Lit::Text, Some(bytes.clone()))
        }
        _ => return Err(crate::internal(node.span)),
    };
    let ty = attempt!(state.arena.ty(&lit.ty(), node.span, meter));
    let output = attempt!(state.arena.add(
        crate::inference::Term::Push(frame.stack, ty),
        node.span,
        meter
    ));
    let variables = alloc::vec![crate::inference::Variable::Stack(frame.stack)];
    let id = attempt!(super::add(
        frame.body,
        super::Draft {
            kind: super::DraftKind::Literal(lit),
            variables,
            text,
            span: node.span
        },
        state,
        meter
    ));
    frame.sequence.push(id);
    frame.stack = output;
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; scheme lookup, fresh witness creation, stack unification, effect union, and draft insertion reject through typed diagnostics under the shared meter."
)]
pub(super) fn builtin(
    call: Call,
    frame: &mut super::Frame,
    state: &mut super::State,
    environment: &noble_kernel::contracts::Env,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let definition = noble_kernel::contracts::Definition(call.id);
    let scheme = match environment.scheme(definition) {
        Some(scheme) => scheme,
        None => return Err(crate::internal(call.span)),
    };
    let variables = attempt!(state.arena.variables(&scheme.var_kinds, call.span, meter));
    let input = attempt!(state
        .arena
        .pattern_stack(&scheme.stack_in, &variables, call.span, meter));
    let output =
        attempt!(state
            .arena
            .pattern_stack(&scheme.stack_out, &variables, call.span, meter));
    let effect =
        attempt!(state
            .arena
            .effect_pattern(&scheme.effects, &variables, call.span, meter));
    if let Err(error) = state.arena.unify(frame.stack, input, call.span, meter) {
        if error.kind != crate::DiagnosticKind::Invalid {
            return Err(error);
        }
        let origin = mismatched_literal_origin(frame, state, scheme);
        let message = state
            .arena
            .join_stacks(input, frame.stack, call.span, meter)
            .ok();
        let difference = state
            .arena
            .stack_difference(input, frame.stack, call.span, meter)
            .ok()
            .flatten()
            .map(|(shared_prefix, expected, actual)| crate::StackDifference {
                shared_prefix,
                expected,
                actual,
                expected_word: Some(call.span),
                actual_word: producer(frame, state),
            });
        return Err(contextualize(
            error,
            definition,
            environment.kind(definition),
            frame.caller,
            origin,
            message,
            difference,
        ));
    }
    frame.effect = attempt!(state
        .arena
        .effect_union(frame.effect, effect, call.span, meter));
    let id = attempt!(super::add(
        frame.body,
        super::Draft {
            kind: super::DraftKind::Invocation(definition),
            variables,
            text: None,
            span: call.span
        },
        state,
        meter
    ));
    frame.sequence.push(id);
    frame.stack = output;
    Ok(())
}

/// A signed occurrence uses the already instantiated owner body's variables.
/// In particular, a quotation cannot choose a fresh stack row or effect for
/// its recursive call; that would silently admit polymorphic recursion.
#[expect(
    tigerstyle::assertion_density,
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite; the matched enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added."
)]
pub(super) fn signed_self(
    span: crate::Span,
    frame: &mut super::Frame,
    state: &mut super::State,
    scope: &super::Scope<'_>,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let scheme = match frame.tree {
        super::TreeKey::Root => scope.root_signature,
        super::TreeKey::Named(index) => scope
            .session
            .definitions
            .get(attempt!(crate::offset(index, span)))
            .and_then(|definition| definition.signature.as_ref()),
    };
    let Some(scheme) = scheme else {
        return Err(crate::invalid(
            span,
            "signed self requires its owner signature",
        ));
    };
    let Some(body) = state.bodies.get(frame.body) else {
        return Err(crate::internal(span));
    };
    if body.owner.is_none() {
        return Err(crate::invalid(
            span,
            "signed self requires its declared module owner",
        ));
    }
    let Some(variables) = body.owner_variables.as_ref() else {
        return Err(crate::invalid(
            span,
            "signed self has no owner-body substitution",
        ));
    };
    let input = attempt!(state
        .arena
        .pattern_stack(&scheme.stack_in, variables, span, meter));
    let output = attempt!(state
        .arena
        .pattern_stack(&scheme.stack_out, variables, span, meter));
    let effects = attempt!(state
        .arena
        .effect_pattern(&scheme.effects, variables, span, meter));
    attempt!(state.arena.unify(frame.stack, input, span, meter));
    frame.effect = attempt!(state.arena.effect_union(frame.effect, effects, span, meter));
    let body_index = frame.body.saturating_sub(1);
    let target = state
        .definition_base
        .checked_add(body_index)
        .ok_or_else(|| crate::internal(span))?;
    let definition = noble_kernel::contracts::Definition(attempt!(crate::index(target, span)));
    let variables = variables.clone();
    let id = attempt!(super::add(
        frame.body,
        super::Draft {
            kind: super::DraftKind::SignedSelf(definition),
            variables,
            text: None,
            span,
        },
        state,
        meter
    ));
    frame.sequence.push(id);
    frame.stack = output;
    Ok(())
}

/// The span of the word that produced the current stack: the last node of the current body.
/// At the start of a body the stack is the body's input, which no word in it produced.
fn producer(frame: &super::Frame, state: &super::State) -> Option<crate::Span> {
    let last = frame.sequence.last()?;
    let body = state.bodies.get(frame.body)?;
    body.nodes
        .get(usize::try_from(last.0).ok()?)
        .map(|draft| draft.span)
}

#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::platform_dependent_cast,
    reason = "Owner: noble-contracts; the operand is a u32 table index and the cast is a widening that cannot fail where usize is at least 32 bits; the portable form would add fallible plumbing to infallible lookups"
)]
#[expect(
    tigerstyle::unbounded_collection_growth,
    reason = "Owner: noble-contracts; the collection is bounded by the surrounding admission limits, which are checked before this loop runs"
)]
/// A ref is selected from the root's typed borrowed-input sidecar, never
/// constructed from a value or literal. The unqualified spelling chooses the
/// uniquely most-specific compatible interface; ambiguous peers require the
/// explicit borrowed formal ordinal (`slot.invoke.N`).
#[expect(
    tigerstyle::allocation_in_loop,
    reason = "Owner: noble-contracts; the loop owns one forwarded slot vector per parameter and the allocation is charged to the metered budget"
)]
pub(super) fn slot_invoke(
    selected: Option<u32>,
    span: crate::Span,
    frame: &mut super::Frame,
    state: &mut super::State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let actual = attempt!(state.arena.stack_value(frame.stack, span, meter));
    let mut choice: Option<(u32, alloc::vec::Vec<u32>, usize)> = None;
    let mut is_ambiguous = false;
    for borrowed in &state.borrows {
        attempt!(meter.charge(1, span));
        if selected.is_some_and(|ordinal| ordinal != borrowed.ordinal) {
            continue;
        }
        let noble_kernel::types::Ty::LiveRef(input, _, _) = &borrowed.ty else {
            return Err(crate::internal(span));
        };
        let value_count = input
            .iter()
            .filter(|ty| !matches!(ty, noble_kernel::types::Ty::LiveRef(..)))
            .count();
        if value_count > actual.len()
            || !input
                .iter()
                .filter(|ty| !matches!(ty, noble_kernel::types::Ty::LiveRef(..)))
                .zip(&actual[actual.len().saturating_sub(value_count)..])
                .all(|(left, right)| left == right)
        {
            continue;
        }
        let mut forwarded = alloc::vec::Vec::new();
        let mut is_compatible = true;
        for formal in input
            .iter()
            .filter(|ty| matches!(ty, noble_kernel::types::Ty::LiveRef(..)))
        {
            let mut positions = state
                .borrows
                .iter()
                .filter(|candidate| &candidate.ty == formal)
                .map(|candidate| candidate.logical_position);
            match (positions.next(), positions.next()) {
                (Some(position), None) => forwarded.push(position),
                (None, _) | (Some(_), Some(_)) => {
                    is_compatible = false;
                    break;
                }
            }
        }
        if !is_compatible {
            continue;
        }
        let specificity = forwarded.len();
        if selected.is_some()
            || choice
                .as_ref()
                .is_none_or(|(_, _, previous)| specificity > *previous)
        {
            choice = Some((borrowed.ordinal, forwarded, specificity));
            is_ambiguous = false;
        } else if choice
            .as_ref()
            .is_some_and(|(_, _, previous)| specificity == *previous)
        {
            is_ambiguous = true;
        }
    }
    if is_ambiguous {
        return Err(crate::invalid(
            span,
            "is_ambiguous is_compatible borrowed references; select slot.invoke.N",
        ));
    }
    let Some((ordinal, forwarded, _)) = choice else {
        return Err(crate::invalid(
            span,
            "slot.invoke requires a is_compatible host-issued borrowed input and exact forwarded refs",
        ));
    };
    let noble_kernel::types::Ty::LiveRef(input, output, ceiling) =
        &state.borrows[ordinal as usize].ty
    else {
        return Err(crate::internal(span));
    };
    // Source preflight excludes refs from output; only values enter the stack.
    if output
        .iter()
        .any(|ty| matches!(ty, noble_kernel::types::Ty::LiveRef(..)))
    {
        return Err(crate::invalid(span, "live reference cannot be returned"));
    }
    let value_count = input
        .iter()
        .filter(|ty| !matches!(ty, noble_kernel::types::Ty::LiveRef(..)))
        .count();
    let prefix = &actual[..actual.len().saturating_sub(value_count)];
    let mut resulting = alloc::vec::Vec::with_capacity(prefix.len().saturating_add(output.len()));
    resulting.extend_from_slice(prefix);
    resulting.extend_from_slice(output);
    let after = attempt!(state.arena.stack(&resulting, span, meter));
    let invoked = ceiling.union(&noble_kernel::types::EffSet::from_ids(&[
        noble_kernel::contracts::LIVE_DISPATCH,
    ]));
    let effect = attempt!(state.arena.effect_constant(&invoked, span, meter));
    frame.effect = attempt!(state.arena.effect_union(frame.effect, effect, span, meter));
    let node = attempt!(super::add(
        frame.body,
        super::Draft {
            kind: super::DraftKind::SlotInvoke {
                ref_ordinal: ordinal,
                forwarded_ref_ordinals: forwarded,
            },
            variables: alloc::vec::Vec::new(),
            text: None,
            span,
        },
        state,
        meter
    ));
    frame.sequence.push(node);
    frame.stack = after;
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
fn mismatched_literal_origin(
    frame: &super::Frame,
    state: &super::State,
    scheme: &noble_kernel::words::Scheme,
) -> Option<crate::Span> {
    let node = frame.sequence.last()?;
    let draft = state
        .bodies
        .get(frame.body)?
        .nodes
        .get(usize::try_from(node.0).ok()?)?;
    let super::DraftKind::Literal(literal) = &draft.kind else {
        return None;
    };
    let expected = scheme.stack_in.last()?;
    let is_matches = match (expected, literal) {
        (noble_kernel::shapes::Pattern::I64, noble_kernel::untrusted::Lit::I64(_))
        | (noble_kernel::shapes::Pattern::Bool, noble_kernel::untrusted::Lit::Bool(_))
        | (noble_kernel::shapes::Pattern::Text, noble_kernel::untrusted::Lit::Text)
        | (noble_kernel::shapes::Pattern::Unit, noble_kernel::untrusted::Lit::Unit) => true,
        (
            noble_kernel::shapes::Pattern::I64
            | noble_kernel::shapes::Pattern::Bool
            | noble_kernel::shapes::Pattern::Text
            | noble_kernel::shapes::Pattern::Unit,
            _,
        ) => false,
        _ => return None,
    };
    (!is_matches).then_some(draft.span)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; the invariants of this linear dispatcher are enforced by its callers and the contracts test suite"
)]
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-contracts; the parameter list is the internal call shape; bundling it would change the extracted subject"
)]
fn contextualize(
    mut error: crate::Diagnostic,
    definition: noble_kernel::contracts::Definition,
    behavior: Option<noble_kernel::contracts::Behavior>,
    caller: Option<crate::Span>,
    origin: Option<crate::Span>,
    stacks: Option<(alloc::string::String, alloc::string::String)>,
    difference: Option<crate::StackDifference>,
) -> crate::Diagnostic {
    let (expected_stack, actual_stack) = match stacks {
        Some(stacks) => stacks,
        None => return error,
    };
    let mut message = alloc::string::String::from("stack/program join: expected ");
    message.push_str(&expected_stack);
    message.push_str("; actual ");
    message.push_str(&actual_stack);
    message.push_str("; word ");
    let word_start = message.len();
    crate::program::append_bootstrap_spelling(definition, &mut message);
    let word = alloc::string::String::from(&message[word_start..]);
    message.push_str("; ");
    message.push_str(&error.message);
    if let Some(caller) = caller {
        message.push_str("; while specializing a resolved definition");
        error.span = caller;
    }
    error.message = message;
    error.join_with(crate::JoinDiagnostic {
        word,
        expected_stack,
        actual_stack,
        constraint: if matches!(
            behavior,
            Some(
                noble_kernel::contracts::Behavior::If
                    | noble_kernel::contracts::Behavior::Case
                    | noble_kernel::contracts::Behavior::ListCase
            )
        ) {
            "branch-join"
        } else {
            "stack-type"
        },
        value_origin: if caller.is_some() { None } else { origin },
        // While specializing a resolved definition the words lie in another body, so, as for the
        // value origin, only the positional difference is kept.
        difference: difference.map(|difference| {
            if caller.is_some() {
                crate::StackDifference {
                    expected_word: None,
                    actual_word: None,
                    ..difference
                }
            } else {
                difference
            }
        }),
    })
}
