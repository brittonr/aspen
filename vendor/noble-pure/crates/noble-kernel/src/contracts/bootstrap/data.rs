//! Data and control word contracts in `Definition` order, plus the table.
//!
//! `Definition` order: `dup`, `drop`, `swap`, `dip`, `+`, `-`, `*`, `=`,
//! `quote`, `compose`, `run`, `reflect`, `unit`, `pair`, `unpair`, `inl`,
//! `inr`, `case`, `if`, `nil`, `cons`, `list.case`, `test.emit` — the
//! complete 23-entry bootstrap table.

fn unit() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![crate::words::VariableKind::Stack],
        alloc::vec![super::stack_var(0)],
        alloc::vec![super::stack_var(0), super::UNIT],
        alloc::vec![],
    )
}

fn pair_word() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Value
        ],
        alloc::vec![
            super::stack_var(0),
            super::value_var(1),
            super::value_var(2)
        ],
        alloc::vec![
            super::stack_var(0),
            super::pair(
                crate::shapes::Pattern::Var(crate::words::Variable(1)),
                crate::shapes::Pattern::Var(crate::words::Variable(2))
            ),
        ],
        alloc::vec![],
    )
}

fn unpair() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Value
        ],
        alloc::vec![
            super::stack_var(0),
            super::pair(
                crate::shapes::Pattern::Var(crate::words::Variable(1)),
                crate::shapes::Pattern::Var(crate::words::Variable(2))
            ),
        ],
        alloc::vec![
            super::stack_var(0),
            super::value_var(1),
            super::value_var(2)
        ],
        alloc::vec![],
    )
}

fn inl() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Value
        ],
        alloc::vec![super::stack_var(0), super::value_var(1)],
        alloc::vec![
            super::stack_var(0),
            super::sum(
                crate::shapes::Pattern::Var(crate::words::Variable(1)),
                crate::shapes::Pattern::Var(crate::words::Variable(2))
            ),
        ],
        alloc::vec![],
    )
}

fn inr() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Value
        ],
        alloc::vec![super::stack_var(0), super::value_var(2)],
        alloc::vec![
            super::stack_var(0),
            super::sum(
                crate::shapes::Pattern::Var(crate::words::Variable(1)),
                crate::shapes::Pattern::Var(crate::words::Variable(2))
            ),
        ],
        alloc::vec![],
    )
}

fn case() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            super::stack_var(0),
            super::sum(
                crate::shapes::Pattern::Var(crate::words::Variable(1)),
                crate::shapes::Pattern::Var(crate::words::Variable(2))
            ),
            super::program(
                alloc::vec![super::stack_var(0), super::value_var(1)],
                alloc::vec![super::stack_var(3)],
                alloc::vec![super::effect_var(4)],
            ),
            super::program(
                alloc::vec![super::stack_var(0), super::value_var(2)],
                alloc::vec![super::stack_var(3)],
                alloc::vec![super::effect_var(5)],
            ),
        ],
        alloc::vec![super::stack_var(3)],
        alloc::vec![super::effect_var(4), super::effect_var(5)],
    )
}

fn if_word() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            super::stack_var(0),
            super::BOOL,
            super::program(
                alloc::vec![super::stack_var(0)],
                alloc::vec![super::stack_var(1)],
                alloc::vec![super::effect_var(2)],
            ),
            super::program(
                alloc::vec![super::stack_var(0)],
                alloc::vec![super::stack_var(1)],
                alloc::vec![super::effect_var(3)],
            ),
        ],
        alloc::vec![super::stack_var(1)],
        alloc::vec![super::effect_var(2), super::effect_var(3)],
    )
}

fn nil() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value
        ],
        alloc::vec![super::stack_var(0)],
        alloc::vec![
            super::stack_var(0),
            super::list(crate::shapes::Pattern::Var(crate::words::Variable(1))),
        ],
        alloc::vec![],
    )
}

fn cons() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value
        ],
        alloc::vec![
            super::stack_var(0),
            super::value_var(1),
            super::list(crate::shapes::Pattern::Var(crate::words::Variable(1))),
        ],
        alloc::vec![
            super::stack_var(0),
            super::list(crate::shapes::Pattern::Var(crate::words::Variable(1))),
        ],
        alloc::vec![],
    )
}

fn list_case() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Value,
            crate::words::VariableKind::Stack,
            crate::words::VariableKind::Effect,
            crate::words::VariableKind::Effect
        ],
        alloc::vec![
            super::stack_var(0),
            super::list(crate::shapes::Pattern::Var(crate::words::Variable(1))),
            super::program(
                alloc::vec![super::stack_var(0)],
                alloc::vec![super::stack_var(2)],
                alloc::vec![super::effect_var(3)],
            ),
            super::program(
                alloc::vec![
                    super::stack_var(0),
                    super::value_var(1),
                    super::list(crate::shapes::Pattern::Var(crate::words::Variable(1)))
                ],
                alloc::vec![super::stack_var(2)],
                alloc::vec![super::effect_var(4)],
            ),
        ],
        alloc::vec![super::stack_var(2)],
        alloc::vec![super::effect_var(3), super::effect_var(4)],
    )
}

fn test_emit() -> crate::words::Scheme {
    super::scheme(
        alloc::vec![crate::words::VariableKind::Stack],
        alloc::vec![super::stack_var(0), super::TEXT],
        alloc::vec![super::stack_var(0), super::UNIT],
        alloc::vec![crate::shapes::EffectSlot::Effect(
            crate::contracts::TEST_EMIT
        )],
    )
}

/// The bootstrap table in `Definition` order.
pub fn table() -> alloc::vec::Vec<(crate::contracts::Behavior, crate::words::Scheme)> {
    alloc::vec![
        (crate::contracts::Behavior::Dup, super::dup()),
        (crate::contracts::Behavior::Drop, super::drop()),
        (crate::contracts::Behavior::Swap, super::swap()),
        (crate::contracts::Behavior::Dip, super::dip()),
        (crate::contracts::Behavior::Arith, super::arith()),
        (crate::contracts::Behavior::Arith, super::arith()),
        (crate::contracts::Behavior::Arith, super::arith()),
        (crate::contracts::Behavior::Equals, super::equals()),
        (crate::contracts::Behavior::Quote, super::quote()),
        (crate::contracts::Behavior::Compose, super::compose()),
        (crate::contracts::Behavior::Run, super::run()),
        (crate::contracts::Behavior::Reflect, super::reflect()),
        (crate::contracts::Behavior::Unit, unit()),
        (crate::contracts::Behavior::Pair, pair_word()),
        (crate::contracts::Behavior::Unpair, unpair()),
        (crate::contracts::Behavior::Inl, inl()),
        (crate::contracts::Behavior::Inr, inr()),
        (crate::contracts::Behavior::Case, case()),
        (crate::contracts::Behavior::If, if_word()),
        (crate::contracts::Behavior::Nil, nil()),
        (crate::contracts::Behavior::Cons, cons()),
        (crate::contracts::Behavior::ListCase, list_case()),
        (crate::contracts::Behavior::TestEmit, test_emit()),
    ]
}
