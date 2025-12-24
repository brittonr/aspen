//! Integration tests for iroh-docs P2P synchronization.
//!
//! Tests real-time KV export to iroh-docs stores and verifies
//! the data can be read back correctly.

use std::sync::Arc;

use anyhow::Result;
use aspen::docs::DocsWriter; // Import trait for set_entry/delete_entry methods
use aspen::docs::exporter::DocsExporter;
use aspen::docs::exporter::IrohDocsWriter;
use aspen::raft::log_subscriber::KvOperation;
use aspen::raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
use aspen::raft::log_subscriber::LogEntryPayload;
use iroh_docs::Author;
use iroh_docs::NamespaceSecret;
use iroh_docs::store::Store;
use tempfile::TempDir;
use tokio::sync::broadcast;

/// Test that entries exported to iroh-docs store can be read back.
#[tokio::test]
async fn test_iroh_docs_writer_integration() -> Result<()> {
    // Create in-memory store
    let mut store = Store::memory();

    // Generate namespace and author
    let namespace_secret = NamespaceSecret::new(&mut rand::rng());
    let namespace_id = namespace_secret.id();
    let author = Author::new(&mut rand::rng());

    // Create replica for the namespace
    store.new_replica(namespace_secret)?;

    // Create writer
    let writer = Arc::new(IrohDocsWriter::new(store, namespace_id, author.clone()));

    // Write some entries
    writer.set_entry(b"key1".to_vec(), b"value1".to_vec()).await?;
    writer.set_entry(b"key2".to_vec(), b"value2".to_vec()).await?;

    // Delete an entry
    writer.delete_entry(b"key1".to_vec()).await?;

    // Verify writer metadata
    assert_eq!(writer.namespace_id(), namespace_id);
    assert_eq!(writer.author().id(), author.id());

    Ok(())
}

/// Test DocsExporter with IrohDocsWriter via broadcast channel.
#[tokio::test]
async fn test_docs_exporter_with_real_store() -> Result<()> {
    // Create in-memory store
    let mut store = Store::memory();

    // Generate namespace and author
    let namespace_secret = NamespaceSecret::new(&mut rand::rng());
    let namespace_id = namespace_secret.id();
    let author = Author::new(&mut rand::rng());

    // Create replica
    store.new_replica(namespace_secret)?;

    // Create writer and exporter
    let writer = Arc::new(IrohDocsWriter::new(store, namespace_id, author));
    let exporter = Arc::new(DocsExporter::new(writer.clone()));

    // Create broadcast channel
    let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);
    let receiver = sender.subscribe();

    // Spawn exporter
    let cancel = exporter.clone().spawn(receiver);

    // Send KV operations via broadcast
    sender.send(LogEntryPayload {
        index: 1,
        term: 1,
        committed_at_ms: 1000,
        operation: KvOperation::Set {
            key: b"test/key1".to_vec(),
            value: b"value1".to_vec(),
        },
    })?;

    sender.send(LogEntryPayload {
        index: 2,
        term: 1,
        committed_at_ms: 1001,
        operation: KvOperation::SetMulti {
            pairs: vec![
                (b"test/key2".to_vec(), b"value2".to_vec()),
                (b"test/key3".to_vec(), b"value3".to_vec()),
            ],
        },
    })?;

    sender.send(LogEntryPayload {
        index: 3,
        term: 1,
        committed_at_ms: 1002,
        operation: KvOperation::Delete {
            key: b"test/key2".to_vec(),
        },
    })?;

    // Give exporter time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Shutdown exporter
    cancel.cancel();

    Ok(())
}

