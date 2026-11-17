//! The fixed v1 word-contract table, in `Definition` order.
//!
//! Each helper builds one scheme; `table` assembles them so the environment
//! builder stays one short loop.

const UNIT: crate::shapes::Pattern = crate::shapes::Pattern::Unit;
const BOOL: crate::shapes::Pattern = crate::shapes::Pattern::Bool;
const I64: crate::shapes::Pattern = crate::shapes::Pattern::I64;
const TEXT: crate::shapes::Pattern = crate::shapes::Pattern::Text;
const SYNTAX: crate::shapes::Pattern = crate::shapes::Pattern::Syntax;

fn stack_var(index: u32) -> crate::shapes::Pattern {
    crate::shapes::Pattern::StackVar(crate::words::Variable(index))
}

fn value_var(index: u32) -> crate::shapes::Pattern {
    crate::shapes::Pattern::Var(crate::words::Variable(index))
}

fn effect_var(index: u32) -> crate::shapes::EffectSlot {
    crate::shapes::EffectSlot::Var(crate::words::Variable(index))
}

fn pair(left: crate::shapes::Pattern, right: crate::shapes::Pattern) -> crate::shapes::Pattern {
    crate::shapes::Pattern::Pair(alloc::boxed::Box::new(left), alloc::boxed::Box::new(right))
}

fn sum(left: crate::shapes::Pattern, right: crate::shapes::Pattern) -> crate::shapes::Pattern {
    crate::shapes::Pattern::Sum(alloc::boxed::Box::new(left), alloc::boxed::Box::new(right))
}

fn list(item: crate::shapes::Pattern) -> crate::shapes::Pattern {
    crate::shapes::Pattern::List(alloc::boxed::Box::new(item))
}

fn program(
    item_in: alloc::vec::Vec<crate::shapes::Pattern>,
    item_out: alloc::vec::Vec<crate::shapes::Pattern>,
    effects: alloc::vec::Vec<crate::shapes::EffectSlot>,
) -> crate::shapes::Pattern {
    crate::shapes::Pattern::program(item_in, item_out, effects)
}

fn scheme(
    var_kinds: alloc::vec::Vec<crate::words::VariableKind>,
    stack_in: alloc::vec::Vec<crate::shapes::Pattern>,
    stack_out: alloc::vec::Vec<crate::shapes::Pattern>,
    effects: alloc::vec::Vec<crate::shapes::EffectSlot>,
) -> crate::words::Scheme {
    crate::words::Scheme {
        var_kinds,
        stack_in,
        stack_out,
        effects,
    }
}

fn dup() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value
        ],
        alloc::vec![stack_var(0), value_var(1)],
        alloc::vec![stack_var(0), value_var(1), value_var(1)],
        alloc::vec![],
    )
}

fn drop() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value
        ],
        alloc::vec![stack_var(0), value_var(1)],
        alloc::vec![stack_var(0)],
        alloc::vec![],
    )
}

fn swap() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Value
        ],
        alloc::vec![stack_var(0), value_var(1), value_var(2)],
        alloc::vec![stack_var(0), value_var(2), value_var(1)],
        alloc::vec![],
    )
}

fn dip() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            stack_var(0),
            value_var(1),
            program(
                alloc::vec![stack_var(0)],
                alloc::vec![stack_var(2)],
                alloc::vec![effect_var(3)],
            ),
        ],
        alloc::vec![stack_var(2), value_var(1)],
        alloc::vec![effect_var(3)],
    )
}

fn arith() -> crate::words::Scheme {
    scheme(
        alloc::vec![crate::words::VariableKind::Stack],
        alloc::vec![stack_var(0), I64, I64],
        alloc::vec![stack_var(0), I64],
        alloc::vec![],
    )
}

/// `=`: `S I64 I64 -- S Bool ! {}`. The bootstrap numeric vocabulary's
/// equality compares two `I64` values and returns `Bool`; it is not an
/// implicit generic equality operator for every type (K-NUM-01).
fn equals() -> crate::words::Scheme {
    scheme(
        alloc::vec![crate::words::VariableKind::Stack],
        alloc::vec![stack_var(0), I64, I64],
        alloc::vec![stack_var(0), BOOL],
        alloc::vec![],
    )
}

fn quote() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Stack
        ],
        alloc::vec![stack_var(0), value_var(1)],
        alloc::vec![
            stack_var(0),
            program(
                alloc::vec![stack_var(2)],
                alloc::vec![stack_var(2), value_var(1)],
                alloc::vec![],
            ),
        ],
        alloc::vec![],
    )
}

fn compose() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            stack_var(0),
            program(
                alloc::vec![stack_var(1)],
                alloc::vec![stack_var(2)],
                alloc::vec![effect_var(4)],
            ),
            program(
                alloc::vec![stack_var(2)],
                alloc::vec![stack_var(3)],
                alloc::vec![effect_var(5)],
            ),
        ],
        alloc::vec![
            stack_var(0),
            program(
                alloc::vec![stack_var(1)],
                alloc::vec![stack_var(3)],
                alloc::vec![effect_var(4), effect_var(5)],
            ),
        ],
        alloc::vec![],
    )
}

fn run() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            stack_var(0),
            program(
                alloc::vec![stack_var(0)],
                alloc::vec![stack_var(1)],
                alloc::vec![effect_var(2)],
            ),
        ],
        alloc::vec![stack_var(1)],
        alloc::vec![effect_var(2)],
    )
}

fn reflect() -> crate::words::Scheme {
    scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            stack_var(0),
            program(
                alloc::vec![stack_var(1)],
                alloc::vec![stack_var(2)],
                alloc::vec![effect_var(3)],
            ),
        ],
        alloc::vec![stack_var(0), SYNTAX],
        alloc::vec![],
    )
}

pub(crate) mod data;
