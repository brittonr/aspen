#[derive(Clone, Copy)]
pub(super) struct Context<'a> {
    pub(super) source: &'a [u8],
    pub(super) tree: &'a super::Tree,
}

#[octet::sealed_enum]
pub(super) enum Step {
    Visit(u32),
    Pair,
    Sum,
    List,
    Program(usize, usize),
}

pub(super) struct State {
    pub(super) steps: alloc::vec::Vec<Step>,
    pub(super) values: alloc::vec::Vec<noble_kernel::types::Ty>,
    constructors: u32,
}

impl State {
    #[expect(
        clippy::vec_init_then_push,
        reason = "Owner: noble-maintainers; explicit owned push avoids the pinned Aeneas erased-region failure in vec! array conversion; reassess when the translator supports it."
    )]
    pub(super) fn new(root: u32) -> Self {
        let mut steps = alloc::vec::Vec::with_capacity(1);
        steps.push(Step::Visit(root));
        Self {
            steps,
            values: alloc::vec::Vec::new(),
            constructors: 0,
        }
    }

    #[expect(
        tigerstyle::missing_const_fn,
        reason = "Owner: noble-maintainers; taking a type mutates an owned Vec and metering or missing-value failures construct diagnostics."
    )]
    fn take_type(
        &mut self,
        span: crate::Span,
        meter: &mut crate::Meter,
    ) -> Result<noble_kernel::types::Ty, crate::Diagnostic> {
        attempt!(meter.charge(1, span));
        match self.values.pop() {
            Some(value) => Ok(value),
            None => Err(crate::internal(span)),
        }
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; constructor limits and required child values are explicit error checks before owned structural types are assembled."
)]
pub(super) fn step(
    context: Context<'_>,
    step: Step,
    span: crate::Span,
    mut state: State,
    meter: &mut crate::Meter,
) -> Result<State, crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    match step {
        Step::Visit(id) => {
            state.constructors = state.constructors.saturating_add(1);
            if state.constructors > super::TYPE_CAP {
                return Err(crate::Diagnostic::new(
                    crate::DiagnosticKind::Exhausted,
                    span,
                    "type size limit (256 constructors) exceeded",
                ));
            }
            state = attempt!(visit(context, id, state, meter));
        }
        Step::Pair | Step::Sum => {
            let right = match state.values.pop() {
                Some(value) => value,
                None => return Err(crate::internal(span)),
            };
            let left = match state.values.pop() {
                Some(value) => value,
                None => return Err(crate::internal(span)),
            };
            state.values.push(if matches!(step, Step::Pair) {
                noble_kernel::types::Ty::Pair(
                    alloc::boxed::Box::new(left),
                    alloc::boxed::Box::new(right),
                )
            } else {
                noble_kernel::types::Ty::Sum(
                    alloc::boxed::Box::new(left),
                    alloc::boxed::Box::new(right),
                )
            });
        }
        Step::List => {
            let value = match state.values.pop() {
                Some(value) => value,
                None => return Err(crate::internal(span)),
            };
            state
                .values
                .push(noble_kernel::types::Ty::List(alloc::boxed::Box::new(value)));
        }
        Step::Program(input_count, output_count) => {
            let (next, outputs) = attempt!(take_types(state, output_count, span, meter));
            let (next, inputs) = attempt!(take_types(next, input_count, span, meter));
            state = next;
            state.values.push(noble_kernel::types::Ty::program(
                inputs,
                outputs,
                noble_kernel::types::EffSet::empty(),
            ));
        }
    }
    Ok(state)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; type visiting performs fallible syntax lookup and grows owned value and continuation Vecs."
)]
fn visit(
    context: Context<'_>,
    id: u32,
    mut state: State,
    meter: &mut crate::Meter,
) -> Result<State, crate::Diagnostic> {
    let node = attempt!(context.tree.node(id));
    match &node.form {
        super::Form::Atom => {
            let kind = attempt!(context.tree.atom(context.source, id));
            state.values.push(attempt!(atom_type(kind, node.span)));
        }
        super::Form::Round(children) => {
            state = attempt!(super::constructors::schedule(
                context, children, node.span, state, meter
            ));
        }
        super::Form::Square(_) => {
            return Err(crate::invalid(node.span, "program syntax is not a type"))
        }
    }
    Ok(state)
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; this total byte-token classifier uses non-const slice equality and reports unknown or resource types through owned diagnostics."
)]
fn atom_type(kind: &[u8], span: crate::Span) -> Result<noble_kernel::types::Ty, crate::Diagnostic> {
    if kind == b"Unit" {
        return Ok(noble_kernel::types::Ty::Unit);
    }
    if kind == b"Bool" {
        return Ok(noble_kernel::types::Ty::Bool);
    }
    if kind == b"I64" {
        return Ok(noble_kernel::types::Ty::I64);
    }
    if kind == b"Text" {
        return Ok(noble_kernel::types::Ty::Text);
    }
    if kind == b"Syntax" {
        return Ok(noble_kernel::types::Ty::Syntax);
    }
    if kind == b"Contract" {
        return Ok(noble_kernel::types::Ty::Contract);
    }
    if kind == b"Evidence" {
        return Ok(noble_kernel::types::Ty::Evidence);
    }
    if kind == b"Certified" {
        return Ok(noble_kernel::types::Ty::Certified);
    }
    if kind == b"Resource" {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Unsupported,
            span,
            "resource and host types are outside the pure contract fragment",
        ));
    }
    Err(crate::invalid(span, "unknown type"))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each required child type is popped fallibly and the final order restoration is metered before returning."
)]
fn take_types(
    mut state: State,
    mut count: usize,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(State, alloc::vec::Vec<noble_kernel::types::Ty>), crate::Diagnostic> {
    // The constructor records this exact count before its children are visited.
    let mut result = alloc::vec::Vec::with_capacity(count);
    let mut failure = None;
    while count > 0 {
        match state.take_type(span, meter) {
            Ok(value) => result.push(value),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        count -= 1;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    attempt!(meter.charge(attempt!(crate::index(result.len(), span)), span));
    result.reverse();
    Ok((state, result))
}

pub(super) fn item(
    context: Context<'_>,
    children: &[u32],
    at: usize,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::types::Ty, crate::Diagnostic> {
    attempt!(meter.charge(1, span));
    super::ty(
        context.source,
        context.tree,
        attempt!(super::child(children, at, span)),
        meter,
    )
}
