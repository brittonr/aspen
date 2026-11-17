//! Exact bounded projection into the runtime observation operation encoding.
#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; projection mutates only freshly allocated bounded pending/output scratch; the candidate and semantic graph remain immutable"
)]

#[octet::sealed_enum]
enum Pending {
    Node(u32),
    CloseQuotation,
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; canonical_op_events rejects failed lowering and threads the first bounded queue/emission refusal through the loop; malformed or oversized programs must remain typed failures."
)]
pub(crate) fn canonical_op_events(
    candidate: &noble_kernel::untrusted::Candidate,
) -> Result<alloc::vec::Vec<(u32, u64)>, crate::companion::Refusal> {
    let subject = match crate::wire::lower_subject(candidate) {
        Ok(subject) => subject,
        Err(_) => return Err(crate::companion::Refusal::MismatchedSubject),
    };
    let entry_bound = subject
        .nodes
        .len()
        .min(crate::companion::subject::OBSERVATION_CAP);
    let mut out = alloc::vec::Vec::with_capacity(entry_bound);
    let mut pending = alloc::vec::Vec::with_capacity(entry_bound);
    attempt!(enqueue(&subject.body, &mut pending));
    let mut failure = None;
    while let Some(item) = pending.pop() {
        if let Err(refusal) = emit(&subject, item, &mut pending, &mut out) {
            failure = Some(refusal);
            break;
        }
    }
    match failure {
        Some(refusal) => Err(refusal),
        None => Ok(out),
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; one bounded projection step grows only its private scratch queue and output buffers."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; emit grows the runtime output Vec for quotation closure or delegates to emit_node's owned pending/output Vec transition."
)]
fn emit(
    subject: &crate::wire::SemanticSubject,
    item: Pending,
    pending: &mut alloc::vec::Vec<Pending>,
    out: &mut alloc::vec::Vec<(u32, u64)>,
) -> Result<(), crate::companion::Refusal> {
    if out.len() >= crate::companion::subject::OBSERVATION_CAP {
        return Err(crate::companion::Refusal::ExhaustedReplay);
    }
    match item {
        Pending::CloseQuotation => {
            out.push((6, 0));
            Ok(())
        }
        Pending::Node(index) => emit_node(subject, index, pending, out),
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the bounded operation-owned queue must grow, which requires its owned Vec rather than a slice."
)]
fn enqueue(
    body: &[u32],
    pending: &mut alloc::vec::Vec<Pending>,
) -> Result<(), crate::companion::Refusal> {
    if pending.len().saturating_add(body.len()) > crate::companion::subject::OBSERVATION_CAP {
        return Err(crate::companion::Refusal::ExhaustedReplay);
    }
    let mut at = body.len();
    while at > 0 {
        at -= 1;
        pending.push(Pending::Node(body[at]));
    }
    Ok(())
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; projection grows only the bounded operation-owned queue and output buffers, so slice arguments cannot implement this transition."
)]
#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; emit_node uses non-const usize::try_from for checked node lookup and runtime Vec pushes to emit operations and schedule quotation closure."
)]
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; emit_node rejects unrepresentable or missing node indices and quotation queue exhaustion before scheduling children; hostile references return MismatchedSubject or ExhaustedReplay, not assertion panics."
)]
fn emit_node(
    subject: &crate::wire::SemanticSubject,
    index: u32,
    pending: &mut alloc::vec::Vec<Pending>,
    out: &mut alloc::vec::Vec<(u32, u64)>,
) -> Result<(), crate::companion::Refusal> {
    let index = match usize::try_from(index) {
        Ok(index) => index,
        Err(_) => return Err(crate::companion::Refusal::MismatchedSubject),
    };
    let node = match subject.nodes.get(index) {
        Some(node) => node,
        None => return Err(crate::companion::Refusal::MismatchedSubject),
    };
    match node {
        crate::wire::SemanticNode::I64(value) => {
            out.push((1, u64::from_ne_bytes(value.to_ne_bytes())))
        }
        crate::wire::SemanticNode::Boolean(value) => out.push((4, u64::from(*value))),
        crate::wire::SemanticNode::UnitValue => out.push((5, 0)),
        crate::wire::SemanticNode::Word(definition) => out.push((2, u64::from(*definition))),
        crate::wire::SemanticNode::Quotation(body) => {
            if pending.len() >= crate::companion::subject::OBSERVATION_CAP {
                return Err(crate::companion::Refusal::ExhaustedReplay);
            }
            out.push((3, 0));
            pending.push(Pending::CloseQuotation);
            attempt!(enqueue(body, pending));
        }
    }
    Ok(())
}
