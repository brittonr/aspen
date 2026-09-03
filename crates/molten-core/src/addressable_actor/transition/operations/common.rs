use crate::addressable_actor::*;

pub(super) fn wake_effect(reason: &WakeReason, wake_ref: &str) -> super::super::EffectDraft {
    let kind = match reason {
        WakeReason::Message { .. } => ActorEffectIntentKind::DeliverMessage,
        WakeReason::Timer { .. } => ActorEffectIntentKind::InvokeTimer,
        WakeReason::Connection { .. } => ActorEffectIntentKind::AcceptConnection,
        WakeReason::Operator { .. } => ActorEffectIntentKind::NotifyOperator,
    };
    draft(kind, Some(wake_ref.to_string()), Some(reason.subject_ref().to_string()))
}

pub(super) fn change(
    kind: ActorTransitionKind,
    next: ActorState,
    effects: Vec<super::super::EffectDraft>,
) -> super::super::PlannedChange {
    super::super::PlannedChange {
        kind,
        next,
        effects,
        restored_classes: Vec::new(),
    }
}

pub(super) fn draft(
    kind: ActorEffectIntentKind,
    wake_ref: Option<String>,
    subject_ref: Option<String>,
) -> super::super::EffectDraft {
    super::super::EffectDraft {
        kind,
        wake_ref,
        subject_ref,
    }
}

pub(super) fn illegal(phase: ActorPhase, operation: &str) -> ActorIssue {
    ActorIssue::IllegalPhase {
        phase,
        operation: operation.to_string(),
    }
}
