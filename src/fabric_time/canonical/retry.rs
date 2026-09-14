use super::CanonicalTimeEvent;
use super::CanonicalTimeEventKind;
use super::canonical_event;
use super::field;
use crate::error::Result;
use crate::fabric_time::RetryPlan;
use crate::preserves_rail::string;

// r[impl molten.audit_f12.validation]
pub(in crate::fabric_time) fn canonical_retry_events(
    profile_ref: &str,
    plan: &RetryPlan,
) -> Result<Vec<CanonicalTimeEvent>> {
    [
        ("retry-delay", plan.delay.ticks, plan.delay.domain),
        ("retry-planned", plan.deadline.target.ticks(), plan.deadline.target.domain()),
    ]
    .into_iter()
    .map(|(action, ticks, domain)| {
        canonical_event(
            profile_ref,
            CanonicalTimeEventKind::Deadline,
            plan.deadline.generation,
            &plan.deadline.subject_id,
            action,
            ticks,
            vec![field("domain", string(domain.as_str()))],
            &["explicit-input", "generation-bound", "domain-bound", "bounded-evidence"],
        )
    })
    .collect()
}
