use std::collections;

use super::super::domain;
use super::super::planning;
use crate::world_commit;
use crate::world_head;

pub(super) fn reference(label: &str) -> String {
    format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
}

pub(super) fn input() -> domain::CurrentnessInput {
    let subject = domain::WitnessSubject {
        branch_id: world_head::WorldBranchId::new("main").unwrap(),
        claim_ref: world_head::WorldHeadClaimRef::new(reference("claim")).unwrap(),
        generation: 2,
    };
    let contract = domain::ProviderContract {
        provider_id: "fixture-provider".into(),
        contract_id: "fixture-independent-contract-v1".into(),
        source_revision: "fixture-source-v1".into(),
        license: "MIT OR Apache-2.0".into(),
    };
    let witnesses = collections::BTreeSet::from(["independent-a".into()]);
    domain::CurrentnessInput {
        selected_profile: domain::AssuranceProfile::IndependentWitnessV1,
        policy: domain::WitnessPolicy {
            required_profile: domain::AssuranceProfile::IndependentWitnessV1,
            provider: contract.clone(),
            allowed_witnesses: witnesses.clone(),
            minimum_quorum: 1,
            maximum_age: 10,
        },
        operation_id: reference("operation"),
        subject: subject.clone(),
        local_generation: 1,
        retained: domain::RetainedWitnessState::Empty,
        observation: Some(domain::WitnessObservation::Available(Box::new(available(subject, contract, witnesses)))),
        now: 105,
    }
}

fn available(
    subject: domain::WitnessSubject,
    contract: domain::ProviderContract,
    witnesses: collections::BTreeSet<String>,
) -> domain::AvailableWitnessObservation {
    let checkpoint_id = reference("checkpoint");
    domain::AvailableWitnessObservation {
        checkpoint: domain::SignedCheckpoint {
            provider_id: contract.provider_id.clone(),
            checkpoint_id: checkpoint_id.clone(),
            revision: 3,
            signature_verified: true,
            observed_at: 100,
        },
        contract,
        append: domain::AppendObservation {
            operation_id: reference("operation"),
            subject: subject.clone(),
            accepted: true,
        },
        inclusion: domain::InclusionObservation {
            subject: subject.clone(),
            checkpoint_id: checkpoint_id.clone(),
            verified: true,
        },
        consistency: domain::ConsistencyObservation {
            predecessor: None,
            checkpoint_id: checkpoint_id.clone(),
            verified: true,
        },
        quorum: domain::QuorumObservation {
            checkpoint_id: checkpoint_id.clone(),
            witnesses,
            verified: true,
        },
        current_head: domain::CurrentHeadObservation {
            subject,
            checkpoint_id,
            verified: true,
            phase: domain::CurrentHeadPhase::Committed,
        },
    }
}

pub(super) fn observation(input: &mut domain::CurrentnessInput) -> &mut domain::AvailableWitnessObservation {
    match input.observation.as_mut().unwrap() {
        domain::WitnessObservation::Available(value) => value,
        _ => panic!("available fixture"),
    }
}

pub(super) fn finalization() -> planning::FinalizationInput {
    let input = input();
    let policy = world_head::WorldHeadPolicyRef::new(reference("policy")).unwrap();
    let before = world_head::WorldHeadState {
        branch_id: input.subject.branch_id.clone(),
        branch_class: world_head::WorldBranchClass::Local,
        head: world_commit::WorldCommitRef::new(reference("before")).unwrap(),
        generation: 1,
        policy_ref: policy.clone(),
    };
    let staged = planning::StagedClaim {
        operation_id: input.operation_id.clone(),
        selected_profile: input.selected_profile,
        retained: input.retained.clone(),
        transition: world_head::WorldHeadTransitionPlan {
            claim_ref: input.subject.claim_ref.clone(),
            before: Some(before.clone()),
            after: world_head::WorldHeadState {
                head: world_commit::WorldCommitRef::new(reference("after")).unwrap(),
                generation: 2,
                ..before.clone()
            },
            choregraph_before_identity: Some(reference("before")),
            choregraph_after_identity: reference("after"),
            currentness: world_head::WorldHeadCurrentnessClass::RelativeToObservedStore,
        },
    };
    planning::FinalizationInput {
        durable_stage: Some(staged.clone()),
        staged,
        current_head: Some(before),
        authority: world_head::WorldHeadAuthorityObservation {
            authority_ref: world_head::WorldHeadAuthorityRef::new(reference("authority")).unwrap(),
            policy_ref: policy,
            admitted: true,
            observed_generation: 1,
        },
        currentness: input,
    }
}
