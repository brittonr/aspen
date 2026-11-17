//! Visibility of definition behavior and exported nominal payloads.

pub(super) fn check(
    def: crate::contracts::Definition,
    node: crate::untrusted::NodeId,
    context: &super::super::parts::Ctx,
) -> Result<(), super::super::Fail> {
    let kind = match context.env.kind(def) {
        Some(kind) => kind,
        None => return Ok(()), // The existing unknown-definition diagnostic follows.
    };
    if visible_behavior(context.env, def, kind) {
        return Ok(());
    }
    Err(super::super::parts::invalid_without_stacks(
        super::super::parts::site(Some(node), Some(def)),
        crate::untrusted::Constraint::PrivateDefinition(def),
    ))
}

/// An unrecognized behavior is private until its visibility rule is defined.
#[expect(
    tigerstyle::missing_const_fn,
    reason = "reads runtime visibility tables through the nominal and generic helpers, which are not const"
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
fn visible_behavior(
    env: &crate::contracts::Env,
    def: crate::contracts::Definition,
    kind: crate::contracts::Behavior,
) -> bool {
    if let Some((id, access)) = nominal_access(kind) {
        return nominal_visible(env, id, access);
    }
    if let Some((id, access)) = generic_access(kind) {
        return generic_visible(env, id, access);
    }
    if matches!(
        kind,
        crate::contracts::Behavior::BoundEmit(_)
            | crate::contracts::Behavior::BoundClock(_)
            | crate::contracts::Behavior::BoundCodeExport(_)
            | crate::contracts::Behavior::BoundSend(_)
    ) {
        return !env.calculator && bound_emit_visible(env, def);
    }
    if kind == crate::contracts::Behavior::TextByte {
        // Standalone cursor, declared text (whose fixed prefix contract
        // validation checks) and the calculator; ordinary modules never.
        return (env.text_cursor || env.calculator) && def.0 == 26;
    }
    if kind == crate::contracts::Behavior::I64Less {
        return env.text_cursor && env.declared_modules && !env.calculator && def.0 == 27;
    }
    if env.has_checked_i64() {
        match kind {
            crate::contracts::Behavior::CheckedI64Add => return def.0 == 23,
            crate::contracts::Behavior::CheckedI64Sub => return def.0 == 24,
            crate::contracts::Behavior::CheckedI64Mul => return def.0 == 25,
            crate::contracts::Behavior::CheckedI64Neg => return def.0 == 26,
            crate::contracts::Behavior::CheckedI64Abs => return def.0 == 27,
            crate::contracts::Behavior::CheckedI64Less => return def.0 == 28,
            crate::contracts::Behavior::CheckedI64DivRem => return def.0 == 29,
            _ => {}
        }
    }
    if env.calculator {
        match kind {
            crate::contracts::Behavior::CalculatorIterate => return def.0 == 27,
            crate::contracts::Behavior::CalculatorCompare => return def.0 == 28,
            crate::contracts::Behavior::CalculatorDivRem => return def.0 == 29,
            crate::contracts::Behavior::CalculatorTextFromAscii => return def.0 == 30,
            crate::contracts::Behavior::CalculatorTextLength => return def.0 == 31,
            _ => {}
        }
    }
    if let crate::contracts::Behavior::TestEmit = kind {
        return env.ambient_test_emit_visible();
    }
    ordinary_visible(kind)
}

