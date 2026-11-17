#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; constructor names, arities, and host-type exclusions are explicit fallible validation before children are scheduled."
)]
pub(super) fn schedule(
    context: super::typing::Context<'_>,
    children: &[u32],
    span: crate::Span,
    mut state: super::typing::State,
    meter: &mut crate::Meter,
) -> Result<super::typing::State, crate::Diagnostic> {
    let head = attempt!(super::child(children, 0, span));
    let op = attempt!(context.tree.atom(context.source, head));
    if op == b"Pair" || op == b"Sum" {
        if children.len() != 3 {
            return Err(crate::invalid(span, "Pair and Sum require two types"));
        }
        state.steps.push(if op == b"Pair" {
            super::typing::Step::Pair
        } else {
            super::typing::Step::Sum
        });
        state
            .steps
            .push(super::typing::Step::Visit(attempt!(super::child(
                children, 2, span
            ))));
        state
            .steps
            .push(super::typing::Step::Visit(attempt!(super::child(
                children, 1, span
            ))));
        return Ok(state);
    }
    if op == b"List" {
        if children.len() != 2 {
            return Err(crate::invalid(span, "List requires one item type"));
        }
        state.steps.push(super::typing::Step::List);
        state
            .steps
            .push(super::typing::Step::Visit(attempt!(super::child(
                children, 1, span
            ))));
        return Ok(state);
    }
    if op == b"Program" {
        return program(context, children, span, state, meter);
    }
    if op == b"Resource" || op == b"Effect" {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            span,
            "host and effect types are outside the pure contract fragment",
        ));
    }
    Err(crate::invalid(span, "unknown type constructor"))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; Program arity and effect restrictions return diagnostics, then its checked interface lists grow the owned worklist."
)]
fn program(
    context: super::typing::Context<'_>,
    children: &[u32],
    span: crate::Span,
    mut state: super::typing::State,
    meter: &mut crate::Meter,
) -> Result<super::typing::State, crate::Diagnostic> {
    if children.len() > 3 {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            span,
            "effect annotations are outside the pure Program type",
        ));
    }
    if children.len() != 3 {
        return Err(crate::invalid(
            span,
            "Program requires input and output type lists",
        ));
    }
    let input = attempt!(super::child(children, 1, span));
    let inputs = attempt!(context.tree.round(input));
    let output = attempt!(super::child(children, 2, span));
    let outputs = attempt!(context.tree.round(output));
    state
        .steps
        .push(super::typing::Step::Program(inputs.len(), outputs.len()));
    state = attempt!(queue_types(outputs, span, state, meter));
    queue_types(inputs, span, state, meter)
}

fn queue_types(
    children: &[u32],
    span: crate::Span,
    mut state: super::typing::State,
    meter: &mut crate::Meter,
) -> Result<super::typing::State, crate::Diagnostic> {
    // Each child contributes one step, charged before the stack grows.
    let mut remaining = children.len();
    let mut failure = None;
    while remaining > 0 {
        remaining -= 1;
        match queued_type(children, remaining, span, meter) {
            Ok(step) => state.steps.push(step),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(state),
    }
}

fn queued_type(
    children: &[u32],
    at: usize,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<super::typing::Step, crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    Ok(super::typing::Step::Visit(attempt!(super::child(
        children, at, span
    ))))
}
