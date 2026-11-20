//! Native in-process consumer-adapter qualification over the real Raft and
//! application paths. This is not guest, KVM, or production evidence.

use super::*;
use crate::fabric_consistency::chaoscontrol::ChaosControlChainProjector;
use crate::fabric_consistency::chaoscontrol::ChaosControlConformanceProfile;
use crate::fabric_consistency::chaoscontrol::ChaosControlObservationMode;
use crate::fabric_consistency::chaoscontrol::CommittedApplyObservation;
use crate::fabric_consistency::chaoscontrol::conformance::safety;
use crate::fabric_consistency::chaoscontrol::conformance::wire;

const PROFILE_ID: &str = "molten-live-cluster-conformance-v1";
const COMMAND_SCHEMA: &str = "live-cluster-command-schema";

fn workload(initial_state_ref: &str) -> chaoscontrol_smr::SmrWorkloadPlan {
    chaoscontrol_smr::admit_profile(&chaoscontrol_smr::SmrWorkloadProfile {
        schema: chaoscontrol_smr::PROFILE_SCHEMA.into(),
        profile_id: PROFILE_ID.into(),
        initial_state_ref: initial_state_ref.into(),
        observation_mode: chaoscontrol_smr::ObservationMode::Lossless,
        bounds: chaoscontrol_smr::WorkloadBounds {
            maximum_commands: 8,
            maximum_command_bytes: 2_048,
            maximum_clients: 3,
            maximum_concurrency: 3,
            maximum_virtual_progress: 100,
            maximum_trace_events: 64,
            maximum_fault_actions: 8,
            maximum_replay_events: 64,
            maximum_evidence_bytes: 1_048_576,
            maximum_reduction_attempts: 8,
        },
        liveness: chaoscontrol_smr::LivenessProfile {
            profile_id: "live-cluster-progress".into(),
            required_progress: 1,
            virtual_progress_horizon: 16,
        },
    })
    .expect("admitted live-cluster workload")
}

fn protocol(group: &crate::fabric_consistency::Group) -> chaoscontrol_smr::observation::ProtocolProfile {
    let participants = [
        super::super::tests::NODE_A,
        super::super::tests::NODE_B,
        super::super::tests::NODE_C,
    ]
    .into_iter()
    .enumerate()
    .map(|(guest_id, node)| chaoscontrol_smr::observation::Participant {
        participant_ref: super::super::tests::test_ref(&format!("conformance-participant-{node}")),
        producer_ref: super::super::tests::test_ref(&format!("conformance-observer-{node}")),
        guest_id: u32::try_from(guest_id).expect("guest id"),
        generation: group.service_generation,
    })
    .collect();
    let profile = chaoscontrol_smr::observation::ProtocolProfile {
        schema: chaoscontrol_smr::observation::PROFILE_SCHEMA.into(),
        protocol_ref: super::super::tests::test_ref("conformance-protocol"),
        projection_schema_ref: super::super::tests::test_ref("conformance-projection-schema"),
        boundary_schema_ref: super::super::tests::test_ref("conformance-boundary-schema"),
        oracle_adapter_ref: super::super::tests::test_ref("conformance-adapter"),
        independent_oracle_ref: super::super::tests::test_ref("conformance-oracle"),
        novelty_selection_ref: super::super::tests::test_ref("conformance-selection"),
        participants,
        bounds: chaoscontrol_smr::observation::Bounds {
            records: 16,
            record_bytes: 65_536,
            total_bytes: 131_072,
            boundaries: 8,
            cohort_backlog: 8,
            oracle_work: 16,
        },
        marker_policy: chaoscontrol_smr::observation::MarkerPolicy::Disabled,
        non_claims: chaoscontrol_smr::observation::NON_CLAIMS.iter().map(|value| (*value).into()).collect(),
    };
    profile.admit().expect("admitted conformance protocol");
    profile
}

fn command_bytes(request_ref: &str, command_ref: &str) -> Vec<u8> {
    crate::preserves_rail::canonical_bytes(&crate::preserves_rail::record("live-cluster-conformance-command-v1", vec![
        crate::preserves_rail::string(request_ref),
        crate::preserves_rail::string(command_ref),
        crate::preserves_rail::string(super::super::tests::test_ref(COMMAND_SCHEMA)),
    ]))
    .expect("canonical command bytes")
}

