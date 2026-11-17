pub(super) fn natural(
    context: super::Context<'_>,
    id: u32,
    meter: &mut crate::Meter,
) -> Result<u32, crate::Diagnostic> {
    let span = attempt!(context.tree.node(id)).span;
    match attempt!(crate::syntax::integer(
        attempt!(context.tree.atom(context.source, id)),
        span,
        meter
    )) {
        Some(value) => match u32::try_from(value) {
            Ok(value) => Ok(value),
            Err(_) => Err(crate::invalid(span, "expected a nonnegative u32 identity")),
        },
        None => Err(crate::invalid(span, "expected a decimal identity")),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; each explicit witness binding is parsed fallibly and preserves the first user-facing error."
)]
pub(super) fn parse(
    context: super::Context<'_>,
    form: super::Form<'_>,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::words::Inst, crate::Diagnostic> {
    let mut out = alloc::vec::Vec::with_capacity(form.children.len().saturating_sub(2));
    let mut at = 2usize;
    let mut failure = None;
    while at < form.children.len() {
        match binding(context, form, at, meter) {
            Ok(binding) => {
                out.push(binding);
                at += 1;
            }
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(noble_kernel::words::Inst { bindings: out }),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; explicit binding kinds, arities, identities, and pure effects are checked through diagnostics while structural types allocate."
)]
fn binding(
    context: super::Context<'_>,
    form: super::Form<'_>,
    at: usize,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::words::Binding, crate::Diagnostic> {
    attempt!(meter.charge(1, form.span));
    let id = attempt!(crate::syntax::child(form.children, at, form.span));
    let span = attempt!(context.tree.node(id)).span;
    let parts = attempt!(context.tree.round(id));
    let op = attempt!(context.tree.atom(
        context.source,
        attempt!(crate::syntax::child(parts, 0, span))
    ));
    let form = super::Form {
        children: parts,
        span,
    };
    if op == b"stack" {
        return Ok(noble_kernel::words::Binding::Stack(attempt!(stack(
            context, form, meter
        ))));
    }
    if op == b"value" {
        if parts.len() != 2 {
            return Err(crate::invalid(
                span,
                "value binding requires exactly one type",
            ));
        }
        return Ok(noble_kernel::words::Binding::Value(attempt!(
            crate::syntax::ty(
                context.source,
                context.tree,
                attempt!(crate::syntax::child(parts, 1, span)),
                meter
            )
        )));
    }
    if op == b"effect" {
        if parts.len() != 1 {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Unsupported,
                span,
                "nonempty effect bindings are outside the pure fragment",
            ));
        }
        return Ok(noble_kernel::words::Binding::Effect(
            noble_kernel::types::EffSet::empty(),
        ));
    }
    if op == b"ref" {
        if parts.len() != 2 {
            return Err(crate::invalid(
                span,
                "ref binding requires exactly one variable identity",
            ));
        }
        return Ok(noble_kernel::words::Binding::Ref(
            noble_kernel::words::Variable(attempt!(natural(
                context,
                attempt!(crate::syntax::child(parts, 1, span)),
                meter
            ))),
        ));
    }
    Err(crate::invalid(
        span,
        "unknown witness binding; expected stack, value, effect, or ref",
    ))
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; metered stack-type parsing propagates explicit type and exhaustion errors instead of panic preconditions."
)]
fn stack(
    context: super::Context<'_>,
    form: super::Form<'_>,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::types::Ty>, crate::Diagnostic> {
    let mut types = alloc::vec::Vec::with_capacity(form.children.len().saturating_sub(1));
    let mut at = 1usize;
    let mut failure = None;
    while at < form.children.len() {
        match stack_type(context, form, at, meter) {
            Ok(ty) => {
                types.push(ty);
                at += 1;
            }
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(types),
    }
}

fn stack_type(
    context: super::Context<'_>,
    form: super::Form<'_>,
    at: usize,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::types::Ty, crate::Diagnostic> {
    attempt!(meter.charge(1, form.span));
    crate::syntax::ty(
        context.source,
        context.tree,
        attempt!(crate::syntax::child(form.children, at, form.span)),
        meter,
    )
}
