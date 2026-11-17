//! Exact membership in the finite supported claim algebra.

pub(crate) fn expression(prepared: &crate::Prepared, index: u32) -> Option<&crate::Expr> {
    match usize::try_from(index) {
        Ok(index) => prepared.expressions().get(index),
        Err(_) => None,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_true uses expression's non-const checked index conversion and Prepared expression lookup before recognizing the required Boolean literal."
)]
fn is_true(prepared: &crate::Prepared, index: u32) -> bool {
    match expression(prepared, index) {
        Some(slot) => is_boolean_true(&slot.kind),
        None => false,
    }
}

const fn is_boolean_true(kind: &crate::ExprKind) -> bool {
    if let crate::ExprKind::Bool(value) = kind {
        return *value;
    }
    false
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_output calls expression's checked usize::try_from and runtime Prepared accessor to recognize output slot zero without unchecked indexing."
)]
fn is_output(prepared: &crate::Prepared, index: u32) -> bool {
    match expression(prepared, index) {
        Some(slot) => is_output_zero(&slot.kind),
        None => false,
    }
}

const fn is_output_zero(kind: &crate::ExprKind) -> bool {
    if let crate::ExprKind::Output(index) = kind {
        return *index == 0;
    }
    false
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_input recognizes input slot zero only after expression's non-const checked conversion and Prepared arena lookup."
)]
fn is_input(prepared: &crate::Prepared, index: u32) -> bool {
    match expression(prepared, index) {
        Some(slot) => is_input_zero(&slot.kind),
        None => false,
    }
}

pub(crate) const fn is_input_zero(kind: &crate::ExprKind) -> bool {
    if let crate::ExprKind::Input(index) = kind {
        return *index == 0;
    }
    false
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_param_zero recognizes parameter slot zero through expression's non-const checked index conversion and runtime Prepared expression accessor."
)]
fn is_param_zero(prepared: &crate::Prepared, index: u32) -> bool {
    match expression(prepared, index) {
        Some(slot) => is_parameter_zero(&slot.kind),
        None => false,
    }
}

const fn is_parameter_zero(kind: &crate::ExprKind) -> bool {
    if let crate::ExprKind::Param(index) = kind {
        return *index == 0;
    }
    false
}

pub(crate) const fn literal(kind: &crate::ExprKind) -> Option<i64> {
    if let crate::ExprKind::I64(value) = kind {
        return Some(*value);
    }
    None
}

#[expect(
    clippy::question_mark,
    reason = "Owner: noble-maintainers; explicit Option propagation remains supported by the pinned extractor."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; increment_operand resolves optional definition references with non-const usize::try_from and Prepared::definitions/expressions accessors before reading the literal."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; increment_operand bounds expression and definition lookups and returns None for missing references or nonliteral bodies; arbitrary predicate nodes must not be asserted to belong to the increment algebra."
)]
fn increment_operand(prepared: &crate::Prepared, index: u32) -> Option<i64> {
    let slot = match expression(prepared, index) {
        Some(slot) => slot,
        None => return None,
    };
    if let crate::ExprKind::Definition(definition) = slot.kind {
        let index = match usize::try_from(definition) {
            Ok(index) => index,
            Err(_) => return None,
        };
        let definition = match prepared.definitions().get(index) {
            Some(definition) => definition,
            None => return None,
        };
        return match expression(prepared, definition.body) {
            Some(body) => literal(&body.kind),
            None => None,
        };
    }
    literal(&slot.kind)
}

#[expect(
    clippy::question_mark,
    reason = "Owner: noble-maintainers; explicit Option propagation remains supported by the pinned extractor."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; add_increment uses runtime checked expression lookup and increment_operand's non-const definition resolution while recognizing either addition operand order."
)]
fn add_increment(prepared: &crate::Prepared, index: u32) -> Option<i64> {
    let slot = match expression(prepared, index) {
        Some(slot) => slot,
        None => return None,
    };
    if let crate::ExprKind::Add(left, right) = slot.kind {
        if is_input(prepared, left) {
            return increment_operand(prepared, right);
        }
        if is_input(prepared, right) {
            return increment_operand(prepared, left);
        }
    }
    None
}

