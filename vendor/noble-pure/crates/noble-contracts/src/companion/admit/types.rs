#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; folding mutates only fresh private digest scratch; the retained Prepared and all borrowed types remain immutable, and no host state or mutable capability is exposed"
)]

/// A stable constructor tag for a type, folded without type recursion.
const fn ty_tag(ty: &noble_kernel::types::Ty) -> u64 {
    match ty {
        noble_kernel::types::Ty::Unit => 1,
        noble_kernel::types::Ty::Bool => 2,
        noble_kernel::types::Ty::I64 => 3,
        noble_kernel::types::Ty::Text => 4,
        noble_kernel::types::Ty::Syntax => 5,
        noble_kernel::types::Ty::Contract => 6,
        noble_kernel::types::Ty::Evidence => 7,
        noble_kernel::types::Ty::Certified => 8,
        noble_kernel::types::Ty::Pair(_, _) => 9,
        noble_kernel::types::Ty::Sum(_, _) => 10,
        noble_kernel::types::Ty::List(_) => 11,
        noble_kernel::types::Ty::Program(_, _, _) => 12,
        noble_kernel::types::Ty::Resource(_) => 13,
        noble_kernel::types::Ty::Nominal(_, _) => 14,
        noble_kernel::types::Ty::GenericNominal(_, _, _) => 15,
        noble_kernel::types::Ty::LiveRef(_, _, _) => 16,
    }
}

/// Include nested types, program endpoints, effects and resource kinds.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; fold visits the finite Ty tree through private pending scratch and length-guarded program endpoint/effect scans; empty programs and effects are valid digest inputs, not assertion failures."
)]
#[expect(
    tigerstyle::function_length,
    reason = "Owner: noble-contracts; the single-pass dispatcher is kept linear so its stage order stays reviewable"
)]
pub(in crate::companion) fn fold(
    fold: &mut crate::companion::digest::Fold,
    ty: &noble_kernel::types::Ty,
) {
    let mut pending = alloc::vec::Vec::with_capacity(1);
    pending.push(ty);
    while let Some(node) = pending.pop() {
        fold.absorb(ty_tag(node));
        match node {
            noble_kernel::types::Ty::Pair(a, b) | noble_kernel::types::Ty::Sum(a, b) => {
                pending.push(b);
                pending.push(a);
            }
            noble_kernel::types::Ty::List(element) => pending.push(element),
            noble_kernel::types::Ty::Program(input, output, effects)
            | noble_kernel::types::Ty::LiveRef(input, output, effects) => {
                fold.absorb_count(input.len());
                fold.absorb_count(output.len());
                fold.absorb_count(effects.as_slice().len());
                let mut at = 0usize;
                while at < effects.as_slice().len() {
                    fold.absorb(u64::from(effects.as_slice()[at].0));
                    at = at.saturating_add(1);
                }
                at = output.len();
                while at > 0 {
                    at -= 1;
                    pending.push(&output[at]);
                }
                at = input.len();
                while at > 0 {
                    at -= 1;
                    pending.push(&input[at]);
                }
            }
            noble_kernel::types::Ty::Resource(kind) => fold.absorb(u64::from(kind.0)),
            noble_kernel::types::Ty::Nominal(id, shape) => {
                fold.absorb(id.module);
                fold.absorb(u64::from(id.ordinal));
                match shape.as_ref() {
                    noble_kernel::types::NominalShape::Opaque(base) => {
                        fold.absorb(1);
                        pending.push(base);
                    }
                    noble_kernel::types::NominalShape::Variant(left, right) => {
                        fold.absorb(2);
                        pending.push(right);
                        pending.push(left);
                    }
                }
            }
            noble_kernel::types::Ty::GenericNominal(id, arguments, shape) => {
                fold.absorb(id.module);
                fold.absorb(u64::from(id.ordinal));
                pending.push(&arguments[1]);
                pending.push(&arguments[0]);
                match shape.as_ref() {
                    noble_kernel::types::NominalShape::Opaque(base) => {
                        fold.absorb(1);
                        pending.push(base);
                    }
                    noble_kernel::types::NominalShape::Variant(left, right) => {
                        fold.absorb(2);
                        pending.push(right);
                        pending.push(left);
                    }
                }
            }
            noble_kernel::types::Ty::Unit
            | noble_kernel::types::Ty::Bool
            | noble_kernel::types::Ty::I64
            | noble_kernel::types::Ty::Text
            | noble_kernel::types::Ty::Syntax
            | noble_kernel::types::Ty::Contract
            | noble_kernel::types::Ty::Evidence
            | noble_kernel::types::Ty::Certified => {}
        }
    }
}
