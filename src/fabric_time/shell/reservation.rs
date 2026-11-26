//! Shell ownership for command-pattern resource reservations.
//!
//! The pure core owns capacity, worker selection, and allocation decisions.
//! This shell executes only the decided effects: a rejected command performs no
//! allocation, and a committed command allocates exactly once.

use super::*;

/// The allocation effect the shell executes for a committed reservation.
pub trait EffectPort {
    fn allocate(&mut self, reservation: &ReservationRecord) -> crate::fabric::FabricPortResult<()>;
}

/// One admitted reservation decision, including the decided effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub outcome: ReservationOutcome,
    pub reservation: Option<ReservationRecord>,
    pub issue: Option<ReservationIssue>,
}

impl Decision {
    fn rejected(issue: ReservationIssue) -> Self {
        Self {
            outcome: ReservationOutcome::Rejected,
            reservation: None,
            issue: Some(issue),
        }
    }
}

/// One admitted batch item decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemOutcome {
    pub job_id: String,
    pub decision: Decision,
}

/// The shell-owned reservation state and effect boundary.
pub struct Owner<P: EffectPort> {
    state: ReservationState_,
    effects: P,
}

impl<P: EffectPort> Owner<P> {
    pub fn new(state: ReservationState_, effects: P) -> Self {
        Self { state, effects }
    }

    pub fn state(&self) -> &ReservationState_ {
        &self.state
    }

    pub fn effects_mut(&mut self) -> &mut P {
        &mut self.effects
    }

    /// Evaluates one command against current authoritative state and executes
    /// the decided allocation only when the command commits.
    // r[impl molten.reservation_command.atomic_eval]
    // r[impl molten.reservation_command.pure_core]
    pub fn reserve(&mut self, request: &ReservationRequest) -> crate::error::Result<Decision> {
        let transition = apply_reservation(&self.state, request).map_err(denial_error)?;
        match transition.outcome {
            ReservationOutcome::Committed => self.allocate_and_commit(transition),
            ReservationOutcome::Rejected => {
                let issue = transition
                    .issue
                    .ok_or_else(|| crate::error::Failure::invalid_harness("rejected reservation carries no issue"))?;
                Ok(Decision::rejected(issue))
            }
        }
    }

    /// Admits a bounded batch and evaluates each item as its own command.
    ///
    /// Every committed item is allocated and then committed to authoritative
    /// state before the next item runs, so a later item's adapter failure never
    /// reverts capacity that an earlier item already allocated externally. The
    /// first allocation failure stops the batch and is returned with its port
    /// class preserved; the items committed before it remain in `state()`.
    // r[impl molten.reservation_command.bounded_batches]
    // r[impl molten.fabric_time.batch_item_atomicity_and_fenced_renew]
    pub fn reserve_batch(&mut self, request: &BatchRequest) -> crate::error::Result<Vec<ItemOutcome>> {
        let admitted = admit_batch(&self.state, request).map_err(batch_denial_error)?;
        let mut outcomes = Vec::with_capacity(admitted.items.len());
        for (item, decided) in request.items.iter().zip(admitted.items) {
            let decision = match decided.reservation {
                Some(decided_reservation) => {
                    // `self.state` holds exactly the items committed so far, which is the state the
                    // pure batch evaluated this item against.
                    let transition = apply_reservation(&self.state, item).map_err(denial_error)?;
                    if transition.reservation.as_ref() != Some(&decided_reservation) {
                        return Err(crate::error::Failure::invalid_harness(
                            "batch item decision diverged from authoritative reservation state",
                        ));
                    }
                    self.allocate_and_commit(transition)?
                }
                None => {
                    let issue = decided.issue.ok_or_else(|| {
                        crate::error::Failure::invalid_harness("rejected batch item carries no issue")
                    })?;
                    Decision::rejected(issue)
                }
            };
            outcomes.push(ItemOutcome {
                job_id: decided.job_id,
                decision,
            });
        }
        Ok(outcomes)
    }

    /// Executes the decided allocation for one committed transition and commits
    /// it. A definite adapter failure commits nothing. An uncertain outcome may
    /// already hold external capacity, so the reservation stays committed and
    /// only an admitted enforcement fact (`expire`) can release it.
    fn allocate_and_commit(&mut self, transition: ReservationTransition) -> crate::error::Result<Decision> {
        let reservation = transition
            .reservation
            .ok_or_else(|| crate::error::Failure::invalid_harness("committed reservation carries no record"))?;
        if let Err(error) = self.effects.allocate(&reservation) {
            if error.class() == crate::error::FailureClass::UncertainExternalOutcome {
                self.state = transition.next;
            }
            return Err(allocation_failure(&reservation.job_id, &error));
        }
        self.state = transition.next;
        Ok(Decision {
            outcome: ReservationOutcome::Committed,
            reservation: Some(reservation),
            issue: None,
        })
    }

    /// Marks an expired lease stale and releases capacity only with an
    /// enforcement or termination fact.
    // r[impl molten.reservation_command.expiry_not_release]
    pub fn expire(&mut self, expiry: &ReservationExpiry) -> crate::error::Result<ExpiryOutcome> {
        let transition = expire_reservation(&self.state, expiry).map_err(expiry_denial_error)?;
        self.state = transition.next;
        Ok(transition.outcome)
    }

    /// The operator-visible capacity status.
    pub fn status(&self) -> ReservationStatus {
        reservation_status(&self.state)
    }
}

/// Keeps the port's failure class, including uncertainty, instead of flattening it into harness
/// text.
fn allocation_failure(job_id: &str, error: &crate::fabric::FabricPortError) -> crate::error::Failure {
    crate::error::Failure::classified(
        error.class(),
        format!("reservation allocation failed for {job_id}: {}", error.message()),
    )
}

fn denial_error(issue: ReservationIssue) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("reservation command denied: {issue:?}"))
}

fn batch_denial_error(issue: BatchIssue) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("reservation batch denied: {issue:?}"))
}

fn expiry_denial_error(issue: ExpiryIssue) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(format!("reservation expiry denied: {issue:?}"))
}