#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; output_increment intentionally recognizes either Eq operand as output and parses the opposite addition; swapping these same-arena indices preserves the result, so distinct role wrappers would misstate the symmetric contract."
)]
fn output_increment(prepared: &crate::Prepared, left: u32, right: u32) -> Option<i64> {
    if is_output(prepared, left) {
        return add_increment(prepared, right);
    }
    if is_output(prepared, right) {
        return add_increment(prepared, left);
    }
    None
}

/// Nonmembers retain the entire admitted statement; no predicate is dropped.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; classify checks the true precondition, interface shape/type and effects before indexing and recognizing the conclusion; nonmembers retain Admitted(statement) instead of causing assertion failures."
)]
pub(crate) fn classify(
    prepared: &crate::Prepared,
    statement: u64,
    program: &[(u32, u64)],
) -> crate::companion::admit::ClaimTemplate {
    if !is_true(prepared, prepared.requires()) {
        return crate::companion::admit::ClaimTemplate::Admitted(statement);
    }
    if prepared.inputs().len() != 1 {
        return crate::companion::admit::ClaimTemplate::Admitted(statement);
    }
    if prepared.outputs().len() != 1 {
        return crate::companion::admit::ClaimTemplate::Admitted(statement);
    }
    if prepared.inputs()[0].ty != noble_kernel::types::Ty::I64 {
        return crate::companion::admit::ClaimTemplate::Admitted(statement);
    }
    if !prepared.checked().interface.effects.is_empty() {
        return crate::companion::admit::ClaimTemplate::Admitted(statement);
    }
    let root = match expression(prepared, prepared.ensures()) {
        Some(slot) => slot,
        None => return crate::companion::admit::ClaimTemplate::Admitted(statement),
    };
    if is_family(prepared, &root.kind, program) {
        return crate::companion::admit::ClaimTemplate::IncrementByCapture;
    }
    if let Some(value) = increment_result(prepared, &root.kind) {
        return crate::companion::admit::ClaimTemplate::IncrementBy(value);
    }
    crate::companion::admit::ClaimTemplate::Admitted(statement)
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_family reads runtime Prepared interfaces and uses non-const Ty and operation-slice equality to require the exact capture-family type and program."
)]
fn is_family(prepared: &crate::Prepared, kind: &crate::ExprKind, program: &[(u32, u64)]) -> bool {
    if let crate::ExprKind::Maps(subject, argument, result) = kind {
        return is_output(prepared, *subject)
            && is_param_zero(prepared, *argument)
            && is_family_result(prepared, *result)
            && prepared.params().len() == 1
            && prepared.params()[0].ty == noble_kernel::types::Ty::I64
            && is_increment_program_type(&prepared.outputs()[0].ty)
            && program == [(2, 8), (3, 0), (2, 4), (6, 0), (2, 9)];
    }
    false
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; increment_result reads runtime Prepared parameters/outputs, compares Ty with derived non-const equality and delegates Eq recognition to checked expression lookup."
)]
fn increment_result(prepared: &crate::Prepared, kind: &crate::ExprKind) -> Option<i64> {
    if !prepared.params().is_empty() || prepared.outputs()[0].ty != noble_kernel::types::Ty::I64 {
        return None;
    }
    if let crate::ExprKind::Eq(left, right) = kind {
        return output_increment(prepared, *left, *right);
    }
    None
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_family_result resolves the Maps result through expression's non-const index conversion and Prepared accessor before recognizing parameter/input addition."
)]
fn is_family_result(prepared: &crate::Prepared, index: u32) -> bool {
    match expression(prepared, index) {
        Some(slot) => is_family_addition(prepared, &slot.kind),
        None => false,
    }
}

fn is_family_addition(prepared: &crate::Prepared, kind: &crate::ExprKind) -> bool {
    if let crate::ExprKind::Add(left, right) = kind {
        return (is_param_zero(prepared, *left) && is_input(prepared, *right))
            || (is_param_zero(prepared, *right) && is_input(prepared, *left));
    }
    false
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; is_increment_program_type compares both owned program endpoint slices with non-const Ty slice equality before checking the runtime EffSet emptiness predicate."
)]
fn is_increment_program_type(ty: &noble_kernel::types::Ty) -> bool {
    if let noble_kernel::types::Ty::Program(input, output, effects) = ty {
        return input.as_slice() == [noble_kernel::types::Ty::I64]
            && output.as_slice() == [noble_kernel::types::Ty::I64]
            && effects.is_empty();
    }
    false
}
