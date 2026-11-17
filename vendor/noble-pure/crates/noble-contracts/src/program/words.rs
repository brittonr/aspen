pub(super) struct Call {
    pub def: noble_kernel::contracts::Definition,
    pub current: u32,
    pub explicit: Option<noble_kernel::words::Inst>,
    pub span: crate::Span,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; scheme lookup, effect eligibility, explicit witnesses, and stack equations are checked with typed diagnostics."
)]
pub(super) fn apply(
    call: Call,
    arena: &mut crate::inference::Arena,
    env: &noble_kernel::contracts::Env,
    meter: &mut crate::Meter,
) -> Result<(u32, super::Draft), crate::Diagnostic> {
    let scheme = match env.scheme(call.def) {
        Some(scheme) => scheme,
        None => return Err(crate::invalid(call.span, "unknown word definition")),
    };
    let variables = attempt!(arena.variables(&scheme.var_kinds, call.span, meter));
    attempt!(crate::inference::pure_effects(
        &scheme.effects,
        &variables,
        call.span,
        meter
    ));
    if let Some(inst) = &call.explicit {
        attempt!(super::witness::apply(
            super::witness::Explicit {
                inst,
                kinds: &scheme.var_kinds,
                variables: &variables,
                span: call.span,
            },
            arena,
            meter
        ));
    }
    let input = attempt!(arena.pattern_stack(&scheme.stack_in, &variables, call.span, meter));
    let output = attempt!(arena.pattern_stack(&scheme.stack_out, &variables, call.span, meter));
    attempt!(arena.unify(call.current, input, call.span, meter));
    Ok((
        output,
        super::Draft {
            kind: super::DraftKind::Invocation(call.def),
            variables,
            explicit: call.explicit,
            span: call.span,
        },
    ))
}

pub(super) fn identity(
    word: &[u8],
    span: crate::Span,
) -> Result<noble_kernel::contracts::Definition, crate::Diagnostic> {
    match bootstrap(word) {
        Some(definition) if definition.0 < 22 => Ok(definition),
        Some(_) => Err(host_error(span)),
        None => {
            let is_host = word == b"host";
            let is_call = word == b"call";
            if is_host || is_call {
                Err(host_error(span))
            } else {
                Err(crate::invalid(span, "unknown program word"))
            }
        }
    }
}

fn host_error(span: crate::Span) -> crate::Diagnostic {
    crate::Diagnostic::new(
        crate::DiagnosticKind::Unsupported,
        span,
        "host and effectful words are outside the pure contract fragment",
    )
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; the finite exact spelling table returns None for unknown words; it has no producer invariant to assert and performs no unchecked indexing."
)]
pub(super) fn bootstrap(word: &[u8]) -> Option<noble_kernel::contracts::Definition> {
    let id = match word {
        _ if word == b"dup" => 0,
        _ if word == b"drop" => 1,
        _ if word == b"swap" => 2,
        _ if word == b"dip" => 3,
        _ if word == b"+" => 4,
        _ if word == b"-" => 5,
        _ if word == b"*" => 6,
        _ if word == b"=" => 7,
        _ if word == b"quote" => 8,
        _ if word == b"compose" => 9,
        _ if word == b"run" => 10,
        _ if word == b"reflect" => 11,
        _ if word == b"unit" => 12,
        _ if word == b"pair" => 13,
        _ if word == b"unpair" => 14,
        _ if word == b"inl" => 15,
        _ if word == b"inr" => 16,
        _ if word == b"case" => 17,
        _ if word == b"if" => 18,
        _ if word == b"nil" => 19,
        _ if word == b"cons" => 20,
        _ if word == b"list.case" => 21,
        _ if word == b"test.emit" => 22,
        _ if word == b"test.abort" => 23,
        _ => return None,
    };
    Some(noble_kernel::contracts::Definition(id))
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the finite spelling table appends directly to an owned diagnostic String without indexing or a temporary allocation; unknown diagnostic IDs retain their explicit unknown label."
)]
pub(super) fn append_spelling(
    definition: noble_kernel::contracts::Definition,
    output: &mut alloc::string::String,
) {
    output.push_str(spelling(definition.0).unwrap_or("unknown"));
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-contracts; invalid input is rejected through typed outcomes rather than assertions, and the invariants are enforced by callers and the crate test suite."
)]
pub(super) const fn spelling(identity: u32) -> Option<&'static str> {
    let name = match identity {
        0 => "dup",
        1 => "drop",
        2 => "swap",
        3 => "dip",
        4 => "+",
        5 => "-",
        6 => "*",
        7 => "=",
        8 => "quote",
        9 => "compose",
        10 => "run",
        11 => "reflect",
        12 => "unit",
        13 => "pair",
        14 => "unpair",
        15 => "inl",
        16 => "inr",
        17 => "case",
        18 => "if",
        19 => "nil",
        20 => "cons",
        21 => "list.case",
        22 => "test.emit",
        23 => "test.abort",
        _ => return None,
    };
    Some(name)
}