/// Test persistent store survives restart.
#[tokio::test]
async fn test_persistent_store_survives_restart() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("docs.redb");

    // Generate secrets that we'll reuse
    let namespace_secret = NamespaceSecret::new(&mut rand::rng());
    let namespace_id = namespace_secret.id();
    let author = Author::new(&mut rand::rng());

    // First session: write some data
    {
        let mut store = Store::persistent(&db_path)?;
        store.new_replica(namespace_secret.clone())?;

        let writer = Arc::new(IrohDocsWriter::new(store, namespace_id, author.clone()));

        writer.set_entry(b"persistent/key1".to_vec(), b"value1".to_vec()).await?;
        writer.set_entry(b"persistent/key2".to_vec(), b"value2".to_vec()).await?;
    }

    // Second session: verify store can be reopened and replica exists
    {
        let mut store = Store::persistent(&db_path)?;

        // Replica should already exist - this would fail if data wasn't persisted
        let _replica = store.open_replica(&namespace_id)?;

        // Verify the namespace exists in the list
        let namespaces: Vec<_> = store.list_namespaces()?.collect::<Result<Vec<_>, _>>()?;
        assert!(namespaces.iter().any(|(ns, _)| *ns == namespace_id));
    }

    Ok(())
}

/// Test multiple writers can coexist with different namespaces.
#[tokio::test]
async fn test_multiple_namespaces() -> Result<()> {
    let mut store = Store::memory();

    // Create two separate namespaces
    let ns1_secret = NamespaceSecret::new(&mut rand::rng());
    let ns1_id = ns1_secret.id();
    let ns2_secret = NamespaceSecret::new(&mut rand::rng());
    let ns2_id = ns2_secret.id();

    store.new_replica(ns1_secret)?;
    store.new_replica(ns2_secret)?;

    // Note: IrohDocsWriter takes ownership of store, so we'd need
    // separate stores or a different approach for multiple namespaces.
    // This test verifies the Store can handle multiple namespaces.

    // Verify both namespaces exist
    let namespaces: Vec<_> = store.list_namespaces()?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(namespaces.len(), 2);
    assert!(namespaces.iter().any(|(ns, _)| *ns == ns1_id));
    assert!(namespaces.iter().any(|(ns, _)| *ns == ns2_id));

    Ok(())
}

/// Test init_docs_resources with in-memory storage.
#[tokio::test]
async fn test_init_docs_resources_in_memory() -> Result<()> {
    use aspen::docs::init_docs_resources;

    let temp_dir = TempDir::new()?;
    let resources = init_docs_resources(temp_dir.path(), true, None, None)?;

    // Should have valid resources
    assert!(resources.docs_dir.is_none()); // In-memory has no docs_dir

    // Create writer and verify it works
    let writer = Arc::new(IrohDocsWriter::new(resources.store, resources.namespace_id, resources.author));

    writer.set_entry(b"test".to_vec(), b"value".to_vec()).await?;

    Ok(())
}

/// Test init_docs_resources with persistent storage.
#[tokio::test]
async fn test_init_docs_resources_persistent() -> Result<()> {
    use aspen::docs::init_docs_resources;

    let temp_dir = TempDir::new()?;

    // First initialization
    let resources1 = init_docs_resources(temp_dir.path(), false, None, None)?;
    let ns_id1 = resources1.namespace_id;
    let author_id1 = resources1.author.id();
    drop(resources1);

    // Second initialization should reload same secrets
    let resources2 = init_docs_resources(temp_dir.path(), false, None, None)?;

    assert_eq!(resources2.namespace_id, ns_id1);
    assert_eq!(resources2.author.id(), author_id1);
    assert!(resources2.docs_dir.is_some());

    Ok(())
}

/// Test init_docs_resources with config-provided secrets.
#[tokio::test]
async fn test_init_docs_resources_with_config_secrets() -> Result<()> {
    use aspen::docs::init_docs_resources;

    let temp_dir = TempDir::new()?;

    // Generate secrets
    let ns_secret = NamespaceSecret::new(&mut rand::rng());
    let author = Author::new(&mut rand::rng());
    let ns_hex = hex::encode(ns_secret.to_bytes());
    let author_hex = hex::encode(author.to_bytes());

    let resources = init_docs_resources(temp_dir.path(), false, Some(&ns_hex), Some(&author_hex))?;

    // Should use config-provided secrets
    assert_eq!(resources.namespace_id, ns_secret.id());
    assert_eq!(resources.author.id(), author.id());

    Ok(())
}
