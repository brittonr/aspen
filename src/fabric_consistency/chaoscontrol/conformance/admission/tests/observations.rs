fn observations(expected: &super::ExpectedCohort, plan: &chaoscontrol_smr::SmrWorkloadPlan)
    -> Vec<super::super::wire::BoundObservation>
{
    let capacity = expected.protocol.participants.len().checked_mul(expected.operations.len()).unwrap();
    let mut observations = Vec::with_capacity(capacity);
    for participant in &expected.protocol.participants {
        let mut prior = plan.genesis_digest.clone();
        for operation in &expected.operations {
            let next =
                chaoscontrol_smr::chain_transition(&plan.profile_ref, operation.command_index, &prior, &operation.command_bytes)
                    .unwrap();
            let observation = super::super::super::ChaosControlChainObservation {
                chaoscontrol_profile_ref: plan.profile_ref.clone(),
                group_ref: expected.group_ref.clone(),
                replica_ref: participant.participant_ref.clone(),
                command_index: operation.command_index,
                operation_ref: operation.operation_ref.clone(),
                command_ref: operation.command_ref.clone(),
                command_bytes: operation.command_bytes.clone(),
                prior_digest: prior,
                next_digest: next.clone(),
                application_state_ref: operation.application_state_ref.clone(),
                lifecycle_generation: participant.generation,
                application_receipt_ref: id(&format!("apply-{}-{}", participant.guest_id, operation.command_index)),
            };
            observations.push(
                super::super::wire::project(
                    &expected.protocol,
                    plan,
                    &super::super::wire::ProjectionContext {
                        group_ref: expected.group_ref.clone(),
                        source_sequence: operation.command_index,
                        boundary_ref: operation.boundary_ref.clone(),
                        scheduler_ref: expected.scheduler_ref.clone(),
                        transition_class_ref: operation.transition_class_ref.clone(),
                    },
                    &observation,
                )
                .unwrap(),
            );
            prior = next;
        }
    }
    observations
}
