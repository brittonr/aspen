//! Postorder candidate serialization. Preparation owns and checks the arena.

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending candidate syntax requires the caller's growable String, not a fixed str slice."
)]
pub(super) fn emit(
    out: &mut alloc::string::String,
    candidate: &noble_kernel::untrusted::Candidate,
) {
    match crate::wire::lower_subject(candidate) {
        Ok(subject) => resolved(out, &subject),
        Err(_) => {
            out.push_str("def program : List Op := ");
            super::rejected(out, "unsupported semantic projection");
            out.push('\n');
        }
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending resolved node declarations requires the caller's growable String, not a fixed str slice."
)]
fn resolved(out: &mut alloc::string::String, subject: &crate::wire::SemanticSubject) {
    let mut index = 0usize;
    while index < subject.nodes.len() {
        out.push_str("def node_");
        super::number(out, index as u64);
        out.push_str(" : Op := ");
        node(out, &subject.nodes[index]);
        out.push('\n');
        index += 1;
    }
    out.push_str("def program : List Op := ");
    body(out, &subject.body);
    out.push_str("\n\n");
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending an operation requires the caller's growable String, not a fixed str slice."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; operation serialization grows the caller's String through non-const push and push_str."
)]
fn node(out: &mut alloc::string::String, node: &crate::wire::SemanticNode) {
    match node {
        crate::wire::SemanticNode::Word(def) => {
            out.push_str(".word ");
            super::number(out, u64::from(*def));
        }
        crate::wire::SemanticNode::Quotation(nodes) => {
            out.push_str(".block ");
            body(out, nodes);
        }
        crate::wire::SemanticNode::I64(value) => {
            out.push_str(".lit (.i64 (BitVec.ofInt 64 ");
            super::signed(out, *value);
            out.push_str("))");
        }
        crate::wire::SemanticNode::Boolean(true) => out.push_str(".lit (.bool true)"),
        crate::wire::SemanticNode::Boolean(false) => out.push_str(".lit (.bool false)"),
        crate::wire::SemanticNode::UnitValue => out.push_str(".lit .unit"),
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers. Appending an operation list requires the caller's growable String, not a fixed str slice."
)]
fn body(out: &mut alloc::string::String, nodes: &[u32]) {
    out.push('[');
    let mut index = 0usize;
    while index < nodes.len() {
        if index != 0 {
            out.push_str(", ");
        }
        out.push_str("node_");
        super::number(out, u64::from(nodes[index]));
        index += 1;
    }
    out.push(']');
}
