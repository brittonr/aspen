use super::registry;

fn reference(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("registry-unit-input", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("canonical identity")
}
fn registry() -> registry::Registry {
    registry::Registry::new(registry::RegistryAdmission {
        object_key: "services/example".into(),
        command_schema_ref: reference("schema"),
        endpoints: std::collections::BTreeMap::from([(1, reference("endpoint-one")), (2, reference("endpoint-two"))]),
    })
    .expect("admitted corpus")
}
fn command(registry: &registry::Registry, value: i64) -> crate::fabric_consistency::raft::Command {
    crate::fabric_consistency::raft::Command {
        request_ref: reference(&format!("write-{value}")),
        command_ref: registry.command(value).expect("admitted command"),
        command_schema_ref: registry.schema_ref().into(),
    }
}

#[test]
fn committed_registry_replaces_value_and_restores_exact_snapshot() {
    let mut registry = registry();
    assert_eq!(registry.value(), serde_json::json!({"type":"null"}));
    let first = command(&registry, 1);
    let snapshot_ref = crate::fabric_consistency::raft::CommittedBatchHandler::apply_batch(&mut registry, &[first])
        .expect("first committed write");
    assert_eq!(registry.value(), serde_json::json!({"type":"integer", "value":1}));
    let second = command(&registry, 2);
    crate::fabric_consistency::raft::CommittedBatchHandler::apply_batch(&mut registry, &[second])
        .expect("replacement committed write");
    assert_eq!(registry.value(), serde_json::json!({"type":"integer", "value":2}));
    crate::fabric_consistency::raft::CommittedBatchHandler::restore_snapshot(
        &mut registry,
        &crate::fabric_consistency::raft::SnapshotRestore {
            snapshot_ref: reference("snapshot"),
            application_state_ref: snapshot_ref,
        },
    )
    .expect("admitted state restored");
    assert_eq!(registry.value(), serde_json::json!({"type":"integer", "value":1}));
}

#[test]
fn invalid_commit_batch_and_snapshot_do_not_partially_mutate_registry() {
    let mut registry = registry();
    let valid = command(&registry, 1);
    let invalid = crate::fabric_consistency::raft::Command {
        command_ref: reference("outside-corpus"),
        ..valid.clone()
    };
    assert!(
        crate::fabric_consistency::raft::CommittedBatchHandler::apply_batch(&mut registry, &[valid, invalid]).is_err()
    );
    assert_eq!(registry.value(), serde_json::json!({"type":"null"}));
    assert!(
        crate::fabric_consistency::raft::CommittedBatchHandler::restore_snapshot(
            &mut registry,
            &crate::fabric_consistency::raft::SnapshotRestore {
                snapshot_ref: reference("snapshot"),
                application_state_ref: reference("outside-state-corpus"),
            }
        )
        .is_err()
    );
    assert_eq!(registry.value(), serde_json::json!({"type":"null"}));
    assert!(registry.command(3).is_err());
}

#[test]
fn distinct_integers_cannot_alias_one_endpoint_state() {
    let endpoint = reference("same-endpoint");
    assert!(
        registry::Registry::new(registry::RegistryAdmission {
            object_key: "services/example".into(),
            command_schema_ref: reference("schema"),
            endpoints: std::collections::BTreeMap::from([(1, endpoint.clone()), (2, endpoint)]),
        })
        .is_err()
    );
}

#[tokio::test]
async fn public_socket_rejects_truncated_or_malformed_frames() {
    use tokio::io::AsyncWriteExt as _;
    for bytes in [
        b"{\"schema\":\"onixos.live-client.v1\"}".as_slice(),
        b"not-json\n".as_slice(),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("socket");
        let address = listener.local_addr().expect("address");
        let mut client = tokio::net::TcpStream::connect(address).await.expect("connect");
        let (mut server, _) = listener.accept().await.expect("accept");
        client.write_all(bytes).await.expect("write");
        client.shutdown().await.expect("shutdown");
        assert!(super::runtime::receive(&mut server).await.is_err());
    }
}

const NODE_A_ENDPOINT: &str = "127.0.0.1:7101";
const NODE_B_ENDPOINT: &str = "127.0.0.1:7102";
const NODE_C_ENDPOINT: &str = "127.0.0.1:7103";
const OBJECT_KEY: &str = "key";

fn register_profile() -> super::super::profile::Register {
    serde_json::from_value(serde_json::json!({"schema":"molten.live-register-profile.v1","group":"group",
        "service":"registry","keys":[OBJECT_KEY],"initial_value":null,"values":[1,2],"generator_ref":reference("generator"),
        "adapter_ref":reference("adapter"),"model_ref":reference("model"),"outcome_ref":reference("outcome"),
        "seed":0,"read_weight":1,"write_weight":9,"clients":["public-client"],
        "endpoints":[NODE_A_ENDPOINT, NODE_B_ENDPOINT, NODE_C_ENDPOINT],
        "concurrency":1,"operations":8,"control_operations":2,"consistency_model":"register",
        "recovery":{"timeout_ms":1000,"stable_membership_observations":1,"require_all_endpoint_reads":true},
        "evidence":{"max_attempts_per_operation":1,"max_events":16,"max_uncertain":4,"max_bytes":1048576,
        "checker_states":1000,"checker_branches":1000,"checker_memo_bytes":4096,"checker_reductions":10}}))
    .expect("register profile")
}

fn host_admission(local: &str) -> super::HostAdmission {
    let admission = super::HostAdmission {
        run_id: "fenced-run".into(),
        endpoint: local.parse().expect("local endpoint"),
        max_connections: 4,
        peers: [
            ("node-a", NODE_A_ENDPOINT),
            ("node-b", NODE_B_ENDPOINT),
            ("node-c", NODE_C_ENDPOINT),
        ]
        .map(|(node, endpoint)| (node.to_string(), endpoint.parse().expect("peer endpoint")))
        .into(),
        profile: register_profile(),
    };
    admission.validate().expect("admitted three-voter host");
    admission
}

fn public_request(
    phase: super::super::protocol::Phase,
    endpoint: &str,
    operation: Option<serde_json::Value>,
) -> super::super::protocol::Request {
    super::super::protocol::Request {
        schema: super::super::protocol::SCHEMA.into(),
        phase,
        run_id: "fenced-run".into(),
        endpoint: endpoint.into(),
        operation,
        deadline_ms: 1_000,
        profile: Some(register_profile()),
        authorization: None,
        probe: None,
    }
}

// r[verify molten.consensus.reliability_replay_and_failstop]
#[test]
fn teardown_is_scoped_to_the_named_host_while_forwarded_operations_stay_admitted() {
    let teardown_a = public_request(super::super::protocol::Phase::Teardown, NODE_A_ENDPOINT, None);
    assert_eq!(super::runtime::admit_scope(&host_admission(NODE_A_ENDPOINT), &teardown_a, OBJECT_KEY), Ok(()));
    for replayed_on in [NODE_B_ENDPOINT, NODE_C_ENDPOINT] {
        assert_eq!(
            super::runtime::admit_scope(&host_admission(replayed_on), &teardown_a, OBJECT_KEY),
            Err("teardown-outside-local-endpoint".to_string()),
            "node A's signed teardown replayed to {replayed_on}"
        );
    }
    // An operation addressed to node A is still forwarded through node B to the leader.
    let read_a = public_request(
        super::super::protocol::Phase::Invoke,
        NODE_A_ENDPOINT,
        Some(serde_json::json!({"id":"read-1","process":"public-client","object_key":OBJECT_KEY,"function":"read"})),
    );
    assert_eq!(super::runtime::admit_scope(&host_admission(NODE_B_ENDPOINT), &read_a, OBJECT_KEY), Ok(()));
}

fn failed_outcome(diagnostic: &str) -> crate::fabric_consistency::raft::ReplicaExecutionOutcome {
    let group = crate::fabric_consistency::raft::tests::active_group();
    let state = crate::fabric_consistency::raft::tests::started_state(&group, "node-a");
    crate::fabric_consistency::raft::ReplicaExecutionOutcome::Failed(Box::new(
        crate::fabric_consistency::raft::FailedReplicaTransition {
            retained: state.clone(),
            planned: state,
            completed: Vec::new(),
            transport_failures: Vec::new(),
            failed_kind: crate::fabric_consistency::raft::ReplicaEffectKind::PersistEntries,
            diagnostic: diagnostic.into(),
        },
    ))
}

// r[verify molten.consensus.reliability_replay_and_failstop]
#[test]
fn quarantine_answers_uncertainty_and_latches_fail_stop_while_a_denial_does_not() {
    let request = public_request(
        super::super::protocol::Phase::Invoke,
        NODE_A_ENDPOINT,
        Some(serde_json::json!({"id":"write-1","process":"public-client","object_key":OBJECT_KEY,"function":"write"})),
    );
    let state = crate::fabric_consistency::raft::tests::started_state(
        &crate::fabric_consistency::raft::tests::active_group(),
        "node-a",
    );
    let mut fail_stop = super::runtime::FailStop::default();
    let denial = crate::fabric_consistency::raft::ReplicaExecutionOutcome::Denied {
        retained: state,
        diagnostic: "proposal-denied".into(),
    };
    let denied =
        super::runtime::operation_response(&mut fail_stop, &request, denial).expect("denial ends the operation");
    assert_eq!(denied.outcome, super::super::protocol::Outcome::Fail);
    assert!(denied.pre_effect_denial);
    assert!(fail_stop.latched().is_none(), "a pre-effect denial keeps the host serving");

    let quarantined =
        super::runtime::operation_response(&mut fail_stop, &request, failed_outcome("durable append failed"))
            .expect("quarantine ends the operation");
    assert_eq!(quarantined.outcome, super::super::protocol::Outcome::Info, "the failed effect may have happened");
    assert!(!quarantined.pre_effect_denial);
    let latched = fail_stop.take().expect("quarantine latches fail-stop for the serve loop");
    assert!(latched.to_string().contains("durable append failed"));

    // Settling a quarantine on the serve or wait path latches the same fail-stop.
    assert!(fail_stop.settle(failed_outcome("snapshot persistence failed")).is_err());
    assert!(fail_stop.latched().is_some());
}
