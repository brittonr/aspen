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
