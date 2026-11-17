#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; folding mutates only fresh private digest scratch; the retained Prepared and all borrowed types remain immutable, and no host state or mutable capability is exposed"
)]

const fn expression_tag(kind: &crate::ExprKind) -> u64 {
    match kind {
        crate::ExprKind::I64(..) => 1,
        crate::ExprKind::Bool(..) => 2,
        crate::ExprKind::Unit => 3,
        crate::ExprKind::Input(..) => 4,
        crate::ExprKind::Output(..) => 5,
        crate::ExprKind::Param(..) => 6,
        crate::ExprKind::Definition(..) => 7,
        crate::ExprKind::Not(..) => 8,
        crate::ExprKind::And(..) => 9,
        crate::ExprKind::Or(..) => 10,
        crate::ExprKind::Implies(..) => 11,
        crate::ExprKind::Eq(..) => 12,
        crate::ExprKind::Lt(..) => 13,
        crate::ExprKind::Le(..) => 14,
        crate::ExprKind::Add(..) => 15,
        crate::ExprKind::Sub(..) => 16,
        crate::ExprKind::Mul(..) => 17,
        crate::ExprKind::Pair(..) => 18,
        crate::ExprKind::First(..) => 19,
        crate::ExprKind::Second(..) => 20,
        crate::ExprKind::Inl(..) => 21,
        crate::ExprKind::Inr(..) => 22,
        crate::ExprKind::IsLeft(..) => 23,
        crate::ExprKind::Left(..) => 24,
        crate::ExprKind::Right(..) => 25,
        crate::ExprKind::Nil => 26,
        crate::ExprKind::Cons(..) => 27,
        crate::ExprKind::IsNil(..) => 28,
        crate::ExprKind::Head(..) => 29,
        crate::ExprKind::Tail(..) => 30,
        crate::ExprKind::Length(..) => 31,
        crate::ExprKind::Maps(..) => 32,
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; absorb_expr calls the runtime digest fold and non-const From conversions for Boolean and index operands; canonical integer encoding is not const on the pinned compiler."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; absorb_expr is a total constructor fold that treats referenced indices as data without dereferencing them; arbitrary indices remain digest input, so assertions would change the fingerprint contract."
)]
pub(in crate::companion) fn absorb_expr(
    fold: &mut crate::companion::digest::Fold,
    kind: &crate::ExprKind,
) {
    fold.absorb(expression_tag(kind));
    match kind {
        crate::ExprKind::I64(value) => fold.absorb(u64::from_ne_bytes(value.to_ne_bytes())),
        crate::ExprKind::Bool(value) => fold.absorb(u64::from(*value)),
        crate::ExprKind::Unit | crate::ExprKind::Nil => {}
        crate::ExprKind::Input(index)
        | crate::ExprKind::Output(index)
        | crate::ExprKind::Param(index)
        | crate::ExprKind::Definition(index) => fold.absorb(u64::from(*index)),
        crate::ExprKind::Not(a)
        | crate::ExprKind::First(a)
        | crate::ExprKind::Second(a)
        | crate::ExprKind::Inl(a)
        | crate::ExprKind::Inr(a)
        | crate::ExprKind::IsLeft(a)
        | crate::ExprKind::Left(a)
        | crate::ExprKind::Right(a)
        | crate::ExprKind::IsNil(a)
        | crate::ExprKind::Head(a)
        | crate::ExprKind::Tail(a)
        | crate::ExprKind::Length(a) => fold.absorb(u64::from(*a)),
        crate::ExprKind::And(a, b)
        | crate::ExprKind::Or(a, b)
        | crate::ExprKind::Implies(a, b)
        | crate::ExprKind::Eq(a, b)
        | crate::ExprKind::Lt(a, b)
        | crate::ExprKind::Le(a, b)
        | crate::ExprKind::Add(a, b)
        | crate::ExprKind::Sub(a, b)
        | crate::ExprKind::Mul(a, b)
        | crate::ExprKind::Pair(a, b)
        | crate::ExprKind::Cons(a, b) => {
            fold.absorb(u64::from(*a));
            fold.absorb(u64::from(*b));
        }
        crate::ExprKind::Maps(a, b, c) => {
            fold.absorb(u64::from(*a));
            fold.absorb(u64::from(*b));
            fold.absorb(u64::from(*c));
        }
    }
}

pub(in crate::companion) fn fold_named(
    fold: &mut crate::companion::digest::Fold,
    entries: &[crate::NamedType],
) {
    fold.absorb_count(entries.len());
    let mut index = 0usize;
    while index < entries.len() {
        fold.absorb_bytes(entries[index].name.as_bytes());
        crate::companion::admit::types::fold(fold, &entries[index].ty);
        index = index.saturating_add(1);
    }
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; fold_subject records a lowering-failure marker and otherwise scans nodes, quotation bodies and the root body with length-guarded indices; malformed lowering remains a digest outcome rather than a panic."
)]
pub(in crate::companion) fn fold_subject(
    fold: &mut crate::companion::digest::Fold,
    candidate: &noble_kernel::untrusted::Candidate,
) {
    fold.absorb(u64::from(candidate.format));
    fold.absorb(u64::from(candidate.revision));
    // The semantic projection is the canonical program encoding already used
    // for Lean export; folding it keeps the program body in the statement.
    let subject = match crate::wire::lower_subject(candidate) {
        Ok(subject) => subject,
        Err(_) => {
            fold.absorb(u64::MAX);
            return;
        }
    };
    fold.absorb_count(subject.nodes.len());
    let mut index = 0usize;
    while index < subject.nodes.len() {
        match &subject.nodes[index] {
            crate::wire::SemanticNode::I64(value) => {
                fold.absorb(1);
                fold.absorb(u64::from_ne_bytes(value.to_ne_bytes()));
            }
            crate::wire::SemanticNode::Boolean(value) => {
                fold.absorb(2);
                fold.absorb(u64::from(*value));
            }
            crate::wire::SemanticNode::UnitValue => fold.absorb(3),
            crate::wire::SemanticNode::Word(def) => {
                fold.absorb(4);
                fold.absorb(u64::from(*def));
            }
            crate::wire::SemanticNode::Quotation(nodes) => {
                fold.absorb(5);
                fold.absorb_count(nodes.len());
                let mut at = 0usize;
                while at < nodes.len() {
                    fold.absorb(u64::from(nodes[at]));
                    at = at.saturating_add(1);
                }
            }
        }
        index = index.saturating_add(1);
    }
    fold.absorb_count(subject.body.len());
    let mut at = 0usize;
    while at < subject.body.len() {
        fold.absorb(u64::from(subject.body[at]));
        at = at.saturating_add(1);
    }
}

/// A stable digest over an ordered stack interface.
pub fn interface_signature(entries: &[noble_kernel::types::Ty]) -> u64 {
    let mut fold = crate::companion::digest::Fold::new(crate::companion::digest::DOMAIN_SUBJECT);
    fold.absorb_count(entries.len());
    let mut index = 0usize;
    while index < entries.len() {
        crate::companion::admit::types::fold(&mut fold, &entries[index]);
        index = index.saturating_add(1);
    }
    fold.finish()
}
