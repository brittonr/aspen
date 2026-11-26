use std::path::Path;

use molten::error::Result;
use molten::world_promotion::RetryPlanInput;
use molten::world_promotion::plan_retry_for_reservation;
use molten::world_promotion::read_promotion_grant;
use molten_core::world_promotion::WorldReleaseAttemptRef;
use molten_core::world_promotion::WorldReleaseReservationRef;

pub(super) struct RetryPlanArguments<'a> {
    pub state_root: &'a Path,
    pub request_path: &'a Path,
    pub authority: &'a Path,
    pub trusted_signer: &'a str,
    pub reservation_ref: &'a str,
    pub attempt_ref: &'a str,
    pub next_attempt_ref: &'a str,
    pub is_acknowledged: bool,
}

pub(super) fn retry_plan(arguments: RetryPlanArguments<'_>) -> Result<()> {
    let operator = read_promotion_grant(arguments.authority, arguments.trusted_signer)?;
    let mut store = super::open_store(arguments.state_root)?;
    let request = super::read_request(arguments.request_path)?;
    let reservation_ref =
        WorldReleaseReservationRef::new(arguments.reservation_ref.to_string()).map_err(super::reference_error)?;
    let attempt_ref = WorldReleaseAttemptRef::new(arguments.attempt_ref.to_string()).map_err(super::reference_error)?;
    let next_attempt_ref =
        WorldReleaseAttemptRef::new(arguments.next_attempt_ref.to_string()).map_err(super::reference_error)?;
    let plan = plan_retry_for_reservation(RetryPlanInput {
        store: &mut store,
        request: &request,
        operator: &operator,
        reservation_ref: &reservation_ref,
        attempt_ref: &attempt_ref,
        next_attempt_ref: next_attempt_ref.clone(),
        is_duplicate_risk_acknowledged: arguments.is_acknowledged,
        observed_unix_seconds: super::observed_unix_seconds()?,
    })?;
    println!("reservation_ref={}", plan.reservation_ref);
    println!("previous_attempt_ref={}", plan.previous_attempt_ref);
    println!("next_attempt_ref={}", plan.next_attempt_ref);
    println!("duplicate_risk_acknowledged={}", plan.duplicate_risk_acknowledged);
    println!("same_logical_release={}", plan.same_logical_release);
    println!("operator_authority_current=true");
    println!("external_completion_proven=false");
    println!("decision=planned");
    Ok(())
}
