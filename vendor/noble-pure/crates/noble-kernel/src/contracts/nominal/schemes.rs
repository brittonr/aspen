//! Canonical typed schemes for declared nominal operations.

macro_rules! present {
    ($candidate:expr) => {
        match $candidate {
            Some(value) => value,
            None => return None,
        }
    };
}

fn stack(index: u32) -> crate::shapes::Pattern {
    crate::shapes::Pattern::StackVar(crate::words::Variable(index))
}

fn declared_pattern(decl: &super::super::NominalDecl) -> crate::shapes::Pattern {
    crate::shapes::Pattern::Nominal(decl.id, alloc::boxed::Box::new(decl.shape.clone()))
}

pub(super) fn conversion(
    env: &super::super::Env,
    decl: &super::super::NominalDecl,
    into: bool,
) -> Option<crate::words::Scheme> {
    let crate::types::NominalShape::Opaque(ty) = &decl.shape else {
        return None;
    };
    let representation = present!(super::payload::pattern(env, ty));
    if into {
        return Some(conversion_scheme(declared_pattern(decl), representation));
    }
    Some(conversion_scheme(representation, declared_pattern(decl)))
}

fn conversion_scheme(
    input: crate::shapes::Pattern,
    output: crate::shapes::Pattern,
) -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![stack(0), input],
        stack_out: alloc::vec![stack(0), output],
        effects: alloc::vec![],
    }
}

pub(super) fn arm(
    env: &super::super::Env,
    decl: &super::super::NominalDecl,
    right: bool,
) -> Option<crate::words::Scheme> {
    let crate::types::NominalShape::Variant(left, other) = &decl.shape else {
        return None;
    };
    let representation = if right {
        super::payload::pattern(env, other)
    } else {
        super::payload::pattern(env, left)
    };
    let representation = present!(representation);
    Some(crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![stack(0), representation],
        stack_out: alloc::vec![stack(0), declared_pattern(decl)],
        effects: alloc::vec![],
    })
}

pub(super) fn matcher(
    env: &super::super::Env,
    decl: &super::super::NominalDecl,
) -> Option<crate::words::Scheme> {
    let crate::types::NominalShape::Variant(left, right) = &decl.shape else {
        return None;
    };
    let left = present!(super::payload::pattern(env, left));
    let right = present!(super::payload::pattern(env, right));
    Some(crate::words::Scheme {
        var_kinds: match_variables(),
        stack_in: match_input(decl, left, right),
        stack_out: alloc::vec![stack(1)],
        effects: alloc::vec![
            crate::shapes::EffectSlot::Var(crate::words::Variable(2)),
            crate::shapes::EffectSlot::Var(crate::words::Variable(3)),
        ],
    })
}

fn match_variables() -> alloc::vec::Vec<crate::words::VariableKind> {
    alloc::vec![
        crate::words::VariableKind::Stack,
        crate::words::VariableKind::Stack,
        crate::words::VariableKind::Effect,
        crate::words::VariableKind::Effect,
    ]
}

fn match_input(
    decl: &super::super::NominalDecl,
    left: crate::shapes::Pattern,
    right: crate::shapes::Pattern,
) -> alloc::vec::Vec<crate::shapes::Pattern> {
    alloc::vec![
        stack(0),
        declared_pattern(decl),
        match_handler(left, 2),
        match_handler(right, 3),
    ]
}

fn match_handler(arm: crate::shapes::Pattern, effect: u32) -> crate::shapes::Pattern {
    crate::shapes::Pattern::program(
        alloc::vec![stack(0), arm],
        alloc::vec![stack(1)],
        alloc::vec![crate::shapes::EffectSlot::Var(crate::words::Variable(
            effect
        ))],
    )
}

fn generic_vars() -> alloc::vec::Vec<crate::words::VariableKind> {
    alloc::vec![
        crate::words::VariableKind::Stack,
        crate::words::VariableKind::Value,
        crate::words::VariableKind::Value,
    ]
}

fn generic_pattern(decl: &super::super::GenericVariantDecl) -> crate::shapes::Pattern {
    crate::shapes::Pattern::GenericNominal(
        decl.id,
        alloc::boxed::Box::new([
            crate::shapes::Pattern::Var(crate::words::Variable(1)),
            crate::shapes::Pattern::Var(crate::words::Variable(2)),
        ]),
        decl.payload_params,
    )
}

