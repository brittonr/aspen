#[octet::sealed_enum]
enum Step {
    Visit(u32),
    Finish(super::application::Form),
}

struct State {
    span: crate::Span,
    steps: alloc::vec::Vec<Step>,
    values: alloc::vec::Vec<u32>,
}

#[derive(Clone, Copy)]
struct Round<'a> {
    children: &'a [u32],
    span: crate::Span,
}

impl State {
    #[expect(
        clippy::vec_init_then_push,
        reason = "Owner: noble-maintainers; explicit owned push avoids the pinned Aeneas erased-region failure in vec! array conversion; reassess when the translator supports it."
    )]
    fn new(root: u32, span: crate::Span) -> Self {
        let mut steps = alloc::vec::Vec::with_capacity(1);
        steps.push(Step::Visit(root));
        Self {
            span,
            steps,
            values: alloc::vec::Vec::new(),
        }
    }
}

pub(crate) fn resolve(
    context: super::Context<'_>,
    root: u32,
    arena: &mut super::Arena,
    meter: &mut crate::Meter,
) -> Result<u32, crate::Diagnostic> {
    let span = attempt!(context.tree.node(root)).span;
    let mut state = State::new(root, span);
    let mut failure = None;
    while let Some(next) = state.steps.pop() {
        if let Err(error) = step(context, next, &mut state, arena, meter) {
            failure = Some(error);
            break;
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    if state.values.len() != 1 {
        return Err(crate::internal(span));
    }
    super::required(state.values.pop(), span)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; worklist execution checks required operands and propagates metering errors while growing the expression arena."
)]
fn step(
    context: super::Context<'_>,
    step: Step,
    state: &mut State,
    arena: &mut super::Arena,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, state.span));
    match step {
        Step::Visit(id) => visit(context, id, state, arena, meter),
        Step::Finish(form) => {
            let third = if super::arity(form.op) == 3 {
                Some(attempt!(super::required(state.values.pop(), form.span)))
            } else {
                None
            };
            let second = if super::arity(form.op) >= 2 {
                Some(attempt!(super::required(state.values.pop(), form.span)))
            } else {
                None
            };
            let first = if super::arity(form.op) >= 1 {
                Some(attempt!(super::required(state.values.pop(), form.span)))
            } else {
                None
            };
            let expr = attempt!(super::application::resolve(
                form,
                (first, second, third),
                &arena.expressions,
                meter
            ));
            state.values.push(attempt!(arena.push(expr, meter)));
            Ok(())
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; syntax IDs, literal parsing, and unsupported quotations produce diagnostics; accepted literals allocate arena entries."
)]
fn visit(
    context: super::Context<'_>,
    id: u32,
    state: &mut State,
    arena: &mut super::Arena,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let node = attempt!(context.tree.node(id));
    match &node.form {
        crate::syntax::Form::Atom => {
            let word = attempt!(context.tree.atom(context.source, id));
            let (kind, ty) = attempt!(literal(word, node.span, meter));
            state.values.push(attempt!(arena.push(
                crate::Expr {
                    kind,
                    ty,
                    span: node.span,
                    total: true,
                    uses_output: false
                },
                meter
            )));
            Ok(())
        }
        crate::syntax::Form::Round(children) => round(
            context,
            Round {
                children,
                span: node.span,
            },
            state,
            arena,
            meter,
        ),
        crate::syntax::Form::Square(_) => Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            node.span,
            "executable quotations are not logical terms; refer to a typed program binding",
        )),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; integer parsing meters source bytes and invalid atoms allocate diagnostics."
)]
fn literal(
    word: &[u8],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(crate::ExprKind, noble_kernel::types::Ty), crate::Diagnostic> {
    if word == b"true" {
        return Ok((crate::ExprKind::Bool(true), noble_kernel::types::Ty::Bool));
    }
    if word == b"false" {
        return Ok((crate::ExprKind::Bool(false), noble_kernel::types::Ty::Bool));
    }
    if word == b"unit" {
        return Ok((crate::ExprKind::Unit, noble_kernel::types::Ty::Unit));
    }
    match attempt!(crate::syntax::integer(word, span, meter)) {
        Some(value) => Ok((crate::ExprKind::I64(value), noble_kernel::types::Ty::I64)),
        None => Err(crate::invalid(
            span,
            "unknown expression atom; references must name in, out, param, or def explicitly",
        )),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    tigerstyle::compound_condition,
    reason = "Owner: noble-maintainers; the four explicit reference namespaces share one arity check and fallible lookup; successful lookup grows the expression arena."
)]
fn round(
    context: super::Context<'_>,
    round: Round<'_>,
    state: &mut State,
    arena: &mut super::Arena,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let op = attempt!(context.tree.atom(
        context.source,
        attempt!(crate::syntax::child(round.children, 0, round.span))
    ));
    if op == b"in" || op == b"out" || op == b"param" || op == b"def" {
        if round.children.len() != 2 {
            return Err(crate::invalid(
                round.span,
                "binding reference requires exactly one name",
            ));
        }
        let name = attempt!(context.tree.atom(
            context.source,
            attempt!(crate::syntax::child(round.children, 1, round.span))
        ));
        let expr = attempt!(super::references::resolve(
            super::references::Reference {
                operator: op,
                name,
                span: round.span
            },
            context,
            &arena.expressions,
            meter
        ));
        state.values.push(attempt!(arena.push(expr, meter)));
        return Ok(());
    }
    queue_operator(context, round, op, state, meter)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; arity is 0..=3 by the closed Op match, so adding one or two is safe. Annotation/arity validation returns diagnostics and scheduling grows a Vec."
)]
fn queue_operator(
    context: super::Context<'_>,
    round: Round<'_>,
    op: &[u8],
    state: &mut State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let operation = attempt!(super::operation(op, round.span));
    let arity = super::arity(operation);
    let is_annotated = matches!(operation, super::Op::Inl | super::Op::Inr | super::Op::Nil);
    let expected = arity + if is_annotated { 2 } else { 1 };
    if round.children.len() != expected {
        return Err(crate::invalid(
            round.span,
            "expression operator has the wrong number of arguments",
        ));
    }
    let annotation = if is_annotated {
        Some(attempt!(crate::syntax::ty(
            context.source,
            context.tree,
            attempt!(crate::syntax::child(round.children, 1, round.span)),
            meter
        )))
    } else {
        None
    };
    state.steps.push(Step::Finish(super::application::Form {
        op: operation,
        span: round.span,
        annotation,
    }));
    let start = if is_annotated { 2 } else { 1 };
    queue_children(round, start, state, meter)
}

fn queue_children(
    round: Round<'_>,
    start: usize,
    state: &mut State,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    let mut at = round.children.len();
    let mut failure = None;
    while at > start {
        at -= 1;
        match queued_child(round, at, meter) {
            Ok(step) => state.steps.push(step),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn queued_child(
    round: Round<'_>,
    at: usize,
    meter: &mut crate::Meter,
) -> Result<Step, crate::Diagnostic> {
    attempt!(meter.charge(1, round.span));
    Ok(Step::Visit(attempt!(crate::syntax::child(
        round.children,
        at,
        round.span
    ))))
}
