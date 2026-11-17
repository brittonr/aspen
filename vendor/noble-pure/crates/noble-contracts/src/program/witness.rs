pub(super) struct Explicit<'a> {
    pub inst: &'a noble_kernel::words::Inst,
    pub kinds: &'a [noble_kernel::words::VariableKind],
    pub variables: &'a [crate::inference::Variable],
    pub span: crate::Span,
}

#[derive(Clone, Copy)]
struct Bindings<'a> {
    values: &'a [noble_kernel::words::Binding],
    variables: &'a [crate::inference::Variable],
    span: crate::Span,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; witness arity, cycle/kind resolution, metering, and each inferred binding are checked through typed errors."
)]
pub(super) fn apply(
    explicit: Explicit<'_>,
    arena: &mut crate::inference::Arena,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    if explicit.inst.bindings.len() != explicit.kinds.len() {
        return Err(crate::invalid(
            explicit.span,
            "explicit word witness has the wrong number of bindings",
        ));
    }
    let resolved = match noble_kernel::words::resolve::bindings(
        explicit.kinds,
        explicit.inst,
        meter.available(),
    ) {
        Ok((resolved, spent)) => {
            attempt!(meter.charge(spent, explicit.span));
            resolved
        }
        Err(noble_kernel::words::InstError::WalkExhausted) => {
            return Err(crate::Diagnostic::new(
                crate::DiagnosticKind::Exhausted,
                explicit.span,
                "explicit witness resolution exhausted its work limit",
            ))
        }
        Err(_) => {
            return Err(crate::invalid(
                explicit.span,
                "explicit word witness has a cycle, unknown reference, or wrong binding kind",
            ))
        }
    };
    let bindings = Bindings {
        values: &resolved.bindings,
        variables: explicit.variables,
        span: explicit.span,
    };
    let mut at = 0usize;
    let mut failure = None;
    while at < bindings.values.len() {
        match apply_binding(bindings, at, arena, meter) {
            Ok(()) => at += 1,
            Err(problem) => {
                failure = Some(problem);
                break;
            }
        }
    }
    match failure {
        Some(problem) => Err(problem),
        None => Ok(()),
    }
}

#[expect(
    tigerstyle::assertion_density,
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; binding kinds and pure effects are validated fallibly while structural type construction and unification allocate."
)]
fn apply_binding(
    bindings: Bindings<'_>,
    at: usize,
    arena: &mut crate::inference::Arena,
    meter: &mut crate::Meter,
) -> Result<(), crate::Diagnostic> {
    attempt!(meter.charge(1, bindings.span));
    let variable = attempt!(crate::inference::variable_at(
        bindings.variables,
        attempt!(crate::index(at, bindings.span)),
        bindings.span
    ));
    match (variable, bindings.values.get(at)) {
        (
            crate::inference::Variable::Value(target),
            Some(noble_kernel::words::Binding::Value(ty)),
        ) => {
            let value = attempt!(arena.ty(ty, bindings.span, meter));
            arena.unify(target, value, bindings.span, meter)
        }
        (
            crate::inference::Variable::Stack(target),
            Some(noble_kernel::words::Binding::Stack(stack)),
        ) => {
            let value = attempt!(arena.stack(stack, bindings.span, meter));
            arena.unify(target, value, bindings.span, meter)
        }
        (
            crate::inference::Variable::Effect,
            Some(noble_kernel::words::Binding::Effect(effects)),
        ) => {
            if !effects.is_empty() {
                Err(crate::Diagnostic::new(
                    crate::DiagnosticKind::Unsupported,
                    bindings.span,
                    "effectful word witnesses are outside the pure fragment",
                ))
            } else {
                Ok(())
            }
        }
        _ => Err(crate::invalid(
            bindings.span,
            "explicit word witness has the wrong binding kind",
        )),
    }
}
