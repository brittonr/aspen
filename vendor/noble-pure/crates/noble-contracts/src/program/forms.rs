#[expect(
    tigerstyle::assertion_density,
    tigerstyle::compound_condition,
    reason = "Owner: noble-maintainers; the four logical namespaces share an explicit ghost-to-runtime rejection; all other invalid forms return diagnostics."
)]
pub(super) fn round(
    context: super::Context<'_>,
    form: super::Form<'_>,
    frame: super::Frame,
    state: &mut super::Resolution,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let operation = attempt!(context.tree.atom(
        context.source,
        attempt!(crate::syntax::child(form.children, 0, form.span))
    ));
    if operation == b"block" {
        return block(context, form, frame, state, meter);
    }
    if operation == b"word" {
        return word(context, form, frame, state, meter);
    }
    if operation == b"param" || operation == b"in" || operation == b"out" || operation == b"def" {
        return Err(crate::invalid(form.span, "logical bindings cannot supply erased runtime values; capture the actual stack value with quote"));
    }
    Err(crate::Diagnostic::new(
        crate::DiagnosticKind::Unsupported,
        form.span,
        "unsupported program form; use a typed block or explicit word witness",
    ))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; word identity and witness checks return diagnostics, then accepted inference grows draft and frame Vecs."
)]
fn word(
    context: super::Context<'_>,
    form: super::Form<'_>,
    mut frame: super::Frame,
    state: &mut super::Resolution,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let def_id = attempt!(super::bindings::natural(
        context,
        attempt!(crate::syntax::child(form.children, 1, form.span)),
        meter
    ));
    if def_id == 22 {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            form.span,
            "test.emit is an unsupported host effect",
        ));
    }
    if def_id > 22 {
        return Err(crate::invalid(
            form.span,
            "unknown word definition identity",
        ));
    }
    let explicit = attempt!(super::bindings::parse(context, form, meter));
    let (next, draft) = attempt!(super::words::apply(
        super::words::Call {
            def: noble_kernel::contracts::Definition(def_id),
            current: frame.stack,
            explicit: Some(explicit),
            span: form.span,
        },
        &mut state.arena,
        context.env,
        meter
    ));
    frame.body.push(attempt!(state.add(draft, meter)));
    frame.stack = next;
    state.frames.push(frame);
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; block arity, interfaces, body shape, inferred equations, and depth limits are explicit fallible checks."
)]
fn block(
    context: super::Context<'_>,
    form: super::Form<'_>,
    mut frame: super::Frame,
    state: &mut super::Resolution,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    if form.children.len() != 4 {
        return Err(crate::invalid(
            form.span,
            "block requires input types, output types, and a bracketed body",
        ));
    }
    let input_types = attempt!(crate::syntax::types(
        context.source,
        context.tree,
        attempt!(crate::syntax::child(form.children, 1, form.span)),
        meter
    ));
    let output_types = attempt!(crate::syntax::types(
        context.source,
        context.tree,
        attempt!(crate::syntax::child(form.children, 2, form.span)),
        meter
    ));
    let child_body = attempt!(crate::syntax::child(form.children, 3, form.span));
    attempt!(context.tree.square(child_body));
    let input = attempt!(state.arena.stack(&input_types, form.span, meter));
    let output = attempt!(state.arena.stack(&output_types, form.span, meter));
    let program = attempt!(state.arena.add(
        crate::inference::Term::Program(input, output),
        form.span,
        meter
    ));
    let surrounding = frame.stack;
    frame.stack = attempt!(state.arena.add(
        crate::inference::Term::Push(surrounding, program),
        form.span,
        meter
    ));
    attempt!(meter.depth(
        attempt!(crate::index(state.frames.len(), form.span)).saturating_add(1),
        form.span
    ));
    state.frames.push(frame);
    state.frames.push(super::Frame {
        syntax: child_body,
        position: 0,
        stack: input,
        expected: output,
        body: alloc::vec::Vec::new(),
        origin: Some((surrounding, input, output, form.span)),
    });
    Ok(())
}