pub(super) fn generic_arm(
    decl: &super::super::GenericVariantDecl,
    right: bool,
) -> crate::words::Scheme {
    // Payload parameters are u8; widening makes the increment exact even for
    // malformed values outside the declared [0, 1] mapping.
    let arm = u32::from(decl.payload_params[usize::from(right)]).saturating_add(1);
    crate::words::Scheme {
        var_kinds: generic_vars(),
        stack_in: alloc::vec![
            stack(0),
            crate::shapes::Pattern::Var(crate::words::Variable(arm))
        ],
        stack_out: alloc::vec![stack(0), generic_pattern(decl)],
        effects: alloc::vec![],
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-kernel; the invariants of this linear dispatcher are enforced by its callers and the kernel test suite"
)]
pub(super) fn generic_matcher(decl: &super::super::GenericVariantDecl) -> crate::words::Scheme {
    let mut vars = generic_vars();
    vars.push(crate::words::VariableKind::Stack);
    vars.push(crate::words::VariableKind::Effect);
    vars.push(crate::words::VariableKind::Effect);
    let left = crate::shapes::Pattern::Var(crate::words::Variable(
        u32::from(decl.payload_params[0]).saturating_add(1),
    ));
    let right = crate::shapes::Pattern::Var(crate::words::Variable(
        u32::from(decl.payload_params[1]).saturating_add(1),
    ));
    crate::words::Scheme {
        var_kinds: vars,
        stack_in: alloc::vec![
            stack(0),
            generic_pattern(decl),
            generic_handler(left, 4),
            generic_handler(right, 5),
        ],
        stack_out: alloc::vec![stack(3)],
        effects: alloc::vec![
            crate::shapes::EffectSlot::Var(crate::words::Variable(4)),
            crate::shapes::EffectSlot::Var(crate::words::Variable(5)),
        ],
    }
}

fn generic_handler(arm: crate::shapes::Pattern, effect: u32) -> crate::shapes::Pattern {
    crate::shapes::Pattern::program(
        alloc::vec![stack(0), arm],
        alloc::vec![stack(3)],
        alloc::vec![crate::shapes::EffectSlot::Var(crate::words::Variable(
            effect
        ))],
    )
}

pub(super) fn expected_generic_scheme(
    decl: &super::super::GenericVariantDecl,
    kind: super::super::Behavior,
) -> Option<crate::words::Scheme> {
    #[expect(
        tigerstyle::fragile_exhaustive_enum_match,
        reason = "Owner: noble-kernel; the Behavior enum is deliberately sealed, so every exhaustive match over it is reviewed when a variant is added"
    )]
    match kind {
        super::super::Behavior::GenericLeft(id) if id == decl.id => Some(generic_arm(decl, false)),
        super::super::Behavior::GenericRight(id) if id == decl.id => Some(generic_arm(decl, true)),
        super::super::Behavior::GenericMatch(id) if id == decl.id => Some(generic_matcher(decl)),
        super::super::Behavior::GenericLeft(_) => None,
        super::super::Behavior::GenericRight(_) => None,
        super::super::Behavior::GenericMatch(_) => None,
        super::super::Behavior::Dup => None,
        super::super::Behavior::Drop => None,
        super::super::Behavior::Swap => None,
        super::super::Behavior::Dip => None,
        super::super::Behavior::Arith => None,
        super::super::Behavior::Equals => None,
        super::super::Behavior::Quote => None,
        super::super::Behavior::Compose => None,
        super::super::Behavior::Run => None,
        super::super::Behavior::Reflect => None,
        super::super::Behavior::Unit => None,
        super::super::Behavior::Pair => None,
        super::super::Behavior::Unpair => None,
        super::super::Behavior::Inl => None,
        super::super::Behavior::Inr => None,
        super::super::Behavior::Case => None,
        super::super::Behavior::If => None,
        super::super::Behavior::Nil => None,
        super::super::Behavior::Cons => None,
        super::super::Behavior::ListCase => None,
        super::super::Behavior::TestEmit => None,
        super::super::Behavior::TextByte => None,
        super::super::Behavior::I64Less => None,
        super::super::Behavior::CheckedI64Add => None,
        super::super::Behavior::CheckedI64Sub => None,
        super::super::Behavior::CheckedI64Mul => None,
        super::super::Behavior::CheckedI64Neg => None,
        super::super::Behavior::CheckedI64Abs => None,
        super::super::Behavior::CheckedI64Less => None,
        super::super::Behavior::CheckedI64DivRem => None,
        super::super::Behavior::CalculatorIterate => None,
        super::super::Behavior::CalculatorCompare => None,
        super::super::Behavior::CalculatorDivRem => None,
        super::super::Behavior::CalculatorTextFromAscii => None,
        super::super::Behavior::CalculatorTextLength => None,
        super::super::Behavior::Reserved => None,
        super::super::Behavior::BoundEmit(_) => None,
        super::super::Behavior::BoundClock(_) => None,
        super::super::Behavior::BoundCodeExport(_) => None,
        super::super::Behavior::BoundSend(_) => None,
        super::super::Behavior::NominalNew(_) => None,
        super::super::Behavior::NominalInto(_) => None,
        super::super::Behavior::NominalLeft(_) => None,
        super::super::Behavior::NominalRight(_) => None,
        super::super::Behavior::NominalMatch(_) => None,
        super::super::Behavior::Named => None,
    }
}

