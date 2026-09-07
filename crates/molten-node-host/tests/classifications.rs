#[test]
fn store_labels_keep_the_fixed_inventory() {
    type Kind = molten_node_host::local_store::LocalStoreKind;
    let expected = [
        (Kind::Artifact, "artifact"),
        (Kind::Chunk, "chunk"),
        (Kind::Retention, "retention"),
        (Kind::Dataspace, "dataspace"),
        (Kind::Exchange, "exchange"),
        (Kind::Ledger, "ledger"),
        (Kind::Delivery, "delivery"),
        (Kind::Durable, "durable"),
    ];
    for (kind, label) in expected {
        assert_eq!(kind.as_str(), label);
    }
}

#[test]
fn namespace_registry_keeps_order_and_intentional_aliases() {
    type Kind = molten_node_host::node_state::NodeStateNamespaceKind;
    let expected = [
        (Kind::Identity, "identity"),
        (Kind::Secrets, "identity"),
        (Kind::Ledger, "ledger"),
        (Kind::ControlInbox, "control/inbox"),
        (Kind::ControlOutbox, "control/outbox"),
        (Kind::ControlIngress, "control/iroh-ingress"),
        (Kind::ControlIdempotency, "control/idempotency"),
        (Kind::ControlService, "control/service"),
        (Kind::Services, "services"),
        (Kind::Receipts, "receipts"),
        (Kind::Registry, "registry"),
        (Kind::Chunks, "chunks"),
        (Kind::Storage, "storage"),
        (Kind::Ingress, "control/iroh-ingress"),
    ];
    assert_eq!(Kind::ALL, expected.map(|(kind, _)| kind));
    assert_eq!(Kind::ALL.into_iter().collect::<std::collections::BTreeSet<_>>().len(), 14);
    for (kind, label) in expected {
        assert_eq!(kind.as_str(), label);
    }
}

#[test]
fn directory_aliases_do_not_share_entry_authority() {
    type Kind = molten_node_host::node_state::NodeStateNamespaceKind;
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let path = molten_node_host::node_state::NodeStatePath::parse("value").unwrap();
    for (first, second) in [(Kind::Identity, Kind::Secrets), (Kind::ControlIngress, Kind::Ingress)] {
        assert_ne!(first, second);
        assert_eq!(first.as_str(), second.as_str());
        let left = root.namespace(first).unwrap();
        let right = root.namespace(second).unwrap();
        left.write(&path, b"value").unwrap();
        assert_eq!(right.read(&path, 5).unwrap(), b"value");
        let left_entries = left.list_entries().unwrap();
        let right_entries = right.list_entries().unwrap();
        assert_eq!(left_entries.len(), 1);
        assert_eq!(right_entries.len(), 1);
        for (view, foreign) in [(&left, &right_entries[0]), (&right, &left_entries[0])] {
            assert!(view.read_entry(foreign, 5).is_err());
            assert!(view.remove_entry(foreign).is_err());
            assert_eq!(view.read(&path, 5).unwrap(), b"value");
        }
        assert_eq!(left.read_entry(&left_entries[0], 5).unwrap(), b"value");
        assert_eq!(right.read_entry(&right_entries[0], 5).unwrap(), b"value");
    }
}

#[test]
fn observations_keep_absence_denial_and_acquired_handle_use_distinct() {
    type Observation = molten_node_host::node_state::NodeStateFileObservation;
    type Error = molten_node_host::error::MoltenError;
    let temp = cap_tempfile::TempDir::new(cap_std::ambient_authority()).unwrap();
    let root = molten_node_host::node_state::NodeStateRoot::from_dir(temp.try_clone().unwrap());
    let namespace = root.identity().unwrap();
    let path = molten_node_host::node_state::NodeStatePath::parse("value").unwrap();
    assert!(matches!(namespace.observe_file(&path).unwrap(), Observation::Missing));
    assert_eq!(namespace.unix_mode(&path).unwrap(), None);
    assert_eq!(
        namespace.read(&path, 5).unwrap_err(),
        Error::invalid_harness("node state file value does not exist")
    );
    namespace.create_dir_all(&path).unwrap();
    assert!(matches!(
        namespace.observe_file(&path).unwrap(),
        Observation::NonRegular(molten_node_host::node_state::NodeStateEntryKind::Directory)
    ));
    assert_eq!(
        namespace.read(&path, 5).unwrap_err(),
        Error::invalid_harness("node state read leaf value must be a regular file")
    );
    assert_eq!(
        namespace.unix_mode(&path).unwrap_err(),
        Error::invalid_harness("node state leaf value must be a regular file")
    );
    let file_path = molten_node_host::node_state::NodeStatePath::parse("regular").unwrap();
    namespace.write(&file_path, b"value").unwrap();
    let Observation::Regular(file) = namespace.observe_file(&file_path).unwrap() else {
        panic!("expected acquired regular file");
    };
    assert_eq!(file.size(), 5);
    assert_eq!(file.unix_mode(), namespace.unix_mode(&file_path).unwrap());
    assert_eq!(file.read_bounded(5).unwrap(), b"value");
    assert!(namespace.read(&file_path, 4).is_err());
    assert_eq!(namespace.read(&file_path, 5).unwrap(), b"value");
}
