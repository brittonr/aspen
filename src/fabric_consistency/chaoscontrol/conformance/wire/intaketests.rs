//! Producer guest-history interoperability of supplied offline wire records.

struct OfflineIntake {
    sources: Vec<chaoscontrol_smr::observation::guest::GuestSource>,
    execution_ref: String,
    observations: Vec<chaoscontrol_smr::observation::ProtocolObservation>,
    drains: Vec<chaoscontrol_smr::observation::Drain>,
}

impl chaoscontrol_smr::observation::guest::MultiNodeGuestIntake for OfflineIntake {
    fn sources(&self) -> &[chaoscontrol_smr::observation::guest::GuestSource] {
        &self.sources
    }

    fn execution_ref(&self) -> &str {
        &self.execution_ref
    }

    fn observations(&self) -> &[chaoscontrol_smr::observation::ProtocolObservation] {
        &self.observations
    }

    fn drains(&self) -> &[chaoscontrol_smr::observation::Drain] {
        &self.drains
    }
}

fn intake(
    protocol: &chaoscontrol_smr::observation::ProtocolProfile,
    observations: Vec<chaoscontrol_smr::observation::ProtocolObservation>,
) -> OfflineIntake {
    OfflineIntake {
        sources: protocol
            .participants
            .iter()
            .map(|participant| chaoscontrol_smr::observation::guest::GuestSource {
                guest_id: participant.guest_id,
                executable_ref: super::fixtures::id("offline-executable-control-not-an-executed-guest"),
                source_revision_ref: super::fixtures::id("offline-source-control"),
                adapter_ref: protocol.oracle_adapter_ref.clone(),
                launch_contract_ref: super::fixtures::id("offline-launch-control"),
            })
            .collect(),
        execution_ref: super::fixtures::id("supplied-offline-intake-identity-not-an-execution-attestation"),
        observations,
        drains: protocol
            .participants
            .iter()
            .map(|participant| chaoscontrol_smr::observation::Drain {
                participant_ref: participant.participant_ref.clone(),
                producer_ref: participant.producer_ref.clone(),
                generation: participant.generation,
                last_sequence: 2,
                dropped: 0,
                final_drain: true,
            })
            .collect(),
    }
}

#[test]
fn committed_apply_encoder_interoperates_with_real_producer_guest_history() {
    let (protocol, plan) = super::fixtures::profiles();
    assert_ne!(protocol.admit().expect("protocol identity"), plan.profile_ref);
    let capacity = protocol.participants.len().checked_mul(2).expect("fixture record count");
    let mut records = Vec::with_capacity(capacity);
    let mut projectors = [super::fixtures::projector(&plan), super::fixtures::projector(&plan)];
    for index in 1..=2 {
        for (participant, projector) in projectors.iter_mut().enumerate() {
            let command = format!("command-{index}");
            let observation = projector
                .project(&super::fixtures::apply(&protocol, participant, index, command.as_bytes()))
                .expect("existing committed-apply projection");
            let encoded = super::project(&protocol, &plan, &super::fixtures::context(index), &observation)
                .expect("wire encoding");
            super::validate(&protocol, &plan, &super::fixtures::id("group"), &encoded)
                .expect("consumer source and transition checks");
            super::binding_ref(&encoded.apply).expect("canonical retained provenance");
            records.push(encoded.envelope);
        }
    }
    let mut intake = intake(&protocol, records);
    let report = chaoscontrol_smr::observation::guest::validate_guest_history(
        &protocol,
        &super::fixtures::context(2).boundary_ref,
        &plan,
        &intake,
    )
    .expect("real producer source/cohort/history intake");
    assert_eq!(report.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert_eq!(report.history.observations_admitted, 4);
    // A valid envelope with an omitted source event is incomplete, not a pass.
    intake.observations.remove(0);
    assert!(
        chaoscontrol_smr::observation::guest::validate_guest_history(
            &protocol,
            &super::fixtures::context(2).boundary_ref,
            &plan,
            &intake,
        )
        .is_err()
    );
}