const fn generic_access(
    kind: crate::contracts::Behavior,
) -> Option<(crate::types::NominalTypeId, Access)> {
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-kernel; the Behavior enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
    )]
    match kind {
        crate::contracts::Behavior::GenericLeft(id) => Some((id, Access::Left)),
        crate::contracts::Behavior::GenericRight(id) => Some((id, Access::Right)),
        crate::contracts::Behavior::GenericMatch(id) => Some((id, Access::Match)),
        crate::contracts::Behavior::Dup => None,
        crate::contracts::Behavior::Drop => None,
        crate::contracts::Behavior::Swap => None,
        crate::contracts::Behavior::Dip => None,
        crate::contracts::Behavior::Arith => None,
        crate::contracts::Behavior::Equals => None,
        crate::contracts::Behavior::Quote => None,
        crate::contracts::Behavior::Compose => None,
        crate::contracts::Behavior::Run => None,
        crate::contracts::Behavior::Reflect => None,
        crate::contracts::Behavior::Unit => None,
        crate::contracts::Behavior::Pair => None,
        crate::contracts::Behavior::Unpair => None,
        crate::contracts::Behavior::Inl => None,
        crate::contracts::Behavior::Inr => None,
        crate::contracts::Behavior::Case => None,
        crate::contracts::Behavior::If => None,
        crate::contracts::Behavior::Nil => None,
        crate::contracts::Behavior::Cons => None,
        crate::contracts::Behavior::ListCase => None,
        crate::contracts::Behavior::TestEmit => None,
        crate::contracts::Behavior::TextByte => None,
        crate::contracts::Behavior::I64Less => None,
        crate::contracts::Behavior::CheckedI64Add => None,
        crate::contracts::Behavior::CheckedI64Sub => None,
        crate::contracts::Behavior::CheckedI64Mul => None,
        crate::contracts::Behavior::CheckedI64Neg => None,
        crate::contracts::Behavior::CheckedI64Abs => None,
        crate::contracts::Behavior::CheckedI64Less => None,
        crate::contracts::Behavior::CheckedI64DivRem => None,
        crate::contracts::Behavior::CalculatorIterate => None,
        crate::contracts::Behavior::CalculatorCompare => None,
        crate::contracts::Behavior::CalculatorDivRem => None,
        crate::contracts::Behavior::CalculatorTextFromAscii => None,
        crate::contracts::Behavior::CalculatorTextLength => None,
        crate::contracts::Behavior::Reserved => None,
        crate::contracts::Behavior::BoundEmit(_) => None,
        crate::contracts::Behavior::BoundClock(_) => None,
        crate::contracts::Behavior::BoundCodeExport(_) => None,
        crate::contracts::Behavior::BoundSend(_) => None,
        crate::contracts::Behavior::NominalNew(_) => None,
        crate::contracts::Behavior::NominalInto(_) => None,
        crate::contracts::Behavior::NominalLeft(_) => None,
        crate::contracts::Behavior::NominalRight(_) => None,
        crate::contracts::Behavior::NominalMatch(_) => None,
        crate::contracts::Behavior::Named => None,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "reads the runtime generic-variant table, which is not const"
)]
fn generic_visible(
    env: &crate::contracts::Env,
    id: crate::types::NominalTypeId,
    access: Access,
) -> bool {
    if caller_is_owner(env.caller_module, id.module) {
        return env.generic_variant(id).is_some();
    }
    let Some(decl) = env.generic_variant(id) else {
        return false;
    };
    if !decl.exported {
        return false;
    }
    match access {
        Access::Left => decl.public[0],
        Access::Right => decl.public[1],
        Access::Match => decl.public[0] && decl.public[1],
        Access::New | Access::Into => false,
    }
}

/// A missing owner or out-of-range identity can never grant access.
fn bound_emit_visible(env: &crate::contracts::Env, def: crate::contracts::Definition) -> bool {
    let Ok(index) = usize::try_from(def.0) else {
        return false;
    };
    let Some(Some(owner)) = env.definition_owners.get(index) else {
        return false;
    };
    caller_is_owner(env.caller_module, *owner)
}

const fn caller_is_owner(caller_module: Option<u64>, owner_module: u64) -> bool {
    if let Some(caller_module) = caller_module {
        return caller_module == owner_module;
    }
    false
}

const fn ordinary_visible(kind: crate::contracts::Behavior) -> bool {
    stack_behavior_visible(kind) || aggregate_behavior_visible(kind)
}

const fn stack_behavior_visible(kind: crate::contracts::Behavior) -> bool {
    matches!(
        kind,
        crate::contracts::Behavior::Dup
            | crate::contracts::Behavior::Drop
            | crate::contracts::Behavior::Swap
            | crate::contracts::Behavior::Dip
            | crate::contracts::Behavior::Arith
            | crate::contracts::Behavior::Equals
            | crate::contracts::Behavior::Quote
            | crate::contracts::Behavior::Compose
            | crate::contracts::Behavior::Run
            | crate::contracts::Behavior::Reflect
    )
}

