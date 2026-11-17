//! Instantiation of one scheme into a concrete interface, with the
//! eligibility and effect-identity checks the fragment requires.

/// The literal scheme: `S -- S <literal type>`.
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; every Lit constructor requires its own result type in the literal scheme; new literals must fail compilation until that semantic mapping is supplied."
)]
pub(crate) fn literal_scheme(lit: crate::untrusted::Lit) -> crate::words::Scheme {
    let pattern = match lit {
        crate::untrusted::Lit::I64(_) => crate::shapes::Pattern::I64,
        crate::untrusted::Lit::Bool(_) => crate::shapes::Pattern::Bool,
        crate::untrusted::Lit::Text => crate::shapes::Pattern::Text,
        crate::untrusted::Lit::Unit => crate::shapes::Pattern::Unit,
    };
    crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![crate::shapes::Pattern::StackVar(crate::words::Variable(0))],
        stack_out: alloc::vec![
            crate::shapes::Pattern::StackVar(crate::words::Variable(0)),
            pattern,
        ],
        effects: alloc::vec![],
    }
}

/// The quotation scheme: `R -- R Program<A, C, e>`.
pub(crate) fn quotation_scheme() -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds: alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect
        ],
        stack_in: alloc::vec![crate::shapes::Pattern::StackVar(crate::words::Variable(0))],
        stack_out: alloc::vec![
            crate::shapes::Pattern::StackVar(crate::words::Variable(0)),
            crate::shapes::Pattern::program(
                alloc::vec![crate::shapes::Pattern::StackVar(crate::words::Variable(1))],
                alloc::vec![crate::shapes::Pattern::StackVar(crate::words::Variable(2))],
                alloc::vec![crate::shapes::EffectSlot::Var(crate::words::Variable(3))],
            ),
        ],
        effects: alloc::vec![],
    }
}

/// The value slot that requires `Data`, for behaviors that constrain one.
pub(crate) const fn data_slot(
    behavior: Option<crate::contracts::Behavior>,
) -> Option<crate::words::Variable> {
    match behavior {
        Some(crate::contracts::Behavior::Dup)
        | Some(crate::contracts::Behavior::Drop)
        | Some(crate::contracts::Behavior::Quote) => Some(crate::words::Variable(1)),
        Some(crate::contracts::Behavior::Swap)
        | Some(crate::contracts::Behavior::Dip)
        | Some(crate::contracts::Behavior::Arith)
        | Some(crate::contracts::Behavior::Equals)
        | Some(crate::contracts::Behavior::Compose)
        | Some(crate::contracts::Behavior::Run)
        | Some(crate::contracts::Behavior::Reflect)
        | Some(crate::contracts::Behavior::Unit)
        | Some(crate::contracts::Behavior::Pair)
        | Some(crate::contracts::Behavior::Unpair)
        | Some(crate::contracts::Behavior::Inl)
        | Some(crate::contracts::Behavior::Inr)
        | Some(crate::contracts::Behavior::Case)
        | Some(crate::contracts::Behavior::If)
        | Some(crate::contracts::Behavior::Nil)
        | Some(crate::contracts::Behavior::Cons)
        | Some(crate::contracts::Behavior::ListCase)
        | Some(crate::contracts::Behavior::TestEmit)
        | Some(crate::contracts::Behavior::TextByte)
        | Some(crate::contracts::Behavior::I64Less)
        | Some(crate::contracts::Behavior::CheckedI64Add)
        | Some(crate::contracts::Behavior::CheckedI64Sub)
        | Some(crate::contracts::Behavior::CheckedI64Mul)
        | Some(crate::contracts::Behavior::CheckedI64Neg)
        | Some(crate::contracts::Behavior::CheckedI64Abs)
        | Some(crate::contracts::Behavior::CheckedI64Less)
        | Some(crate::contracts::Behavior::CheckedI64DivRem)
        | Some(crate::contracts::Behavior::CalculatorIterate)
        | Some(crate::contracts::Behavior::CalculatorCompare)
        | Some(crate::contracts::Behavior::CalculatorDivRem)
        | Some(crate::contracts::Behavior::CalculatorTextFromAscii)
        | Some(crate::contracts::Behavior::CalculatorTextLength)
        | Some(crate::contracts::Behavior::Reserved)
        | Some(crate::contracts::Behavior::BoundEmit(_))
        | Some(crate::contracts::Behavior::BoundClock(_))
        | Some(crate::contracts::Behavior::BoundCodeExport(_))
        | Some(crate::contracts::Behavior::BoundSend(_))
        | Some(crate::contracts::Behavior::NominalNew(_))
        | Some(crate::contracts::Behavior::NominalInto(_))
        | Some(crate::contracts::Behavior::NominalLeft(_))
        | Some(crate::contracts::Behavior::NominalRight(_))
        | Some(crate::contracts::Behavior::NominalMatch(_))
        | Some(crate::contracts::Behavior::GenericLeft(_))
        | Some(crate::contracts::Behavior::GenericRight(_))
        | Some(crate::contracts::Behavior::GenericMatch(_))
        | Some(crate::contracts::Behavior::Named)
        | None => None,
    }
}

