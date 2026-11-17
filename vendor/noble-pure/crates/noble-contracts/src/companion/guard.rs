//! Finite prechecked guard templates and their exact wrapper programs.
//! An unmatched whole precondition is unsupported, never an assumed guard.

const CODE_LT_I64_MAX: &str = "lt-i64-max";
const CODE_NE_I64_MIN: &str = "ne-i64-min";
const CODE_EQ_I64_LITERAL: &str = "eq-i64-literal";

impl crate::companion::GuardTemplate {
    pub(crate) fn digest(self) -> u64 {
        let mut fold = crate::companion::digest::Fold::new(crate::companion::digest::DOMAIN_GUARD);
        match self {
            crate::companion::GuardTemplate::LtI64Max => fold.absorb(1),
            crate::companion::GuardTemplate::NeI64Min => fold.absorb(2),
            crate::companion::GuardTemplate::EqI64Literal(value) => {
                fold.absorb(3);
                fold.absorb(u64::from_ne_bytes(value.to_ne_bytes()));
            }
        }
        fold.finish()
    }

    pub fn code(self) -> &'static str {
        match self {
            crate::companion::GuardTemplate::LtI64Max => CODE_LT_I64_MAX,
            crate::companion::GuardTemplate::NeI64Min => CODE_NE_I64_MIN,
            crate::companion::GuardTemplate::EqI64Literal(_) => CODE_EQ_I64_LITERAL,
        }
    }
}

#[expect(
    clippy::question_mark,
    reason = "Owner: noble-maintainers; explicit Option propagation preserves the pinned extractor's supported control flow."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; literal_of resolves a checked expression index through runtime Prepared accessors and TryFrom; reassess when that shared arena lookup becomes const."
)]
fn literal_of(prepared: &crate::Prepared, index: u32) -> Option<i64> {
    let slot = match crate::companion::admit::recognize::expression(prepared, index) {
        Some(slot) => slot,
        None => return None,
    };
    crate::companion::admit::recognize::literal(&slot.kind)
}

/// Match the complete precondition on the actual input, never a nested fragment.
pub(crate) fn templates_of(
    prepared: &crate::Prepared,
) -> alloc::vec::Vec<crate::companion::GuardTemplate> {
    let mut found = alloc::vec::Vec::with_capacity(1);
    if prepared.checked().interface.stack_in.as_slice() != [noble_kernel::types::Ty::I64] {
        return found;
    }
    let root = match crate::companion::admit::recognize::expression(prepared, prepared.requires()) {
        Some(slot) => slot,
        None => return found,
    };
    if let Some(template) = recognize(prepared, &root.kind) {
        found.push(template);
    }
    found
}

#[expect(
    tigerstyle::missing_const_fn,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; recognize uses runtime expression lookup and Option mapping to match only a complete precondition; unsupported predicates return None rather than becoming assertion failures."
)]
fn recognize(
    prepared: &crate::Prepared,
    kind: &crate::ExprKind,
) -> Option<crate::companion::GuardTemplate> {
    if let crate::ExprKind::Lt(a, b) = kind {
        return if is_input_zero(prepared, *a) && literal_of(prepared, *b) == Some(i64::MAX) {
            Some(crate::companion::GuardTemplate::LtI64Max)
        } else {
            None
        };
    }
    if let crate::ExprKind::Eq(a, b) = kind {
        return input_equality(prepared, *a, *b).map(crate::companion::GuardTemplate::EqI64Literal);
    }
    if let crate::ExprKind::Not(inner) = kind {
        return if is_minimum_equality(prepared, *inner) {
            Some(crate::companion::GuardTemplate::NeI64Min)
        } else {
            None
        };
    }
    None
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_input_zero performs checked runtime Prepared arena lookup, returning false for an unavailable expression rather than evaluating a const descriptor."
)]
fn is_input_zero(prepared: &crate::Prepared, index: u32) -> bool {
    match crate::companion::admit::recognize::expression(prepared, index) {
        Some(slot) => crate::companion::admit::recognize::is_input_zero(&slot.kind),
        None => false,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_minimum_equality resolves expression and operand indices through runtime Prepared accessors and compares Option values through nonconst trait equality."
)]
fn is_minimum_equality(prepared: &crate::Prepared, index: u32) -> bool {
    let slot = match crate::companion::admit::recognize::expression(prepared, index) {
        Some(slot) => slot,
        None => return false,
    };
    if let crate::ExprKind::Eq(a, b) = slot.kind {
        input_equality(prepared, a, b) == Some(i64::MIN)
    } else {
        false
    }
}

