//! Fallible individual traversal steps, outside loop continuations.

/// Registry order and queue-entry bounds fixed for one premise-seeding pass.
#[derive(Clone, Copy)]
pub(super) struct SeedBounds {
    pub(super) next: u32,
    pub(super) entry_bound: usize,
    pub(super) should_reject_duplicates: bool,
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the bounded operation-owned premise queue must grow, requiring Vec rather than a slice."
)]
pub(super) fn seed(
    premises: &[crate::companion::EvidenceId],
    index: usize,
    bounds: SeedBounds,
    pending: &mut alloc::vec::Vec<crate::companion::EvidenceId>,
    budget: &mut crate::companion::Budget,
) -> Result<(), crate::companion::Refusal> {
    attempt!(budget.charge(1, crate::companion::Refusal::ExhaustedReplay));
    let id = premises[index];
    if id.0 == bounds.next {
        return Err(crate::companion::Refusal::CyclicDerivation);
    }
    if id.0 > bounds.next {
        return Err(crate::companion::Refusal::MissingPremise);
    }
    if bounds.should_reject_duplicates {
        attempt!(super::unique_premise(premises, index, budget));
    }
    super::queue(pending, id, bounds.entry_bound)
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; only the freshly allocated traversal queue grows during this bounded registry-read step."
)]
pub(super) fn drain(
    engine: &crate::companion::Core,
    id: crate::companion::EvidenceId,
    pending: &mut alloc::vec::Vec<crate::companion::EvidenceId>,
    budget: &mut crate::companion::Budget,
    entry_bound: usize,
) -> Result<(), crate::companion::Refusal> {
    attempt!(budget.charge(1, crate::companion::Refusal::ExhaustedReplay));
    let entry = attempt!(crate::companion::registry::application::proved(engine, id));
    super::enqueue_ancestors(entry, id, pending, budget, entry_bound)
}

#[expect(
    tigerstyle::ambiguous_params,
    reason = "Owner: noble-maintainers; unique charges one comparison and tests symmetric equality at two in-bounds premise positions, so swapping prior and current preserves both the work charge and DuplicatePremise result."
)]
pub(super) fn unique(
    premises: &[crate::companion::EvidenceId],
    prior: usize,
    current: usize,
    budget: &mut crate::companion::Budget,
) -> Result<(), crate::companion::Refusal> {
    attempt!(budget.charge(1, crate::companion::Refusal::ExhaustedReplay));
    if premises[prior] == premises[current] {
        return Err(crate::companion::Refusal::DuplicatePremise);
    }
    Ok(())
}

#[expect(
    tigerstyle::borrowed_argument_types,
    reason = "Owner: noble-maintainers; the operation-owned ancestor queue performs a checked bounded push, which cannot use a slice."
)]
pub(super) fn ancestor(
    premise: crate::companion::EvidenceId,
    parent: crate::companion::EvidenceId,
    pending: &mut alloc::vec::Vec<crate::companion::EvidenceId>,
    budget: &mut crate::companion::Budget,
    entry_bound: usize,
) -> Result<(), crate::companion::Refusal> {
    attempt!(budget.charge(1, crate::companion::Refusal::ExhaustedReplay));
    if premise.0 >= parent.0 {
        return Err(crate::companion::Refusal::CyclicDerivation);
    }
    super::queue(pending, premise, entry_bound)
}
