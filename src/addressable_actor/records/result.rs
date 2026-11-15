//! Application-facing classification; lifecycle state persistence is not semantic completion.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicOutcome {
    Applied,
    DuplicateReplay,
    Progress,
    Denied,
    Stale,
    Unknown,
}

impl PublicOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::DuplicateReplay => "duplicate-replay",
            Self::Progress => "progress",
            Self::Denied => "denied",
            Self::Stale => "stale",
            Self::Unknown => "unknown",
        }
    }
}

/// Preserves the underlying receipt. This projection never authorizes a retry.
// r[impl molten.addressable_actor.public_result]
pub fn public_outcome(outcome: &super::Outcome) -> PublicOutcome {
    if outcome.status_observation.outcome_unknown {
        return PublicOutcome::Unknown;
    }
    if matches!(
        outcome.transition.issue,
        Some(
            molten_core::addressable_actor::ActorIssue::StaleActorKey
                | molten_core::addressable_actor::ActorIssue::StalePlacement
                | molten_core::addressable_actor::ActorIssue::StaleGeneration
                | molten_core::addressable_actor::ActorIssue::StaleLifecycleSequence
        )
    ) {
        return PublicOutcome::Stale;
    }
    match outcome.receipt.status {
        super::ServiceStatus::Stale => return PublicOutcome::Stale,
        super::ServiceStatus::Denied | super::ServiceStatus::EffectAdmissionDenied => return PublicOutcome::Denied,
        // Metadata can precede a refused acknowledgement. Only an original-result
        // observation can establish the native proof and exact terminal delivery.
        super::ServiceStatus::DuplicateReplay => return PublicOutcome::Unknown,
        super::ServiceStatus::EffectFailed
        | super::ServiceStatus::EffectOutcomeUnknown
        | super::ServiceStatus::Unknown
        | super::ServiceStatus::NotAppliedAfterReconciliation => return PublicOutcome::Unknown,
        _ => {}
    }
    let Some(commit) = &outcome.commit_observation else {
        return PublicOutcome::Unknown;
    };
    if !outcome.receipt.status.commit_confirmed()
        || !commit.currentness.is_current()
        || commit.durability != super::DurabilityOutcome::Durable
        || outcome.effect_observations.len() != outcome.transition.effects.len()
        || outcome
            .effect_observations
            .iter()
            .any(|effect| effect.disposition != super::EffectDisposition::Succeeded || effect.outcome_ref.is_none())
    {
        return PublicOutcome::Unknown;
    }
    if outcome.transition.kind == molten_core::addressable_actor::ActorTransitionKind::DeliveryComplete
        && outcome.transition.effects.len() == 1
        && outcome.transition.effects[0].kind
            == molten_core::addressable_actor::ActorEffectIntentKind::AcknowledgeDelivery
    {
        PublicOutcome::Applied
    } else {
        PublicOutcome::Progress
    }
}