#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; input_equality recognizes either operand order of symmetric equality, so swapping the two expression indices preserves the result."
)]
fn input_equality(prepared: &crate::Prepared, left: u32, right: u32) -> Option<i64> {
    if is_input_zero(prepared, left) {
        literal_of(prepared, right)
    } else if is_input_zero(prepared, right) {
        literal_of(prepared, left)
    } else {
        None
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; templates checks retained contract membership, current context and nonempty recognized guards before a length-bounded copy; absent or stale guard authority must return Refusal, not panic."
)]
pub(crate) fn templates(
    engine: &crate::companion::Core,
    contract: crate::companion::ContractId,
) -> Result<alloc::vec::Vec<crate::companion::GuardTemplate>, crate::companion::Refusal> {
    let entry = attempt!(engine.registry.contract(contract));
    if crate::companion::registry::Registry::stale(
        engine,
        crate::companion::registry::ContextSnapshot {
            policy: entry.policy,
            revision: entry.revision,
        },
    ) {
        return Err(crate::companion::Refusal::StaleContext);
    }
    if entry.guards.is_empty() {
        return Err(crate::companion::Refusal::UnsupportedGuardTemplate);
    }
    let mut out = alloc::vec::Vec::with_capacity(entry.guards.len());
    let mut index = 0usize;
    while index < entry.guards.len() {
        out.push(entry.guards[index]);
        index = index.saturating_add(1);
    }
    Ok(out)
}

/// Produce one quotation. Rejection yields inl without running the subject;
/// successful invocation yields inr. Only prechecked template forms exist.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; wrapper_source emits one explicit branch for each sealed guard template and copies the supplied body unchanged; assertions cannot establish body validity, which belongs to ordinary checking."
)]
pub(crate) fn wrapper_source(
    _core: &crate::companion::Core,
    template: crate::companion::GuardTemplate,
    subject: &str,
) -> alloc::string::String {
    let mut out = alloc::string::String::new();
    match template {
        crate::companion::GuardTemplate::LtI64Max => {
            out.push_str("[ dup ");
            push_i64(&mut out, i64::MAX);
            out.push_str(" = [ drop 1 inl ] [ [ ");
            out.push_str(subject);
            out.push_str(" ] run inr ] if ]");
        }
        crate::companion::GuardTemplate::NeI64Min => {
            out.push_str("[ dup ");
            push_i64(&mut out, i64::MIN);
            out.push_str(" = [ drop 1 inl ] [ [ ");
            out.push_str(subject);
            out.push_str(" ] run inr ] if ]");
        }
        crate::companion::GuardTemplate::EqI64Literal(value) => {
            out.push_str("[ dup ");
            push_i64(&mut out, value);
            out.push_str(" = [ [ ");
            out.push_str(subject);
            out.push_str(" ] run inr ] [ drop 1 inl ] if ]");
        }
    }
    out
}

/// Execute the already observed compiled subject on [argument, program].
/// Rejection consumes both values without starting the protected body.
pub(crate) fn invocation_wrapper(
    template: crate::companion::GuardTemplate,
) -> alloc::string::String {
    let mut out = alloc::string::String::from("[ swap dup ");
    let literal = match template {
        crate::companion::GuardTemplate::LtI64Max => i64::MAX,
        crate::companion::GuardTemplate::NeI64Min => i64::MIN,
        crate::companion::GuardTemplate::EqI64Literal(value) => value,
    };
    push_i64(&mut out, literal);
    match template {
        crate::companion::GuardTemplate::LtI64Max | crate::companion::GuardTemplate::NeI64Min => {
            out.push_str(" = [ drop drop 1 inl ] [ swap run inr ] if ]");
        }
        crate::companion::GuardTemplate::EqI64Literal(_) => {
            out.push_str(" = [ swap run inr ] [ drop drop 1 inl ] if ]");
        }
    }
    out
}

#[expect(
    tigerstyle::mutating_input_in_pure,
    tigerstyle::borrowed_argument_types,
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; push_i64 bounds scratch writes by twenty slots and reverse reads by the filled count; every i64 including MIN has a defined unsigned_abs encoding, with no assertion precondition. Only the private String grows, which a slice cannot express."
)]
pub(crate) fn push_i64(out: &mut alloc::string::String, value: i64) {
    if value < 0 {
        out.push('-');
    }
    let mut digits = [0u8; 20];
    let mut count = 0usize;
    let mut remaining = value.unsigned_abs();
    if remaining == 0 {
        out.push('0');
        return;
    }
    while remaining > 0 && count < 20 {
        let digit = (remaining % 10).to_le_bytes()[0];
        digits[count] = digit.saturating_add(b'0');
        remaining /= 10;
        count = count.saturating_add(1);
    }
    while count > 0 {
        count -= 1;
        out.push(char::from(digits[count]));
    }
}
