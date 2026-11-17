pub(super) fn named(
    container: super::Container<'_>,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<crate::NamedType>, crate::Diagnostic> {
    let mut names = alloc::vec::Vec::with_capacity(container.children.len().saturating_sub(1));
    let mut at = 1usize;
    let mut failure = None;
    while at < container.children.len() {
        match binding(container, at, &names, meter) {
            Ok(binding) => names.push(binding),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        at += 1;
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(names),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; binding arity, freshness, names, and types are checked as fallible source errors; none should become a panic."
)]
fn binding(
    container: super::Container<'_>,
    at: usize,
    names: &[crate::NamedType],
    meter: &mut crate::Meter,
) -> Result<crate::NamedType, crate::Diagnostic> {
    attempt!(meter.charge(1, container.span));
    let id = attempt!(crate::syntax::child(container.children, at, container.span));
    let at_span = attempt!(container.tree.node(id)).span;
    let binding = attempt!(container.tree.round(id));
    if binding.len() != 2 {
        return Err(crate::invalid(
            at_span,
            "binding requires a name and one type",
        ));
    }
    let name = attempt!(crate::syntax::identifier(
        container.source,
        container.tree,
        attempt!(crate::syntax::child(binding, 0, at_span)),
        meter
    ));
    let mut previous = 0usize;
    let mut failure = None;
    while previous < names.len() {
        if let Err(error) = fresh(names, previous, &name, at_span, meter) {
            failure = Some(error);
            break;
        }
        previous += 1;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    let ty = attempt!(crate::syntax::ty(
        container.source,
        container.tree,
        attempt!(crate::syntax::child(binding, 1, at_span)),
        meter
    ));
    Ok(crate::NamedType { name, ty })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; metering and duplicate-name failures allocate diagnostics through non-const helpers."
)]
fn fresh(
    names: &[crate::NamedType],
    at: usize,
    name: &str,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(
        attempt!(crate::index(name.len(), span)).saturating_add(1),
        span
    ));
    match names.get(at) {
        Some(binding) => {
            if binding.name.as_str() == name {
                Err(crate::invalid(
                    span,
                    "name is rebound in the same binding namespace",
                ))
            } else {
                Ok(())
            }
        }
        None => Err(crate::internal(span)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the stack cap and every metered type lookup return explicit diagnostics, including exhaustion."
)]
pub(super) fn types(
    bindings: &[crate::NamedType],
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<alloc::vec::Vec<noble_kernel::types::Ty>, crate::Diagnostic> {
    if bindings.len() > attempt!(crate::offset(crate::inference::STACK_CAP, span)) {
        return Err(crate::Diagnostic::new(
            crate::DiagnosticKind::Exhausted,
            span,
            "declared stack exceeds the 256-value limit",
        ));
    }
    let mut types = alloc::vec::Vec::with_capacity(bindings.len());
    let mut at = 0usize;
    let mut failure = None;
    while at < bindings.len() {
        match cloned_type(bindings, at, span, meter) {
            Ok(ty) => types.push(ty),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        at += 1;
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(types),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; cloning a structural Ty allocates recursive storage and invokes a non-const Clone implementation."
)]
fn cloned_type(
    bindings: &[crate::NamedType],
    at: usize,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<noble_kernel::types::Ty, crate::Diagnostic> {
    attempt!(meter.charge(crate::syntax::TYPE_CAP, span));
    match bindings.get(at) {
        Some(binding) => Ok(binding.ty.clone()),
        None => Err(crate::internal(span)),
    }
}
