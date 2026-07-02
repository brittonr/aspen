    use super::*;

    type AtomicCounter = std::sync::atomic::AtomicU64;
    type AtomicOrdering = std::sync::atomic::Ordering;

    fn create_dir_all(path: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn remove_dir_all(path: &std::path::Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    #[test]
    fn signed_announcement_binds_resource_and_rejects_wrong_key() {
        let resource = Resource::new(
            RESOURCE_ARTIFACT,
            ref_for("artifact"),
            "molten.ledger.artifact.v1",
            "ledger-local",
            "peer:source",
        );
        let announcement = announce_resource(&AnnounceResourceInput {
            peer: "peer:source",
            resource: &resource,
            signer: "peer:source",
            trust_root: "root",
            key: "key",
            policy_refs: &[],
        })
        .expect("announce");
        assert_eq!(announcement.resource, resource);
        let error = parse_announcement(&announcement.value, "root", "wrong-key").expect_err("wrong key rejected");
        assert!(error.to_string().contains("signature verification failed"));
    }

    #[test]
    fn signed_inventory_pull_imports_missing_ledger_artifacts_after_verification() {
        let source = temp_dir("federation-source");
        let destination = temp_dir("federation-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        assert_eq!(inventory.resources.len(), 1);
        let pull = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &[],
        })
        .expect("pull");
        assert_eq!(pull.imported_refs, vec![imported.artifact_ref.clone()]);
        assert_eq!(ledger::read_artifact(&destination, &imported.artifact_ref).expect("read pulled"), artifact);
        assert!(ledger::artifact_kind(&pull.receipt_value) == "federation-receipt");
    }

    #[test]
    fn tampered_inventory_signature_rejects_before_import() {
        let source = temp_dir("federation-tamper-source");
        let destination = temp_dir("federation-tamper-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        let tampered_resource = Resource::new(
            "artifact",
            imported.artifact_ref,
            "molten.ledger.artifact.v1",
            "ledger-tampered",
            "peer:source",
        );
        let tampered_payload = inventory_payload_value("peer:source", &[tampered_resource], &[]);
        let fields =
            inventory.value.collect_simple_record("federation-inventory-v1", Some(4)).expect("inventory fields");
        let tampered = record("federation-inventory-v1", vec![
            value_to_iovalue(&fields[0]),
            record("payload", vec![tampered_payload]),
            value_to_iovalue(&fields[2]),
            value_to_iovalue(&fields[3]),
        ]);
        let error = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &tampered,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &[],
        })
        .expect_err("tampered inventory rejected");
        assert!(error.to_string().contains("signature verification failed"));
        assert!(ledger::list_artifacts(&destination).expect("destination list").is_empty());
    }

    #[test]
    fn delegate_capability_is_required_when_policy_demands_it() {
        let source = temp_dir("federation-delegate-source");
        let destination = temp_dir("federation-delegate-destination");
        let artifact = record("federation-test-artifact", vec![string("hello")]);
        let imported = ledger::import_artifact(&source, &artifact).expect("source import");
        let resource = Resource::new(
            RESOURCE_ARTIFACT,
            imported.artifact_ref.clone(),
            "molten.ledger.artifact.v1",
            "ledger-local",
            "peer:source",
        );
        let delegate = delegate_resource(&resource, "pull", "delegate:source", "delegate-root", "delegate-key")
            .expect("delegate resource");
        let inventory = inventory_for_resources_with_delegates(&InventoryWithDelegatesInput {
            peer: "peer:source",
            resources: std::slice::from_ref(&resource),
            delegates: std::slice::from_ref(&delegate),
            signer: "peer:source",
            trust_root: "root",
            key: "key",
        })
        .expect("inventory with delegate");
        let policy = PullPolicy {
            required_delegate_capability: Some("pull".to_string()),
            delegate_trust_root: "delegate-root".to_string(),
            delegate_key: "delegate-key".to_string(),
            ..PullPolicy::allow_all()
        };
        let pull = pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            policy: &policy,
        })
        .expect("delegate pull");
        assert_eq!(pull.imported_refs, vec![imported.artifact_ref.clone()]);

        let no_delegate_inventory = inventory_for_resources("peer:source", &[resource], "peer:source", "root", "key")
            .expect("inventory without delegate");
        let denied_destination = temp_dir("federation-delegate-denied-destination");
        let denied = pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
            source_root: &source,
            dest_root: &denied_destination,
            inventory_value: &no_delegate_inventory.value,
            trust_root: "root",
            key: "key",
            policy: &policy,
        })
        .expect("delegate denial");
        assert!(denied.imported_refs.is_empty());
        assert_eq!(denied.denied_refs, vec![imported.artifact_ref]);
    }

    #[test]
    fn rate_limit_denies_inventory_before_fetch() {
        let source = temp_dir("federation-rate-source");
        let destination = temp_dir("federation-rate-destination");
        let first = ledger::import_artifact(&source, &record("federation-test-artifact", vec![string("one")]))
            .expect("first import");
        let second = ledger::import_artifact(&source, &record("federation-test-artifact", vec![string("two")]))
            .expect("second import");
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        let policy = PullPolicy {
            max_resources: 1,
            ..PullPolicy::allow_all()
        };
        let pull = pull_ledger_inventory_with_policy(&PullLedgerInventoryPolicyInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            policy: &policy,
        })
        .expect("rate limited pull");
        assert!(pull.imported_refs.is_empty());
        assert_eq!(pull.denied_refs.len(), 2);
        assert!(pull.denied_refs.contains(&first.artifact_ref));
        assert!(pull.denied_refs.contains(&second.artifact_ref));
        assert!(ledger::list_artifacts(&destination).expect("destination list").is_empty());
    }

    #[test]
    fn sync_status_assertions_capture_imports_and_denials() {
        let pull = Pull {
            peer: "peer:source".to_string(),
            imported_refs: vec![ref_for("imported")],
            skipped_refs: Vec::new(),
            denied_refs: vec![ref_for("denied")],
            receipt_value: receipt_value(&ReceiptValueInput {
                operation: "test",
                decision: "fail",
                peer: "peer:source",
                resources: &[],
                imported_refs: &[],
                skipped_refs: &[],
                denied_refs: &[],
            }),
        };
        let assertions = status_assertions(&pull).expect("status assertions");
        assert_eq!(assertions.len(), 3);
        assert!(assertions.iter().any(|assertion| {
            assertion.value.as_iovalue().collect_simple_record("federation-sync-status", None).is_some()
        }));
        assert!(assertions.iter().any(|assertion| {
            assertion.value.as_iovalue().collect_simple_record("federation-imported-resource", None).is_some()
        }));
        assert!(assertions.iter().any(|assertion| {
            assertion.value.as_iovalue().collect_simple_record("federation-denied-resource", None).is_some()
        }));
    }

    #[test]
    fn chunk_manifest_announcement_pulls_through_verified_chunk_store() {
        let source = temp_dir("federation-chunk-source");
        let destination = temp_dir("federation-chunk-destination");
        let iroh = temp_dir("federation-chunk-iroh");
        let put = chunk_store::put_bytes(&source, "artifact", b"abcdef", 2).expect("put chunks");
        let published =
            chunk_store::publish_iroh_blobs(&source, &iroh, &put.manifest_ref, "peer:source").expect("publish chunks");
        let resource = Resource::new(
            RESOURCE_CHUNK_MANIFEST,
            put.manifest_ref.clone(),
            "molten.chunk-store.manifest.v1",
            published.ticket,
            "peer:source",
        );
        let announcement = announce_resource(&AnnounceResourceInput {
            peer: "peer:source",
            resource: &resource,
            signer: "peer:source",
            trust_root: "root",
            key: "key",
            policy_refs: &[],
        })
        .expect("announce chunk");
        let pull = pull_chunk_manifest_from_announcement(&PullChunkManifestInput {
            iroh_root: &iroh,
            dest_root: &destination,
            announcement_value: &announcement.value,
            trust_root: "root",
            key: "key",
            peer: "peer:source",
        })
        .expect("pull chunk manifest");
        assert_eq!(pull.imported_refs.first(), Some(&put.manifest_ref));
        let read = chunk_store::read_object(&destination, &put.manifest_ref).expect("read pulled chunks");
        assert_eq!(read.bytes, b"abcdef");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_receiver_driven_sync_no_push_and_verify_before_import(tc: hegel::TestCase) {
        let count = tc.draw(hegel::generators::integers::<usize>().min_value(1).max_value(4));
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let source = temp_dir("federation-hegel-source");
        let destination = temp_dir("federation-hegel-destination");
        let mut refs = Vec::with_capacity(count);
        for index in 0..count {
            let value = record("federation-hegel-artifact", vec![string(format!("{salt}-{index}"))]);
            refs.push(ledger::import_artifact(&source, &value).expect("source import").artifact_ref);
        }
        let inventory = inventory_ledger(&source, "peer:source", "peer:source", "root", "key").expect("inventory");
        assert!(ledger::list_artifacts(&destination).expect("destination before pull").is_empty());
        let wrong_key = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "wrong-key",
            allowed_resource_types: &[],
        })
        .expect_err("wrong key fails before import");
        assert!(wrong_key.to_string().contains("signature verification failed"));
        assert!(ledger::list_artifacts(&destination).expect("destination after failed verify").is_empty());
        let pull = pull_ledger_inventory(&PullLedgerInventoryInput {
            source_root: &source,
            dest_root: &destination,
            inventory_value: &inventory.value,
            trust_root: "root",
            key: "key",
            allowed_resource_types: &[],
        })
        .expect("pull");
        assert_eq!(pull.imported_refs.len(), count);
        for reference in refs {
            assert!(pull.imported_refs.contains(&reference));
            ledger::read_artifact(&destination, &reference).expect("pulled artifact exists");
        }
    }
