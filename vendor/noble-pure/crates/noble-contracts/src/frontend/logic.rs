pub(super) struct Resolved {
    pub params: alloc::vec::Vec<crate::NamedType>,
    pub definitions: alloc::vec::Vec<crate::LogicDef>,
    pub expressions: alloc::vec::Vec<crate::Expr>,
    pub requires: u32,
    pub ensures: u32,
}

#[derive(Clone, Copy)]
struct Declarations<'a> {
    ids: &'a [u32],
    span: crate::Span,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; contract kind, definitions, and Boolean pre/postconditions are validated through typed diagnostics rather than assertions."
)]
pub(super) fn resolve(
    context: crate::predicate::Context<'_>,
    parts: super::fields::Parts,
    meter: &mut crate::Meter,
) -> Result<Resolved, crate::Diagnostic> {
    let span = attempt!(context.tree.node(context.tree.root)).span;
    if let Some(error) = parts.failure {
        return Err(error);
    }
    attempt!(kind(context, parts.kind));
    let params = match parts.params {
        Some(root) => attempt!(super::bindings::named(
            attempt!(super::Container::new(context.source, context.tree, root)),
            meter
        )),
        None => alloc::vec::Vec::new(),
    };
    let context = crate::predicate::Context {
        params: &params,
        ..context
    };
    let (definitions, mut arena) = attempt!(definitions(
        context,
        Declarations {
            ids: &parts.definitions,
            span
        },
        meter
    ));
    let context = crate::predicate::Context {
        definitions: &definitions,
        ..context
    };
    let requires = attempt!(precondition(
        context,
        parts.requires,
        span,
        &mut arena,
        meter
    ));
    let ensures_syntax = attempt!(super::argument(
        context.tree,
        attempt!(super::required(
            parts.ensures,
            span,
            "missing ensures field"
        ))
    ));
    let ensures = attempt!(crate::predicate::resolving::resolve(
        context,
        ensures_syntax,
        &mut arena,
        meter
    ));
    let postcondition = attempt!(crate::predicate::get(&arena.expressions, ensures, span));
    attempt!(crate::predicate::same(
        &postcondition.ty,
        &noble_kernel::types::Ty::Bool,
        postcondition.span,
        meter
    ));
    Ok(Resolved {
        params,
        definitions,
        expressions: arena.expressions,
        requires,
        ensures,
    })
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; predicate type and transitive output dependence are user-facing validation errors, never panic preconditions."
)]
fn precondition(
    context: crate::predicate::Context<'_>,
    field: Option<u32>,
    span: crate::Span,
    arena: &mut crate::predicate::Arena,
    meter: &mut crate::Meter,
) -> Result<u32, crate::Diagnostic> {
    let requires_syntax = attempt!(super::argument(
        context.tree,
        attempt!(super::required(field, span, "missing requires field"))
    ));
    let requires = attempt!(crate::predicate::resolving::resolve(
        context,
        requires_syntax,
        arena,
        meter
    ));
    let precondition = attempt!(crate::predicate::get(&arena.expressions, requires, span));
    attempt!(crate::predicate::same(
        &precondition.ty,
        &noble_kernel::types::Ty::Bool,
        precondition.span,
        meter
    ));
    if precondition.uses_output() {
        return Err(crate::invalid(
            precondition.span,
            "requires cannot depend on final output bindings, including through definitions",
        ));
    }
    Ok(requires)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; syntax lookup and unsupported-property failures construct owned diagnostics."
)]
fn kind(
    context: crate::predicate::Context<'_>,
    kind: Option<u32>,
) -> Result<(), crate::Diagnostic> {
    let id = match kind {
        Some(kind) => attempt!(super::argument(context.tree, kind)),
        None => return Ok(()),
    };
    if attempt!(context.tree.atom(context.source, id)) != b"partial-correctness" {
        return Err(crate::Diagnostic::new(crate::DiagnosticKind::Unsupported, attempt!(context.tree.node(id)).span, "only partial-correctness is supported; total-correctness and prefix-safety are not reinterpreted"));
    }
    Ok(())
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; sequential resolution exposes only preceding definitions and propagates the first typed failure."
)]
fn definitions(
    context: crate::predicate::Context<'_>,
    declarations: Declarations<'_>,
    meter: &mut crate::Meter,
) -> Result<(alloc::vec::Vec<crate::LogicDef>, crate::predicate::Arena), crate::Diagnostic> {
    let mut definitions = alloc::vec::Vec::with_capacity(declarations.ids.len());
    let mut arena = crate::predicate::Arena::new();
    let mut at = 0usize;
    let mut failure = None;
    while at < declarations.ids.len() {
        let context = crate::predicate::Context {
            definitions: &definitions,
            ..context
        };
        match definition(context, declarations, at, &mut arena, meter) {
            Ok(definition) => definitions.push(definition),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
        at += 1;
    }
    match failure {
        Some(error) => Err(error),
        None => Ok((definitions, arena)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; arity, freshness, declared type, and totality checks reject malformed definitions with diagnostics."
)]
fn definition(
    context: crate::predicate::Context<'_>,
    declarations: Declarations<'_>,
    at: usize,
    arena: &mut crate::predicate::Arena,
    meter: &mut crate::Meter,
) -> Result<crate::LogicDef, crate::Diagnostic> {
    attempt!(meter.charge(1, declarations.span));
    let id = attempt!(crate::syntax::child(
        declarations.ids,
        at,
        declarations.span
    ));
    let at_span = attempt!(context.tree.node(id)).span;
    let items = attempt!(context.tree.round(id));
    if items.len() != 4 {
        return Err(crate::invalid(
            at_span,
            "define requires a name, type, and expression",
        ));
    }
    let name = attempt!(crate::syntax::identifier(
        context.source,
        context.tree,
        attempt!(crate::syntax::child(items, 1, at_span)),
        meter
    ));
    let mut previous = 0usize;
    let mut failure = None;
    while previous < context.definitions.len() {
        if let Err(error) = fresh(context.definitions, previous, &name, at_span, meter) {
            failure = Some(error);
            break;
        }
        previous += 1;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    let ty = attempt!(crate::syntax::ty(
        context.source,
        context.tree,
        attempt!(crate::syntax::child(items, 2, at_span)),
        meter
    ));
    let body = attempt!(crate::predicate::resolving::resolve(
        context,
        attempt!(crate::syntax::child(items, 3, at_span)),
        arena,
        meter
    ));
    let expr = attempt!(crate::predicate::get(&arena.expressions, body, at_span));
    attempt!(crate::predicate::same(&ty, &expr.ty, at_span, meter));
    if !expr.is_total() {
        return Err(crate::invalid(expr.span, "logical definitions must be total; partial projections belong in explicitly guarded predicates"));
    }
    Ok(crate::LogicDef { name, ty, body })
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; duplicate-definition and metering failures construct owned diagnostics through non-const helpers."
)]
fn fresh(
    definitions: &[crate::LogicDef],
    at: usize,
    name: &str,
    span: crate::Span,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(
        attempt!(crate::index(name.len(), span)).saturating_add(1),
        span
    ));
    match definitions.get(at) {
        Some(definition) => {
            if definition.name.as_str() == name {
                Err(crate::invalid(span, "logical definition is rebound"))
            } else {
                Ok(())
            }
        }
        None => Err(crate::internal(span)),
    }
}