const fn aggregate_behavior_visible(kind: crate::contracts::Behavior) -> bool {
    matches!(
        kind,
        crate::contracts::Behavior::Named
            | crate::contracts::Behavior::Unit
            | crate::contracts::Behavior::Pair
            | crate::contracts::Behavior::Unpair
            | crate::contracts::Behavior::Inl
            | crate::contracts::Behavior::Inr
            | crate::contracts::Behavior::Case
            | crate::contracts::Behavior::If
            | crate::contracts::Behavior::Nil
            | crate::contracts::Behavior::Cons
            | crate::contracts::Behavior::ListCase
    )
}

/// Which public operation's payload(s) may cross a module boundary.
#[octet::sealed_enum]
#[derive(Clone, Copy)]
enum Access {
    New,
    Into,
    Left,
    Right,
    Match,
}

const fn nominal_access(
    kind: crate::contracts::Behavior,
) -> Option<(crate::types::NominalTypeId, Access)> {
    if let crate::contracts::Behavior::NominalNew(id) = kind {
        return Some((id, Access::New));
    }
    if let crate::contracts::Behavior::NominalInto(id) = kind {
        return Some((id, Access::Into));
    }
    if let crate::contracts::Behavior::NominalLeft(id) = kind {
        return Some((id, Access::Left));
    }
    if let crate::contracts::Behavior::NominalRight(id) = kind {
        return Some((id, Access::Right));
    }
    if let crate::contracts::Behavior::NominalMatch(id) = kind {
        return Some((id, Access::Match));
    }
    None
}

fn nominal_visible(
    env: &crate::contracts::Env,
    id: crate::types::NominalTypeId,
    access: Access,
) -> bool {
    if caller_is_owner(env.caller_module, id.module) {
        return true;
    }
    let mut index = 0;
    let mut is_visible = false;
    while index < env.nominals.len() {
        let decl = &env.nominals[index];
        if decl.id == id {
            is_visible = public_operation_visible(env, decl, access);
            break;
        }
        index += 1;
    }
    is_visible
}

fn public_operation_visible(
    env: &crate::contracts::Env,
    decl: &crate::contracts::NominalDecl,
    access: Access,
) -> bool {
    let Some((first, second)) = public_payloads(decl, access) else {
        return false;
    };
    env.public_payload(first, 512) && second.is_none_or(|other| env.public_payload(other, 512))
}

type Payloads<'a> = (
    &'a alloc::boxed::Box<crate::types::Ty>,
    Option<&'a alloc::boxed::Box<crate::types::Ty>>,
);

/// Exported shape and operation determine exactly which payloads must be public.
const fn public_payloads(
    decl: &crate::contracts::NominalDecl,
    access: Access,
) -> Option<Payloads<'_>> {
    if !decl.exported {
        return None;
    }
    if let crate::types::NominalShape::Opaque(ty) = &decl.shape {
        return opaque_payload(ty, decl.public, access);
    }
    if let crate::types::NominalShape::Variant(left, right) = &decl.shape {
        return variant_payloads(left, right, decl.public, access);
    }
    None
}

const fn opaque_payload<'a>(
    ty: &'a alloc::boxed::Box<crate::types::Ty>,
    public: [bool; 2],
    access: Access,
) -> Option<Payloads<'a>> {
    match access {
        Access::New if public[0] => Some((ty, None)),
        Access::Into if public[1] => Some((ty, None)),
        Access::New | Access::Into | Access::Left | Access::Right | Access::Match => None,
    }
}

const fn variant_payloads<'a>(
    left: &'a alloc::boxed::Box<crate::types::Ty>,
    right: &'a alloc::boxed::Box<crate::types::Ty>,
    public: [bool; 2],
    access: Access,
) -> Option<Payloads<'a>> {
    match access {
        Access::Left if public[0] => Some((left, None)),
        Access::Right if public[1] => Some((right, None)),
        Access::Match if public[0] && public[1] => Some((left, Some(right))),
        Access::New | Access::Into | Access::Left | Access::Right | Access::Match => None,
    }
}