#[tokio::test]
async fn native_cluster_adapter_projects_real_committed_chain_observations() {
    let group = super::super::tests::active_group();
    let listener_a = crate::fabric_transport::cross_process::tests::listener_with_secret(17).await;
    let listener_b = crate::fabric_transport::cross_process::tests::listener_with_secret(19).await;
    let listener_c = crate::fabric_transport::cross_process::tests::listener_with_secret(23).await;
    let endpoints = std::collections::BTreeMap::from([
        (super::super::tests::NODE_A.to_string(), listener_a.handoff().clone()),
        (super::super::tests::NODE_B.to_string(), listener_b.handoff().clone()),
        (super::super::tests::NODE_C.to_string(), listener_c.handoff().clone()),
    ]);
    let mut node_a = setup::build_node(&group, super::super::tests::NODE_A, listener_a, &endpoints)
        .await
        .expect("node A");
    let mut node_b = setup::build_node(&group, super::super::tests::NODE_B, listener_b, &endpoints)
        .await
        .expect("node B");
    let mut node_c = setup::build_node(&group, super::super::tests::NODE_C, listener_c, &endpoints)
        .await
        .expect("node C");
    workflow::elect_node_a(&mut node_a, &mut node_b, &mut node_c).await.expect("election");

    let initial_state_ref = super::super::tests::test_ref("live-cluster-initial-application-state");
    let plan = workload(&initial_state_ref);
    let protocol = protocol(&group);
    let conformance = ChaosControlConformanceProfile {
        chaoscontrol_profile_ref: plan.profile_ref.clone(),
        observation_mode: ChaosControlObservationMode::Lossless,
        max_command_bytes: 2_048,
        max_projected_observations: 8,
    };
    let mut projectors = std::collections::BTreeMap::new();
    for node in [
        super::super::tests::NODE_A,
        super::super::tests::NODE_B,
        super::super::tests::NODE_C,
    ] {
        let participant_ref = super::super::tests::test_ref(&format!("conformance-participant-{node}"));
        projectors.insert(
            participant_ref,
            ChaosControlChainProjector::bind(conformance.clone(), &initial_state_ref).expect("projector"),
        );
    }

    let command_ref = super::super::tests::test_ref("conformance-command");
    let schema_ref = super::super::tests::test_ref(COMMAND_SCHEMA);
    let mut bounds = Vec::new();
    for index in 1..=2_u64 {
        let request_ref = super::super::tests::test_ref(&format!("conformance-request-{index}"));
        let observed = workflow::replicate_request_observed(
            &mut node_a,
            &mut node_b,
            &mut node_c,
            &request_ref,
            &command_ref,
            &schema_ref,
        )
        .await
        .unwrap_or_else(|error| panic!("observed replication at index {index}: {error}"));
        for apply in observed {
            let participant_ref = super::super::tests::test_ref(&format!("conformance-participant-{}", apply.node_id));
            let projector = projectors.get_mut(&participant_ref).expect("projector");
            let observation = projector
                .project(&CommittedApplyObservation {
                    group_ref: group.binding_ref.clone(),
                    replica_ref: participant_ref.clone(),
                    command_index: apply.command_index,
                    operation_ref: apply.request_ref.clone(),
                    command_ref: command_ref.clone(),
                    command_bytes: command_bytes(&apply.request_ref, &command_ref),
                    application_state_ref: apply.state_ref.clone(),
                    lifecycle_generation: group.service_generation,
                    application_receipt_ref: apply.receipt_ref.clone(),
                })
                .expect("project committed apply");
            let context = wire::ProjectionContext {
                group_ref: group.binding_ref.clone(),
                source_sequence: apply.command_index,
                boundary_ref: super::super::tests::test_ref(&format!("conformance-boundary-{index}")),
                scheduler_ref: super::super::tests::test_ref("conformance-scheduler"),
                transition_class_ref: super::super::tests::test_ref("conformance-transition"),
            };
            let bound = wire::project(&protocol, &plan, &context, &observation).expect("wire projection");
            wire::validate(&protocol, &plan, &group.binding_ref, &bound).expect("wire validation");
            bounds.push(bound);
        }
    }

    assert_eq!(bounds.len(), 4);
    for index in 1..=2_u64 {
        let at_index = bounds.iter().filter(|bound| bound.apply.chain.command_index == index).collect::<Vec<_>>();
        assert_eq!(at_index.len(), 2);
        let digest = &at_index[0].apply.chain.digest;
        let state_ref = &at_index[0].apply.chain.application_state_ref;
        assert!(at_index.iter().all(|bound| &bound.apply.chain.digest == digest));
        assert!(at_index.iter().all(|bound| &bound.apply.chain.application_state_ref == state_ref));
    }
    let receipts = bounds
        .iter()
        .map(|bound| &bound.apply.application_receipt_ref)
        .collect::<std::collections::BTreeSet<_>>();
    // The Raft application port returns one deterministic batch receipt per
    // command, shared by the replicas that applied it.
    assert_eq!(receipts.len(), 2);
    let applies = bounds.iter().map(|bound| safety::AppliedChain::from(&bound.apply)).collect::<Vec<_>>();
    let report = safety::evaluate(&plan, &applies).expect("safety evaluation");
    assert!(!report.is_safety_failure());
    assert_eq!(report.verdict(), chaoscontrol_smr::SafetyVerdict::Pass);

    setup::close_node(node_a).await;
    setup::close_node(node_b).await;
    setup::close_node(node_c).await;
}
