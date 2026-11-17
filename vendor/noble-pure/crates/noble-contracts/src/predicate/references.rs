pub(super) struct Reference<'a> {
    pub operator: &'a [u8],
    pub name: &'a [u8],
    pub span: crate::Span,
}

#[derive(Clone, Copy)]
struct Search<'a> {
    name: &'a [u8],
    span: crate::Span,
}

#[derive(Clone, Copy)]
#[octet::sealed_enum]
enum Namespace {
    Input,
    Output,
    Param,
}

pub(super) fn resolve(
    reference: Reference<'_>,
    context: super::Context<'_>,
    expressions: &[crate::Expr],
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    let search = Search {
        name: reference.name,
        span: reference.span,
    };
    if reference.operator == b"def" {
        definitions(context.definitions, search, expressions, meter)
    } else if reference.operator == b"in" {
        bindings(context.inputs, Namespace::Input, search, meter)
    } else if reference.operator == b"out" {
        bindings(context.outputs, Namespace::Output, search, meter)
    } else {
        bindings(context.params, Namespace::Param, search, meter)
    }
}

fn definitions(
    definitions: &[crate::LogicDef],
    search: Search<'_>,
    expressions: &[crate::Expr],
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    let mut at = 0usize;
    let mut found = Ok(None);
    while at < definitions.len() {
        found = definition(definitions, at, search, expressions, meter);
        if !matches!(found, Ok(None)) {
            break;
        }
        at += 1;
    }
    match attempt!(found) {
        Some(expr) => Ok(expr),
        None => Err(crate::invalid(
            search.span,
            "unknown or forward logical definition; recursive definitions are not allowed",
        )),
    }
}

fn bindings(
    bindings: &[crate::NamedType],
    namespace: Namespace,
    search: Search<'_>,
    meter: &mut crate::Meter,
) -> Result<crate::Expr, crate::Diagnostic> {
    let mut at = 0usize;
    let mut found = Ok(None);
    while at < bindings.len() {
        found = binding(bindings, at, namespace, search, meter);
        if !matches!(found, Ok(None)) {
            break;
        }
        at += 1;
    }
    match attempt!(found) {
        Some(expr) => Ok(expr),
        None => Err(crate::invalid(
            search.span,
            "unknown name in the explicitly selected binding namespace",
        )),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; metered name and body lookups return diagnostics and clone an owned structural type; unmatched names remain ordinary search misses."
)]
fn definition(
    definitions: &[crate::LogicDef],
    at: usize,
    search: Search<'_>,
    expressions: &[crate::Expr],
    meter: &mut crate::Meter,
) -> Result<Option<crate::Expr>, crate::Diagnostic> {
    attempt!(meter.charge(
        attempt!(crate::index(search.name.len(), search.span)).saturating_add(1),
        search.span
    ));
    let definition = match definitions.get(at) {
        Some(definition) => definition,
        None => return Err(crate::internal(search.span)),
    };
    if definition.name.as_bytes() != search.name {
        return Ok(None);
    }
    let body = attempt!(super::get(expressions, definition.body, search.span));
    Ok(Some(crate::Expr {
        kind: crate::ExprKind::Definition(attempt!(crate::index(at, search.span))),
        ty: definition.ty.clone(),
        span: search.span,
        total: body.total,
        uses_output: body.uses_output,
    }))
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; metered binding lookup returns diagnostics and clones an owned structural type before namespace dispatch."
)]
fn binding(
    bindings: &[crate::NamedType],
    at: usize,
    namespace: Namespace,
    search: Search<'_>,
    meter: &mut crate::Meter,
) -> Result<Option<crate::Expr>, crate::Diagnostic> {
    attempt!(meter.charge(
        attempt!(crate::index(search.name.len(), search.span)).saturating_add(1),
        search.span
    ));
    let binding = match bindings.get(at) {
        Some(binding) => binding,
        None => return Err(crate::internal(search.span)),
    };
    if binding.name.as_bytes() != search.name {
        return Ok(None);
    }
    let id = attempt!(crate::index(at, search.span));
    // Finish the binding borrow before joining the namespace alternatives.
    let ty = binding.ty.clone();
    let (kind, has_output) = match namespace {
        Namespace::Input => (crate::ExprKind::Input(id), false),
        Namespace::Output => (crate::ExprKind::Output(id), true),
        Namespace::Param => (crate::ExprKind::Param(id), false),
    };
    Ok(Some(crate::Expr {
        kind,
        ty,
        span: search.span,
        total: true,
        uses_output: has_output,
    }))
}