/// Instantiate one scheme into a concrete interface, validating the witness.
///
/// The witness's reference bindings are resolved first (B-CHECK-05): a
/// cyclic chain rejects as invalid, a chain that would outrun the declared
/// work limit rejects fail-closed, and the resolved witness returns with
/// the interface so callers can read its segments directly.
#[expect(
    tigerstyle::too_many_parameters,
    reason = "Owner: noble-maintainers; the parameters are the kernel-internal instantiation shape (scheme, witness, Data slot, failing site, context, remaining budget) and bundling them into a struct would change the extracted subject for a naming-discipline lint."
)]
pub(crate) fn apply(
    scheme: &crate::words::Scheme,
    inst: &crate::words::Inst,
    data_var: Option<crate::words::Variable>,
    at: super::Site,
    ctx: &super::Ctx,
    fuel: u32,
) -> Result<(crate::untrusted::Interface, crate::words::Inst, u32), super::super::Fail> {
    if !valid_bindings(inst.bindings.as_slice(), ctx.env) {
        return Err(super::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::InvalidType,
        ));
    }
    attempt!(check_bounds(scheme, inst, at, ctx));
    let (resolved, spent) = attempt!(resolve_witness(scheme, inst, at, fuel));
    let interface = attempt!(project(scheme, &resolved, data_var, at, ctx));
    Ok((interface, resolved, spent))
}

/// Reject malformed types and unknown effect identities before resolving refs.
fn valid_bindings(bindings: &[crate::words::Binding], env: &crate::contracts::Env) -> bool {
    let mut index = 0;
    let mut is_every_binding_valid = true;
    while index < bindings.len() {
        let is_valid = match &bindings[index] {
            crate::words::Binding::Value(ty) => env.valid_type(ty, 512),
            crate::words::Binding::Stack(stack) => super::valid_stack_types(stack, env),
            crate::words::Binding::Effect(effects) => valid_effect_bindings(effects, env),
            crate::words::Binding::Ref(_) => true,
        };
        if !is_valid {
            is_every_binding_valid = false;
            break;
        }
        index += 1;
    }
    is_every_binding_valid
}

fn valid_effect_bindings(effects: &crate::types::EffSet, env: &crate::contracts::Env) -> bool {
    let ids = effects.as_slice();
    let mut index = 0;
    let mut is_every_effect_known = true;
    while index < ids.len() {
        if !env.knows_effect(ids[index]) {
            is_every_effect_known = false;
            break;
        }
        index += 1;
    }
    is_every_effect_known
}