pub(super) fn emit_scheme() -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![stack(0), crate::shapes::Pattern::Text],
        stack_out: alloc::vec![stack(0)],
        effects: alloc::vec![crate::shapes::EffectSlot::Effect(super::super::TEST_EMIT)],
    }
}

pub(super) fn valid_emit_scheme(actual: &crate::words::Scheme) -> bool {
    actual.var_kinds.as_slice() == [crate::words::VariableKind::Stack]
        && actual.stack_in.as_slice() == [stack(0), crate::shapes::Pattern::Text]
        && actual.stack_out.as_slice() == [stack(0)]
        && actual.effects.as_slice() == [crate::shapes::EffectSlot::Effect(super::super::TEST_EMIT)]
}

pub(super) fn clock_scheme() -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![stack(0)],
        stack_out: alloc::vec![stack(0), crate::shapes::Pattern::I64],
        effects: alloc::vec![crate::shapes::EffectSlot::Effect(super::super::TEST_CLOCK)],
    }
}

pub(super) fn valid_clock_scheme(actual: &crate::words::Scheme) -> bool {
    actual.var_kinds.as_slice() == [crate::words::VariableKind::Stack]
        && actual.stack_in.as_slice() == [stack(0)]
        && actual.stack_out.as_slice() == [stack(0), crate::shapes::Pattern::I64]
        && actual.effects.as_slice()
            == [crate::shapes::EffectSlot::Effect(super::super::TEST_CLOCK)]
}

pub(super) fn code_export_scheme() -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![
            stack(0),
            crate::shapes::Pattern::Text,
            crate::shapes::Pattern::Text
        ],
        stack_out: alloc::vec![stack(0)],
        effects: alloc::vec![crate::shapes::EffectSlot::Effect(
            super::super::TEST_CODE_EXPORT
        )],
    }
}

pub(super) fn valid_code_export_scheme(actual: &crate::words::Scheme) -> bool {
    actual.var_kinds.as_slice() == [crate::words::VariableKind::Stack]
        && actual.stack_in.as_slice()
            == [
                stack(0),
                crate::shapes::Pattern::Text,
                crate::shapes::Pattern::Text,
            ]
        && actual.stack_out.as_slice() == [stack(0)]
        && actual.effects.as_slice()
            == [crate::shapes::EffectSlot::Effect(
                super::super::TEST_CODE_EXPORT,
            )]
}

pub(super) fn send_scheme() -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds: alloc::vec![crate::words::VariableKind::Stack],
        stack_in: alloc::vec![stack(0), crate::shapes::Pattern::Text],
        stack_out: alloc::vec![stack(0)],
        effects: alloc::vec![crate::shapes::EffectSlot::Effect(super::super::TEST_SEND)],
    }
}

pub(super) fn valid_send_scheme(actual: &crate::words::Scheme) -> bool {
    actual.var_kinds.as_slice() == [crate::words::VariableKind::Stack]
        && actual.stack_in.as_slice() == [stack(0), crate::shapes::Pattern::Text]
        && actual.stack_out.as_slice() == [stack(0)]
        && actual.effects.as_slice() == [crate::shapes::EffectSlot::Effect(super::super::TEST_SEND)]
}

pub(super) fn same_scheme(actual: &crate::words::Scheme, expected: &crate::words::Scheme) -> bool {
    actual.var_kinds == expected.var_kinds
        && actual.stack_in == expected.stack_in
        && actual.stack_out == expected.stack_out
        && actual.effects == expected.effects
}

pub(super) fn expected_scheme(
    env: &super::super::Env,
    decl: &super::super::NominalDecl,
    kind: super::super::Behavior,
) -> Option<crate::words::Scheme> {
    if let super::super::Behavior::NominalNew(id) = kind {
        if decl.id == id && matches!(decl.shape, crate::types::NominalShape::Opaque(_)) {
            conversion(env, decl, false)
        } else {
            None
        }
    } else if let super::super::Behavior::NominalInto(id) = kind {
        if decl.id == id && matches!(decl.shape, crate::types::NominalShape::Opaque(_)) {
            conversion(env, decl, true)
        } else {
            None
        }
    } else if let super::super::Behavior::NominalLeft(id) = kind {
        if decl.id == id && matches!(decl.shape, crate::types::NominalShape::Variant(_, _)) {
            arm(env, decl, false)
        } else {
            None
        }
    } else if let super::super::Behavior::NominalRight(id) = kind {
        if decl.id == id && matches!(decl.shape, crate::types::NominalShape::Variant(_, _)) {
            arm(env, decl, true)
        } else {
            None
        }
    } else if let super::super::Behavior::NominalMatch(id) = kind {
        if decl.id == id && matches!(decl.shape, crate::types::NominalShape::Variant(_, _)) {
            matcher(env, decl)
        } else {
            None
        }
    } else {
        // Other behaviors, including Named and bound hosts, have no nominal scheme.
        None
    }
}
