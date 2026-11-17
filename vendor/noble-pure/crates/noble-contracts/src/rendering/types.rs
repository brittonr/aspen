//! Types retain order and structure; effects in this contract fragment are empty.

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending a type declaration requires the caller's growable String, not a fixed str slice."
)]
pub(super) fn named_stack(
    out: &mut alloc::string::String,
    name: &str,
    entries: &[crate::NamedType],
) {
    out.push_str("def ");
    out.push_str(name);
    out.push_str(" : List Ty := [");
    let mut index = 0usize;
    while index < entries.len() {
        if index != 0 {
            out.push_str(", ");
        }
        value(out, &entries[index].ty);
        index += 1;
    }
    out.push_str("]\n");
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending a type list requires the caller's growable String, not a fixed str slice."
)]
fn stack(out: &mut alloc::string::String, entries: &[noble_kernel::types::Ty]) {
    out.push('[');
    stack_entries(out, entries, 0);
    out.push(']');
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending type entries requires the caller's growable String, not a fixed str slice."
)]
#[expect(
    tigerstyle::no_recursion,
    reason = "Owner: noble-maintainers. Each call advances within a program type capped at 256 constructors; direct tail recursion keeps the type cycle out of Aeneas's higher-order loop combinator."
)]
#[expect(
    tigerstyle::raw_arithmetic_overflow,
    reason = "Owner: noble-maintainers; index < entries.len() implies index < usize::MAX, so the immediate recursive successor cannot overflow."
)]
fn stack_entries(
    out: &mut alloc::string::String,
    entries: &[noble_kernel::types::Ty],
    index: usize,
) {
    // The accepted type-size cap bounds this traversal. A direct tail call
    // keeps mutual type recursion outside Aeneas's higher-order loop combinator.
    if index < entries.len() {
        if index != 0 {
            out.push_str(", ");
        }
        value(out, &entries[index]);
        stack_entries(out, entries, index + 1);
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending a type constructor requires the caller's growable String, not a fixed str slice."
)]
#[expect(
    tigerstyle::no_recursion,
    reason = "Owner: noble-maintainers. Child calls strictly descend a prepared type capped at 256 constructors; direct recursion preserves pinned Aeneas extraction without a borrowed work stack."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; type serialization grows the caller's String through non-const push and push_str."
)]
fn value(out: &mut alloc::string::String, ty: &noble_kernel::types::Ty) {
    match ty {
        noble_kernel::types::Ty::Unit => out.push_str(".unit"),
        noble_kernel::types::Ty::Bool => out.push_str(".bool"),
        noble_kernel::types::Ty::I64 => out.push_str(".i64"),
        noble_kernel::types::Ty::Text => out.push_str(".text"),
        noble_kernel::types::Ty::Syntax => {
            // Scalar delimiters avoid the pinned printer's invalid UTF-8
            // string escapes without changing the reviewed constructor name.
            out.push('.');
            out.push(char::from(0xab));
            out.push_str("syntax");
            out.push(char::from(0xbb));
        }
        noble_kernel::types::Ty::Contract => out.push_str("Contract"),
        noble_kernel::types::Ty::Evidence => out.push_str("Evidence"),
        noble_kernel::types::Ty::Certified => out.push_str("Certified"),
        noble_kernel::types::Ty::Pair(a, b) => binary(out, ".pair", a, b),
        noble_kernel::types::Ty::Sum(a, b) => binary(out, ".sum", a, b),
        noble_kernel::types::Ty::List(item) => {
            out.push_str("(.list ");
            value(out, item);
            out.push(')');
        }
        noble_kernel::types::Ty::Program(inputs, outputs, effects) => {
            if !effects.is_empty() {
                super::rejected(out, "unsupported host effect in contract");
            } else {
                out.push_str("(.program ");
                stack(out, inputs);
                out.push(' ');
                stack(out, outputs);
                out.push(')');
            }
        }
        noble_kernel::types::Ty::LiveRef(..) => {
            super::rejected(
                out,
                "borrowed live reference cannot be serialized in a contract",
            );
        }
        noble_kernel::types::Ty::Resource(_) => {
            super::rejected(out, "unsupported resource in contract");
        }
        noble_kernel::types::Ty::Nominal(_, _) => {
            super::rejected(out, "unsupported nominal type in companion contract");
        }
        noble_kernel::types::Ty::GenericNominal(_, _, _) => {
            super::rejected(
                out,
                "unsupported generic nominal type in companion contract",
            );
        }
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending a binary type requires the caller's growable String, not a fixed str slice."
)]
fn binary(
    out: &mut alloc::string::String,
    name: &str,
    a: &noble_kernel::types::Ty,
    b: &noble_kernel::types::Ty,
) {
    out.push('(');
    out.push_str(name);
    out.push(' ');
    value(out, a);
    out.push(' ');
    value(out, b);
    out.push(')');
}