/// Resolve one witness's reference bindings inside the remaining work budget.
///
/// Each hop spends one unit of `fuel`, so a chain longer than the budget the
/// caller still holds refuses before taking the hop. The spent amount returns
/// to the caller, which charges it against that same budget.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject const Vec dereference, runtime resolve::bindings, and the owned Result destructor (E0015/E0277/E0493/E0658); reassess when those operations become const without changing typed witness failures."
)]
fn resolve_witness(
    scheme: &crate::words::Scheme,
    inst: &crate::words::Inst,
    at: super::Site,
    fuel: u32,
) -> Result<(crate::words::Inst, u32), super::super::Fail> {
    match crate::words::resolve::bindings(&scheme.var_kinds, inst, fuel) {
        Ok((resolved, spent)) => Ok((resolved, spent)),
        Err(crate::words::InstError::CyclicWitness) => Err(super::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::CyclicWitness,
        )),
        Err(crate::words::InstError::WalkExhausted) => Err(super::super::Fail::Exhausted(
            crate::untrusted::LimitKind::Work,
        )),
        Err(crate::words::InstError::ArityMismatch) => Err(super::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::InstantiationArity,
        )),
        Err(crate::words::InstError::OversizedStack) => Err(super::super::Fail::Exhausted(
            crate::untrusted::LimitKind::StackHeight,
        )),
        Err(crate::words::InstError::OversizedType)
        | Err(crate::words::InstError::OversizedEffects) => Err(super::super::Fail::Exhausted(
            crate::untrusted::LimitKind::TypeSize,
        )),
        Err(crate::words::InstError::KindMismatch)
        | Err(crate::words::InstError::UnknownVariable) => Err(instantiation_invalid(at)),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; check_bounds maps witness kind, arity and resource-limit failures to the acceptance protocol's typed outcomes; malformed external witnesses must never be asserted valid."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject non-const Env::effect_count and Scheme::check_inst (E0015); reassess when these bounded checks become const without changing typed rejection order."
)]
fn check_bounds(
    scheme: &crate::words::Scheme,
    inst: &crate::words::Inst,
    at: super::Site,
    ctx: &super::Ctx,
) -> Result<(), super::super::Fail> {
    let max_effects = match ctx.env.effect_count() {
        Some(count) => count,
        None => {
            return Err(super::super::Fail::Unsupported(
                crate::untrusted::UnsupportedKind::SchemeForm,
            ))
        }
    };
    match scheme.check_inst(
        inst,
        ctx.request.limits.stack_height,
        ctx.request.limits.type_size,
        max_effects,
    ) {
        Ok(()) => Ok(()),
        Err(crate::words::InstError::KindMismatch)
        | Err(crate::words::InstError::UnknownVariable)
        | Err(crate::words::InstError::CyclicWitness)
        | Err(crate::words::InstError::WalkExhausted) => Err(instantiation_invalid(at)),
        Err(crate::words::InstError::ArityMismatch) => Err(super::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::InstantiationArity,
        )),
        Err(crate::words::InstError::OversizedStack) => Err(super::super::Fail::Exhausted(
            crate::untrusted::LimitKind::StackHeight,
        )),
        Err(crate::words::InstError::OversizedType)
        | Err(crate::words::InstError::OversizedEffects) => Err(super::super::Fail::Exhausted(
            crate::untrusted::LimitKind::TypeSize,
        )),
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; project validates substitutions, stack limits, effect identities and Data eligibility in that order and rejects through typed diagnostics rather than panics."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; both pinned Rust compilers reject runtime Inst::value, Ty::clone, Vec dereference, substitutions and owned Result destruction (E0015/E0277/E0493/E0658); reassess when they become const without changing typed validation."
)]
fn project(
    scheme: &crate::words::Scheme,
    inst: &crate::words::Inst,
    data_var: Option<crate::words::Variable>,
    at: super::Site,
    ctx: &super::Ctx,
) -> Result<crate::untrusted::Interface, super::super::Fail> {
    // Read the eligible binding as an owned value first: a reference into the
    // instantiation cannot be carried across the substitutions below.
    let eligible: Option<crate::types::Ty> = match data_var {
        Some(var) => match inst.value(var) {
            Some(ty) => Some(ty.clone()),
            None => return Err(instantiation_invalid(at)),
        },
        None => None,
    };
    let stack_in = match scheme.subst_stack(&scheme.stack_in, inst) {
        Ok(stack) => stack,
        Err(problem) => return Err(substitution_failure(at, problem)),
    };
    let stack_out = match scheme.subst_stack(&scheme.stack_out, inst) {
        Ok(stack) => stack,
        Err(problem) => return Err(substitution_failure(at, problem)),
    };
    let effects = match scheme.subst_effects(&scheme.effects, inst) {
        Ok(effects) => effects,
        Err(_) => return Err(instantiation_invalid(at)),
    };
    attempt!(super::limits_of(&stack_in, ctx));
    attempt!(super::limits_of(&stack_out, ctx));
    let unknown = super::effects::first_unknown(effects.as_slice(), &ctx.env.effects);
    if let Some(id) = unknown {
        return Err(super::invalid_without_stacks(
            at,
            crate::untrusted::Constraint::UnknownEffect(id),
        ));
    }
    if let Some(ty) = eligible {
        if !ty.is_data() {
            return Err(super::invalid_without_stacks(
                at,
                crate::untrusted::Constraint::Eligibility(ty),
            ));
        }
    }
    Ok(crate::untrusted::Interface {
        stack_in,
        stack_out,
        effects,
    })
}

const fn instantiation_invalid(at: super::Site) -> super::super::Fail {
    super::invalid_without_stacks(at, crate::untrusted::Constraint::InstantiationKind)
}

/// Report one substitution failure with the refusal it names.
///
/// A substitution that exceeds the local type-size bound is the same refusal
/// `limits_of` would report for the resulting interface, so it keeps that
/// identity even though the oversized value is now refused before it exists.
#[expect(
    tigerstyle::fragile_exhaustive_enum_match,
    reason = "Owner: noble-maintainers; every InstError maps explicitly to its acceptance refusal, and a new substitution error must be classified rather than inherit a fallback."
)]
const fn substitution_failure(
    at: super::Site,
    problem: crate::words::InstError,
) -> super::super::Fail {
    match problem {
        crate::words::InstError::OversizedType => {
            super::super::Fail::Exhausted(crate::untrusted::LimitKind::TypeSize)
        }
        crate::words::InstError::OversizedStack => {
            super::super::Fail::Exhausted(crate::untrusted::LimitKind::StackHeight)
        }
        crate::words::InstError::OversizedEffects => {
            super::super::Fail::Exhausted(crate::untrusted::LimitKind::TypeSize)
        }
        crate::words::InstError::KindMismatch
        | crate::words::InstError::UnknownVariable
        | crate::words::InstError::ArityMismatch
        | crate::words::InstError::CyclicWitness
        | crate::words::InstError::WalkExhausted => instantiation_invalid(at),
    }
}
