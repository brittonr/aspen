//! Emission uses resolved graph edges, never names or raw predicate strings.

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending expression declarations requires the caller's growable String, not a fixed str slice."
)]
pub(super) fn emit(out: &mut alloc::string::String, prepared: &crate::Prepared) {
    let mut index = 0usize;
    while index < prepared.expressions().len() {
        out.push_str("def expression_");
        super::number(out, index as u64);
        out.push_str(" : Term := ");
        term(
            out,
            &prepared.expressions()[index].kind,
            prepared.definitions(),
        );
        out.push('\n');
        index += 1;
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending an expression reference requires the caller's growable String, not a fixed str slice."
)]
fn reference(out: &mut alloc::string::String, index: u32) {
    out.push_str(" expression_");
    super::number(out, u64::from(index));
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending a unary term requires the caller's growable String, not a fixed str slice."
)]
fn unary(out: &mut alloc::string::String, name: &str, a: u32) {
    out.push_str(name);
    reference(out, a);
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending a binary term requires the caller's growable String, not a fixed str slice."
)]
#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; a and b are ordered expression IDs from one arena; each closed ExprKind arm supplies its fields in the same serialization order."
)]
fn binary(out: &mut alloc::string::String, name: &str, a: u32, b: u32) {
    unary(out, name, a);
    reference(out, b);
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending an indexed term requires the caller's growable String, not a fixed str slice."
)]
fn indexed(out: &mut alloc::string::String, name: &str, index: u32) {
    out.push_str(name);
    out.push(' ');
    super::number(out, u64::from(index));
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending term constructors requires the caller's growable String, not a fixed str slice."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; expression serialization grows the caller's String through non-const push and push_str."
)]
fn term(out: &mut alloc::string::String, kind: &crate::ExprKind, definitions: &[crate::LogicDef]) {
    match *kind {
        crate::ExprKind::I64(n) => {
            out.push_str(".i64 ");
            super::signed(out, n);
        }
        crate::ExprKind::Bool(true) => out.push_str(".bool true"),
        crate::ExprKind::Bool(false) => out.push_str(".bool false"),
        crate::ExprKind::Unit => out.push_str(".unit"),
        crate::ExprKind::Input(i) => indexed(out, ".input", i),
        crate::ExprKind::Output(i) => indexed(out, ".output", i),
        crate::ExprKind::Param(i) => indexed(out, ".param", i),
        crate::ExprKind::Definition(i) => {
            indexed(out, ".definition", i);
            let definition = match usize::try_from(i) {
                Ok(index) => definitions.get(index),
                Err(_) => None,
            };
            match definition {
                Some(definition) => reference(out, definition.body),
                None => {
                    out.push(' ');
                    super::rejected(out, "unresolved logical definition");
                }
            }
        }
        crate::ExprKind::Not(a) => unary(out, ".not", a),
        crate::ExprKind::And(a, b) => binary(out, ".and", a, b),
        crate::ExprKind::Or(a, b) => binary(out, ".or", a, b),
        crate::ExprKind::Implies(a, b) => binary(out, ".implies", a, b),
        crate::ExprKind::Eq(a, b) => binary(out, ".eq", a, b),
        crate::ExprKind::Lt(a, b) => binary(out, ".lt", a, b),
        crate::ExprKind::Le(a, b) => binary(out, ".le", a, b),
        crate::ExprKind::Add(a, b) => binary(out, ".add", a, b),
        crate::ExprKind::Sub(a, b) => binary(out, ".sub", a, b),
        crate::ExprKind::Mul(a, b) => binary(out, ".mul", a, b),
        crate::ExprKind::Pair(a, b) => binary(out, ".pair", a, b),
        crate::ExprKind::First(a) => unary(out, ".first", a),
        crate::ExprKind::Second(a) => unary(out, ".second", a),
        crate::ExprKind::Inl(a) => unary(out, ".inl", a),
        crate::ExprKind::Inr(a) => unary(out, ".inr", a),
        crate::ExprKind::IsLeft(a) => unary(out, ".isLeft", a),
        crate::ExprKind::Left(a) => unary(out, ".left", a),
        crate::ExprKind::Right(a) => unary(out, ".right", a),
        crate::ExprKind::Nil => out.push_str(".nil"),
        crate::ExprKind::Cons(a, b) => binary(out, ".cons", a, b),
        crate::ExprKind::IsNil(a) => unary(out, ".isNil", a),
        crate::ExprKind::Head(a) => unary(out, ".head", a),
        crate::ExprKind::Tail(a) => unary(out, ".tail", a),
        crate::ExprKind::Length(a) => unary(out, ".length", a),
        crate::ExprKind::Maps(p, a, b) => {
            binary(out, ".maps", p, a);
            reference(out, b);
        }
    }
}
