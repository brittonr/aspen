use crate::error::MoltenError;
use crate::error::Result;

pub const MAX_CHAOSCONTROL_CLIENT_SESSION_BYTES: usize = 256;

// One logical operation keeps its client-session and sequence identity across
// acknowledgement, definite rejection, timeout, disconnect, retry, and
// recovery. Two operations with the same identity are the same operation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChaosControlLogicalOperation {
    pub client_session: String,
    pub sequence: u64,
}

// Client-visible proposal outcomes. Timeout, disconnect, and process loss map
// to indefinite: they can never become definite non-execution evidence until
// committed history resolves the operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosControlProposalOutcome {
    Acknowledged { committed_index: u64 },
    DefinitelyRejected,
    Indefinite,
}

impl ChaosControlProposalOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Acknowledged { .. } => "acknowledged",
            Self::DefinitelyRejected => "definitely-rejected",
            Self::Indefinite => "indefinite",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChaosControlTransportObservation {
    AcknowledgementReceipt { committed_index: u64 },
    DefiniteRejectionReceipt,
    Timeout,
    Disconnect,
    ProcessLoss,
}

// r[impl molten.consensus.chaoscontrol_operation_identity]
pub const fn map_transport_observation(observation: ChaosControlTransportObservation) -> ChaosControlProposalOutcome {
    match observation {
        ChaosControlTransportObservation::AcknowledgementReceipt { committed_index } => {
            ChaosControlProposalOutcome::Acknowledged { committed_index }
        }
        ChaosControlTransportObservation::DefiniteRejectionReceipt => ChaosControlProposalOutcome::DefinitelyRejected,
        ChaosControlTransportObservation::Timeout
        | ChaosControlTransportObservation::Disconnect
        | ChaosControlTransportObservation::ProcessLoss => ChaosControlProposalOutcome::Indefinite,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosControlProposalAttempt {
    pub operation: ChaosControlLogicalOperation,
    pub operation_ref: String,
    pub outcome: ChaosControlProposalOutcome,
}

// r[impl molten.consensus.chaoscontrol_operation_identity]
// Admits one operation's attempt trace. Every attempt of one logical
// operation must keep the same client-session and sequence identity; an
// indefinite outcome can never later become a definite rejection; and two
// acknowledgements must name the same committed index so one logical
// operation applies at most once.
pub fn admit_proposal_attempts(attempts: &[ChaosControlProposalAttempt]) -> Result<()> {
    if attempts.is_empty() {
        return Err(MoltenError::invalid_harness(
            "ChaosControl operation trace requires at least one proposal attempt",
        ));
    }
    let identity = &attempts[0].operation;
    validate_logical_operation(identity)?;
    let mut acknowledged_index: Option<u64> = None;
    let mut saw_indefinite = false;
    for attempt in attempts {
        if &attempt.operation != identity {
            return Err(MoltenError::invalid_harness(
                "ChaosControl operation retry changed client-session or sequence identity, invalid idempotency input",
            ));
        }
        if attempt.operation_ref != attempts[0].operation_ref {
            return Err(MoltenError::invalid_harness(
                "ChaosControl operation trace mixes operation refs for one logical operation",
            ));
        }
        match attempt.outcome {
            ChaosControlProposalOutcome::Acknowledged { committed_index } => match acknowledged_index {
                None => acknowledged_index = Some(committed_index),
                Some(observed) if observed == committed_index => {}
                Some(observed) => {
                    return Err(MoltenError::invalid_harness(
                        "ChaosControl retry acknowledges a different committed index, operation identity violated",
                    ));
                }
            },
            ChaosControlProposalOutcome::DefinitelyRejected => {
                if saw_indefinite {
                    return Err(MoltenError::invalid_harness(
                        "ChaosControl indefinite outcome cannot become definite non-execution evidence",
                    ));
                }
            }
            ChaosControlProposalOutcome::Indefinite => saw_indefinite = true,
        }
    }
    Ok(())
}

fn validate_logical_operation(operation: &ChaosControlLogicalOperation) -> Result<()> {
    if operation.client_session.is_empty() || operation.client_session.len() > MAX_CHAOSCONTROL_CLIENT_SESSION_BYTES {
        return Err(MoltenError::invalid_harness(
            "ChaosControl client session identity is empty or exceeds the bounded maximum",
        ));
    }
    if operation.sequence == 0 {
        return Err(MoltenError::invalid_harness("ChaosControl operation sequence must start at one"));
    }
    Ok(())
}
