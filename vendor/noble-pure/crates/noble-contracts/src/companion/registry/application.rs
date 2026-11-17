/// Check that an evidence entry applies to exactly this subject under the
/// current policy (VC-VALUE-03). A policy or context change invalidates the
/// decision: `StaleContext`.
///
/// Applicability compares the complete retained subject or a complete
/// independently folded observation with exactly its program operations and
/// endpoint signatures. Neither a digest match nor a cache lookup establishes
/// equality. Every mismatch is `MismatchedSubject`.
pub(crate) fn bind(
    engine: &crate::companion::Core,
    evidence: crate::companion::registry::EvidenceId,
    subject: &crate::companion::Subject,
) -> Result<crate::companion::Subject, crate::companion::Refusal> {
    let entry = attempt!(proved(engine, evidence));
    if entry.subject == *subject {
        return Ok(entry.subject.clone());
    }
    attempt!(matches_observation(&entry.subject, subject));
    Ok(entry.subject.clone())
}

/// Every authority-consuming path requires a current proved premise.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; proved checks evidence and contract membership, then their retained contexts and proved class in order; stale or unproved premises must return typed refusals rather than panic."
)]
pub(crate) fn proved(
    engine: &crate::companion::Core,
    id: crate::companion::registry::EvidenceId,
) -> Result<&crate::companion::registry::EvidenceEntry, crate::companion::Refusal> {
    let entry = attempt!(engine.registry.evidence(id));
    let contract = attempt!(engine.registry.contract(entry.contract));
    if crate::companion::registry::Registry::stale(
        engine,
        crate::companion::registry::ContextSnapshot {
            policy: entry.policy,
            revision: entry.revision,
        },
    ) || crate::companion::registry::Registry::stale(
        engine,
        crate::companion::registry::ContextSnapshot {
            policy: contract.policy,
            revision: contract.revision,
        },
    ) {
        return Err(crate::companion::Refusal::StaleContext);
    }
    if entry.outcome != crate::companion::admit::Outcome::Proved
        || !matches!(
            entry.class,
            crate::companion::registry::EvidenceClass::LeanExact
                | crate::companion::registry::EvidenceClass::Replay
        )
    {
        return Err(crate::companion::Refusal::WrongPremiseClass);
    }
    Ok(entry)
}

/// Complete observations, not attacker-supplied identity fields, establish
/// recipe equality. A truncated observation cannot round-trip this check.
#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; matches_observation checks full subjects, endpoint signatures, the complete independent observation fold and ordered program operations; mismatches are typed refusals, not assertion failures."
)]
pub(crate) fn matches_observation(
    expected: &crate::companion::Subject,
    offered: &crate::companion::Subject,
) -> Result<(), crate::companion::Refusal> {
    if expected == offered {
        return Ok(());
    }
    if offered.input_signature != expected.input_signature
        || offered.output_signature != expected.output_signature
    {
        return Err(crate::companion::Refusal::MismatchedSubject);
    }
    if crate::companion::subject::observe(
        &offered.events,
        crate::companion::InterfaceSignatures {
            input: offered.input_signature,
            output: offered.output_signature,
        },
    ) != *offered
    {
        return Err(crate::companion::Refusal::MismatchedSubject);
    }
    if op_stream(&offered.events) != op_stream(&expected.events) {
        return Err(crate::companion::Refusal::MismatchedSubject);
    }
    Ok(())
}

/// The program-op subsequence of an observed recipe event list. Structural and
/// interface observation atoms are dropped so a guest's extra interface events
/// do not change program identity; every program op (literal, invocation,
/// quotation, aggregate-op) is kept in order.
pub(crate) fn op_stream(events: &[(u32, u64)]) -> alloc::vec::Vec<(u32, u64)> {
    let mut out = alloc::vec::Vec::with_capacity(events.len());
    let mut index = 0usize;
    while index < events.len() {
        let event = events[index];
        if is_program_op(event.0) {
            out.push(event);
        }
        index = index.saturating_add(1);
    }
    out
}

const fn is_program_op(kind: u32) -> bool {
    matches!(kind, 1..=7)
}
