pub struct ReconciledOutcome {
    pub decision: molten_core::world_witness::ReconciliationDecision,
    pub outcome: Option<crate::world_witness::Outcome>,
}

/// Resume only the exact durably staged operation. This never dispatches a new
/// Reserve after a lost response; recovery must first observe the exact pending
/// reservation. External completion cannot grant branch authority.
pub fn reconcile_and_resume<T: crate::world_witness::provider::Transport, P: crate::world_witness::provider::Ports>(
    store: &mut crate::world_witness::store::Database<'_>,
    provider: &mut crate::world_witness::provider::Index<T, P>,
    request: super::Request<'_>,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<ReconciledOutcome> {
    request.stage.validate()?;
    ensure_policy(&request, &mut *authority)?;
    let decision = super::recover(
        store,
        provider,
        super::Request {
            stage: request.stage,
            claim: request.claim,
            template: request.template,
        },
        authority,
    )?;
    let reservation = match &decision {
        molten_core::world_witness::ReconciliationDecision::Finalize(_)
        | molten_core::world_witness::ReconciliationDecision::Retryable
        | molten_core::world_witness::ReconciliationDecision::AlreadyComplete
        | molten_core::world_witness::ReconciliationDecision::Superseded => {
            super::reservation(store, provider, &request)?
        }
        _ => None,
    };
    let Some(reservation) = reservation else {
        return Ok(ReconciledOutcome {
            decision,
            outcome: None,
        });
    };
    let current = store.current(&request.stage.staged.transition.after.branch_id)?;
    let outcome = match &decision {
        molten_core::world_witness::ReconciliationDecision::Finalize(_) => {
            Some(crate::world_witness::lifecycle::effects::finalize(
                store,
                provider,
                authority,
                crate::world_witness::lifecycle::effects::Prepared {
                    stage: request.stage,
                    claim: request.claim,
                    currentness: request.template,
                    reservation: &reservation,
                },
            )?)
        }
        molten_core::world_witness::ReconciliationDecision::Retryable
        | molten_core::world_witness::ReconciliationDecision::AlreadyComplete
            if current.as_ref() == Some(&request.stage.staged.transition.after) =>
        {
            Some(crate::world_witness::lifecycle::effects::complete(
                store,
                provider,
                &reservation,
                request.template,
                request.stage,
            )?)
        }
        molten_core::world_witness::ReconciliationDecision::Superseded => {
            Some(if provider.orphan(&reservation).is_err() {
                crate::world_witness::lifecycle::shell::uncertain(
                    request.stage,
                    crate::world_witness::WitnessUncertainty::Orphan,
                )
            } else {
                crate::world_witness::Outcome::Superseded
            })
        }
        _ => None,
    };
    Ok(ReconciledOutcome { decision, outcome })
}

fn ensure_policy(
    request: &super::Request<'_>,
    authority: &mut impl crate::world_witness::FreshAdmission,
) -> crate::error::Result<()> {
    if authority.witness_policy_ref()?
        != crate::world_witness::lifecycle::durable::policy_identity(&request.stage.witness_policy)?
    {
        return Err(crate::world_witness::store::failure("installed witness policy changed before reconciliation"));
    }
    Ok(())
}
