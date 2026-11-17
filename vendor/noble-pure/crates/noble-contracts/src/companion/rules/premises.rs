#![expect(
    tigerstyle::mutating_input_in_pure,
    reason = "Owner: noble-maintainers; premise traversal mutates only the operation-owned work meter and scratch queue; the registry and borrowed derivation remain immutable"
)]

mod step;

pub(super) fn subject_work(
    budget: &mut crate::companion::Budget,
    subject: &crate::companion::Subject,
) -> Result<(), crate::companion::Refusal> {
    let count = subject
        .events
        .len()
        .saturating_add(subject.captures.len())
        .saturating_add(1);
    let count = match u32::try_from(count) {
        Ok(count) => count,
        Err(_) => return Err(crate::companion::Refusal::ExhaustedReplay),
    };
    attempt!(budget.charge(count, crate::companion::Refusal::ExhaustedReplay));
    if subject.events.len() > crate::companion::subject::OBSERVATION_CAP
        || subject.captures.len() > crate::companion::subject::OBSERVATION_CAP
    {
        return Err(crate::companion::Refusal::ExhaustedReplay);
    }
    Ok(())
}

/// crate::companion::registry::Registry edges always point to already-existing entries. Checking this
/// strict order over the reachable closure proves acyclicity without a DFS
/// whose separately seeded roots could incorrectly look like back edges.
pub(super) fn validate(
    engine: &crate::companion::Core,
    premises: &[crate::companion::registry::EvidenceId],
    budget: &mut crate::companion::Budget,
    should_reject_duplicates: bool,
) -> Result<(), crate::companion::Refusal> {
    let pending = attempt!(seed(engine, premises, budget, should_reject_duplicates));
    drain(engine, pending, budget)
}

#[expect(
    tigerstyle::assertion_density,
    reason = "Owner: noble-maintainers; seed bounds indexing by the offered premise slice and stops on the first charged seed-step refusal, preserving cycle, missing-premise, duplicate and queue-exhaustion outcomes instead of panicking."
)]
fn seed(
    engine: &crate::companion::Core,
    premises: &[crate::companion::registry::EvidenceId],
    budget: &mut crate::companion::Budget,
    should_reject_duplicates: bool,
) -> Result<alloc::vec::Vec<crate::companion::registry::EvidenceId>, crate::companion::Refusal> {
    let next = attempt!(engine.registry.next_evidence());
    let entry_bound = attempt!(queue_capacity(engine.limits));
    let bounds = step::SeedBounds {
        next,
        entry_bound,
        should_reject_duplicates,
    };
    let mut pending = alloc::vec::Vec::with_capacity(premises.len().min(entry_bound));
    let mut index = 0usize;
    let mut failure = None;
    while index < premises.len() {
        if let Err(refusal) = step::seed(premises, index, bounds, &mut pending, budget) {
            failure = Some(refusal);
            break;
        }
        index = index.saturating_add(1);
    }
    match failure {
        Some(refusal) => Err(refusal),
        None => Ok(pending),
    }
}

fn drain(
    engine: &crate::companion::Core,
    mut pending: alloc::vec::Vec<crate::companion::registry::EvidenceId>,
    budget: &mut crate::companion::Budget,
) -> Result<(), crate::companion::Refusal> {
    let entry_bound = attempt!(queue_capacity(engine.limits));
    let mut failure = None;
    while let Some(id) = pending.pop() {
        if let Err(refusal) = step::drain(engine, id, &mut pending, budget, entry_bound) {
            failure = Some(refusal);
            break;
        }
    }
    finish(failure)
}

fn unique_premise(
    premises: &[crate::companion::registry::EvidenceId],
    index: usize,
    budget: &mut crate::companion::Budget,
) -> Result<(), crate::companion::Refusal> {
    let mut prior = 0usize;
    let mut failure = None;
    while prior < index {
        if let Err(refusal) = step::unique(premises, prior, index, budget) {
            failure = Some(refusal);
            break;
        }
        prior = prior.saturating_add(1);
    }
    finish(failure)
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the operation-owned bounded premise queue must grow, requiring Vec rather than a slice."
)]
fn enqueue_ancestors(
    entry: &crate::companion::registry::EvidenceEntry,
    id: crate::companion::registry::EvidenceId,
    pending: &mut alloc::vec::Vec<crate::companion::registry::EvidenceId>,
    budget: &mut crate::companion::Budget,
    entry_bound: usize,
) -> Result<(), crate::companion::Refusal> {
    let mut at = 0usize;
    let mut failure = None;
    while at < entry.premises.len() {
        if let Err(refusal) = step::ancestor(entry.premises[at], id, pending, budget, entry_bound) {
            failure = Some(refusal);
            break;
        }
        at = at.saturating_add(1);
    }
    finish(failure)
}

const fn finish(
    failure: Option<crate::companion::Refusal>,
) -> Result<(), crate::companion::Refusal> {
    match failure {
        Some(refusal) => Err(refusal),
        None => Ok(()),
    }
}

#[expect(
    tigerstyle::missing_const_fn,
    reason = "Owner: noble-maintainers; queue_capacity uses non-const TryFrom to reject work bounds that cannot represent the premise queue's entry capacity on the current target."
)]
fn queue_capacity(limits: crate::Limits) -> Result<usize, crate::companion::Refusal> {
    match usize::try_from(limits.work) {
        Ok(entry_bound) => Ok(entry_bound),
        Err(_) => Err(crate::companion::Refusal::ExhaustedReplay),
    }
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; only the freshly allocated scratch queue grows, and a slice cannot express its checked push."
)]
fn queue(
    pending: &mut alloc::vec::Vec<crate::companion::registry::EvidenceId>,
    id: crate::companion::registry::EvidenceId,
    entry_bound: usize,
) -> Result<(), crate::companion::Refusal> {
    if pending.len() >= entry_bound {
        return Err(crate::companion::Refusal::ExhaustedReplay);
    }
    pending.push(id);
    Ok(())
}
