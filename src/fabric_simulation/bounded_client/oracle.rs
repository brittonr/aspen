use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoverySource {
    /// Read back before admitting peer ingress, bound to the owner's recovery receipt.
    Local(String),
    /// The owner explicitly identified the repair source, not merely a successful read.
    Peer {
        participant: String,
        receipt_ref: String,
    },
    /// An explicit local inventory found no state. This is not global absence.
    Absent(String),
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultApplication {
    Requested,
    Applied,
    Unsupported,
}

/// Independent observations, never a service-supplied list of passing properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub participant: String,
    pub receipt_ref: String,
    pub request_ref: String,
    pub is_receipt_bound: bool,
    pub is_ready: bool,
    pub has_membership: bool,
    pub acknowledged: bool,
    pub application_count: u64,
    pub snapshot_restored: bool,
    pub recovered_acknowledgement: bool,
    pub persistence: Option<molten_core::fabric_durability::PersistenceObservation>,
    pub requested_durability: molten_core::fabric_durability::DurabilityLevel,
    pub health_before: molten_core::fabric_durability::AdapterHealth,
    pub recovery: RecoverySource,
    pub start_term: u64,
    pub end_term: Option<u64>,
    pub observed_ticks: u64,
    pub is_fault_off: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    pub input_ref: String,
    pub executed_requests: Vec<String>,
    pub fault: FaultApplication,
    pub participants: Vec<Participant>,
    pub deployed_control: SemanticControl,
    pub is_control_reached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail { property: String, participant: String },
    NotEvaluated { reason: String },
    Incomplete { reason: String },
    Invalid { reason: String },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub verdict: Verdict,
    pub liveness: Vec<molten_core::protocol_oracle::LivenessResult>,
}

fn invalid(reason: &str) -> Check {
    Check {
        verdict: Verdict::Invalid {
            reason: reason.to_string(),
        },
        liveness: Vec::new(),
    }
}

// r[impl molten.chaoscontrol_client.observation]
// r[impl molten.chaoscontrol_client.controls]
pub fn check_history(input: &Input, history: &History, is_apply_once_checked: bool) -> Check {
    if input.validate().is_err() || input_ref(input).ok().as_deref() != Some(history.input_ref.as_str()) {
        return invalid("input binding mismatch");
    }
    if input.control != history.deployed_control {
        return invalid("semantic control deployment mismatch");
    }
    if !history
        .executed_requests
        .iter()
        .map(String::as_str)
        .eq(input.steps.iter().map(|step| step.request_ref.as_str()))
    {
        return without_liveness(Verdict::Incomplete {
            reason: "commands not executed".to_string(),
        });
    }
    if let Err(check) = validate_participants(input, history, is_apply_once_checked) {
        return check;
    }
    // A suppressed required property never silently qualifies a mutant or healthy history.
    if !is_apply_once_checked {
        return invalid("required apply-once property suppressed");
    }
    if history.participants.len() != input.participants.len() {
        return without_liveness(Verdict::Incomplete {
            reason: "missing participant".to_string(),
        });
    }
    if !input.is_complete() {
        return without_liveness(Verdict::Incomplete {
            reason: "valid prefix has no complete recovery phase".to_string(),
        });
    }
    match history.fault {
        FaultApplication::Unsupported => return without_liveness(Verdict::Unsupported),
        FaultApplication::Requested => {
            return without_liveness(Verdict::NotEvaluated {
                reason: "requested fault was not observed applied".to_string(),
            });
        }
        FaultApplication::Applied => {}
    }
    evaluate_recovery(input, history)
}

fn without_liveness(verdict: Verdict) -> Check {
    Check {
        verdict,
        liveness: Vec::new(),
    }
}

fn validate_participants(input: &Input, history: &History, is_apply_once_checked: bool) -> Result<(), Check> {
    let mut seen = std::collections::BTreeSet::new();
    for participant in &history.participants {
        if !input.participants.contains(&participant.participant) || !seen.insert(&participant.participant) {
            return Err(invalid("unexpected or duplicate participant"));
        }
        if !participant.is_receipt_bound
            || crate::preserves_rail::validate_content_ref(&participant.receipt_ref).is_err()
            || participant.request_ref != input.submit_request_ref
        {
            return Err(invalid("fabricated or mismatched participant receipt"));
        }
        validate_recovery_source(input, participant)?;
        if is_apply_once_checked && participant.application_count > 1 {
            return Err(without_liveness(Verdict::Fail {
                property: APPLY_ONCE_PROPERTY.to_string(),
                participant: participant.participant.clone(),
            }));
        }
    }
    Ok(())
}

