#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; traversal validates syntax lookups and completed stack equations, reporting malformed nested quotations without panics."
)]
pub(super) fn step(
    context: super::Context<'_>,
    mut frame: super::Frame,
    state: &mut super::Resolution,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, state.span));
    let items = attempt!(context.tree.square(frame.syntax));
    if frame.position >= items.len() {
        attempt!(state.arena.unify(
            frame.stack,
            frame.expected,
            attempt!(context.tree.node(frame.syntax)).span,
            meter
        ));
        return complete(frame, state, meter);
    }
    let id = attempt!(crate::syntax::child(items, frame.position, state.span));
    frame.position += 1;
    let node = attempt!(context.tree.node(id));
    match &node.form {
        crate::syntax::Form::Atom => atom(
            (attempt!(context.tree.atom(context.source, id)), node.span),
            frame,
            state,
            context.env,
            meter,
        ),
        crate::syntax::Form::Round(children) => super::forms::round(
            context,
            super::Form {
                children,
                span: node.span,
            },
            frame,
            state,
            meter,
        ),
        crate::syntax::Form::Square(_) => Err(crate::invalid(
            node.span,
            "nested program quotation requires (block (inputs) (outputs) [body])",
        )),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; quotation completion allocates its draft and parent body entry; missing parents remain explicit internal diagnostics."
)]
fn complete(
    frame: super::Frame,
    state: &mut super::Resolution,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    match frame.origin {
        None => state.body = frame.body,
        Some((surrounding, input, output, at)) => {
            let id = attempt!(state.add(
                super::Draft {
                    kind: super::DraftKind::Quotation(frame.body),
                    variables: alloc::vec![
                        crate::inference::Variable::Stack(surrounding),
                        crate::inference::Variable::Stack(input),
                        crate::inference::Variable::Stack(output),
                        crate::inference::Variable::Effect
                    ],
                    explicit: None,
                    span: at,
                },
                meter
            ));
            match state.frames.last_mut() {
                Some(parent) => parent.body.push(id),
                None => return Err(crate::internal(at)),
            }
        }
    }
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; literal/word inference propagates type and budget errors while allocating terms, drafts, and frame continuations."
)]
fn atom(
    token: (&[u8], crate::Span),
    mut frame: super::Frame,
    state: &mut super::Resolution,
    env: &noble_kernel::contracts::Env,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let (word, span) = token;
    match attempt!(literal(word, span, meter)) {
        Some(lit) => {
            let ty = attempt!(state.arena.ty(&lit.ty(), span, meter));
            let next = attempt!(state.arena.add(
                crate::inference::Term::Push(frame.stack, ty),
                span,
                meter
            ));
            frame.body.push(attempt!(state.add(
                super::Draft {
                    kind: super::DraftKind::Literal(lit),
                    variables: alloc::vec![crate::inference::Variable::Stack(frame.stack)],
                    explicit: None,
                    span,
                },
                meter
            )));
            frame.stack = next;
        }
        None => {
            let def = attempt!(super::words::identity(word, span));
            let (next, draft) = attempt!(super::words::apply(
                super::words::Call {
                    def,
                    current: frame.stack,
                    explicit: None,
                    span
                },
                &mut state.arena,
                env,
                meter
            ));
            frame.body.push(attempt!(state.add(draft, meter)));
            frame.stack = next;
        }
    }
    state.frames.push(frame);
    Ok(())
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; signed integer parsing meters source bytes and returns owned range diagnostics."
)]
fn literal(
    word: &[u8],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<Option<noble_kernel::untrusted::Lit>, crate::Diagnostic> {
    if word == b"true" {
        return Ok(Some(noble_kernel::untrusted::Lit::Bool(true)));
    }
    if word == b"false" {
        return Ok(Some(noble_kernel::untrusted::Lit::Bool(false)));
    }
    let Some(value) = attempt!(crate::syntax::integer(word, span, meter)) else {
        return Ok(None);
    };
    Ok(Some(noble_kernel::untrusted::Lit::I64(value)))
}