fn validate_recovery_source(input: &Input, participant: &Participant) -> Result<(), Check> {
    match &participant.recovery {
        RecoverySource::Local(reference) | RecoverySource::Absent(reference) => {
            if crate::preserves_rail::validate_content_ref(reference).is_err() {
                return Err(invalid("invalid recovery receipt"));
            }
        }
        RecoverySource::Peer {
            participant: source,
            receipt_ref,
        } => {
            if source == &participant.participant
                || !input.participants.contains(source)
                || crate::preserves_rail::validate_content_ref(receipt_ref).is_err()
            {
                return Err(invalid("invalid peer recovery source"));
            }
        }
        RecoverySource::Unavailable => {}
    }
    Ok(())
}

fn stabilization(participant: &Participant) -> molten_core::protocol_oracle::ParticipantStabilization {
    let classification = participant.persistence.map(|observation| {
        molten_core::fabric_durability::classify_persistence(participant.requested_durability, observation)
    });
    let health = classification.map_or(participant.health_before, |classification| {
        molten_core::fabric_durability::health_after(participant.health_before, classification)
    });
    molten_core::protocol_oracle::ParticipantStabilization {
        participant: participant.participant.clone(),
        is_ready: participant.is_ready,
        has_required_membership: participant.has_membership,
        is_disruptive_fault_active: !participant.is_fault_off,
        is_durability_satisfied: matches!(
            classification,
            Some(molten_core::fabric_durability::PersistenceClassification::Acknowledged(_))
        ) && health == molten_core::fabric_durability::AdapterHealth::Serving,
        start_progress: participant.start_term,
        end_progress: participant.end_term,
        observed_ticks: participant.observed_ticks,
    }
}

fn evaluate_recovery(input: &Input, history: &History) -> Check {
    let facts = history.participants.iter().map(stabilization).collect::<Vec<_>>();
    let liveness = molten_core::protocol_oracle::evaluate_cohort_liveness(
        &molten_core::protocol_oracle::LivenessPreconditions {
            requires_readiness: true,
            requires_membership: true,
            disruptive_faults_block: true,
            requires_durability: true,
            virtual_progress_horizon: input.recovery_ticks,
        },
        &facts,
    );
    let verdict = history
        .participants
        .iter()
        .zip(&liveness)
        .find_map(|(participant, progress)| recovery_verdict(participant, progress))
        .unwrap_or(Verdict::Pass);
    Check { verdict, liveness }
}

fn recovery_verdict(
    participant: &Participant,
    progress: &molten_core::protocol_oracle::LivenessResult,
) -> Option<Verdict> {
    if matches!(participant.recovery, RecoverySource::Unavailable) || participant.persistence.is_none() {
        return Some(Verdict::Incomplete {
            reason: format!("{} recovery/storage observation unavailable", participant.participant),
        });
    }
    if participant.acknowledged
        && (!participant.recovered_acknowledgement || matches!(participant.recovery, RecoverySource::Absent(_)))
    {
        return Some(Verdict::Fail {
            property: "molten.client.acknowledged-recovery.v1".to_string(),
            participant: participant.participant.clone(),
        });
    }
    if !participant.acknowledged || (participant.application_count == 0 && !participant.snapshot_restored) {
        return Some(Verdict::Incomplete {
            reason: "no acknowledged application observed".to_string(),
        });
    }
    match progress.verdict {
        molten_core::protocol_oracle::LivenessVerdict::Pass => None,
        molten_core::protocol_oracle::LivenessVerdict::Fail => Some(Verdict::Fail {
            property: "molten.client.participant-progress.v1".to_string(),
            participant: participant.participant.clone(),
        }),
        molten_core::protocol_oracle::LivenessVerdict::NotEvaluated(condition) => Some(Verdict::NotEvaluated {
            reason: condition.as_str().to_string(),
        }),
        molten_core::protocol_oracle::LivenessVerdict::Incomplete(condition) => Some(Verdict::Incomplete {
            reason: condition.as_str().to_string(),
        }),
    }
}
