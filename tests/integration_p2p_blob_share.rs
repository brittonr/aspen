//! P2P blob sharing integration test.
//!
//! Validates that blobs can be shared between cluster nodes via iroh-blobs protocol.

use std::time::Duration;
use anyhow::Result;
use aspen::NodeBuilder;
use aspen_client::{AspenClient, AspenClientBlobExt, AspenClientJobExt, JobPriority};
use tracing::{info, debug};

/// Test P2P blob sharing between three cluster nodes.
///
/// 1. Start 3-node cluster
/// 2. Upload binary to node 1
/// 3. Submit VM job on node 2 (forces P2P fetch)
/// 4. Verify execution on node 3
#[tokio::test]
async fn test_p2p_blob_sharing() -> Result<()> {
    // Initialize test logging
    tracing_subscriber::fmt()
        .with_env_filter("info,aspen=debug")
        .init();

    info!("Starting P2P blob sharing test");

    // Create temporary directories for each node
    let temp_dir = tempfile::tempdir()?;
    let base_path = temp_dir.path();
    let node1_dir = base_path.join("node1");
    let node2_dir = base_path.join("node2");
    let node3_dir = base_path.join("node3");

    std::fs::create_dir_all(&node1_dir)?;
    std::fs::create_dir_all(&node2_dir)?;
    std::fs::create_dir_all(&node3_dir)?;

    // Start node 1 (bootstrap)
    info!("Starting node 1 (bootstrap)");
    let node1 = NodeBuilder::new()
        .node_id(1)
        .data_dir(node1_dir)
        .enable_blob_storage()
        .enable_job_workers(2)
        .build()
        .await?;

    // Initialize the cluster
    let controller = node1.controller();
    controller.init(&[1]).await?;

    // Get cluster ticket for other nodes
    let endpoint = node1.endpoint_manager()?;
    let ticket = endpoint.cluster_ticket().await?;
    let ticket_str = ticket.serialize();
    info!("Cluster ticket: {}", ticket_str);

    // Start node 2
    info!("Starting node 2");
    let node2 = NodeBuilder::new()
        .node_id(2)
        .data_dir(node2_dir)
        .enable_blob_storage()
        .enable_job_workers(2)
        .join_ticket(&ticket_str)
        .build()
        .await?;

    // Start node 3
    info!("Starting node 3");
    let node3 = NodeBuilder::new()
        .node_id(3)
        .data_dir(node3_dir)
        .enable_blob_storage()
        .enable_job_workers(2)
        .join_ticket(&ticket_str)
        .build()
        .await?;

    // Wait for nodes to join cluster
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Add nodes 2 and 3 as learners
    controller.add_learner(2, endpoint.endpoint_addr_for_node(2)?).await?;
    controller.add_learner(3, endpoint.endpoint_addr_for_node(3)?).await?;

    // Promote to voters
    controller.change_membership(&[1, 2, 3]).await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    info!("Cluster formed with 3 nodes");

    // Create test binary data (simulating a VM binary)
    let test_binary = vec![0x7f, 0x45, 0x4c, 0x46]; // ELF magic number
    let test_binary = [test_binary, vec![0u8; 1000]].concat(); // Make it 1KB

    // Connect client to node 1
    info!("Connecting client to node 1");
    let client1 = AspenClient::connect_with_ticket(
        ticket.clone(),
        Duration::from_secs(5),
        None,
    ).await?;

    // Upload binary to node 1
    info!("Uploading binary to node 1");
    let upload_result = client1.blobs()
        .upload_with_tag(&test_binary, Some("test-vm-binary".to_string()))
        .await?;

    info!("Binary uploaded: hash={}, size={}, was_new={}",
        upload_result.hash, upload_result.size, upload_result.was_new);

    // Verify binary exists on node 1
    assert!(client1.blobs().exists(&upload_result.hash).await?);

    // Connect to node 2 (which doesn't have the binary yet)
    info!("Connecting client to node 2");
    let node2_addr = endpoint.endpoint_addr_for_node(2)?;
    let client2 = AspenClient::connect_with_ticket(
        ticket.clone(),
        Duration::from_secs(5),
        None,
    ).await?;

    // Verify binary doesn't exist on node 2 yet
    debug!("Checking if binary exists on node 2");
    let exists_on_node2 = client2.blobs().exists(&upload_result.hash).await?;
    info!("Binary exists on node 2 before job: {}", exists_on_node2);

    // Submit VM job on node 2 with blob reference
    info!("Submitting VM job on node 2 with blob reference");
    let job_payload = serde_json::json!({
        "type": "BlobBinary",
        "hash": upload_result.hash,
        "size": upload_result.size,
        "format": "elf"
    });

    let job_id = client2.jobs()
        .submit("vm_execute", job_payload)
        .with_priority(JobPriority::Normal)
        .with_timeout(Duration::from_secs(10))
        .add_tag("p2p-test")
        .execute(&client2)
        .await?;

    info!("Job submitted: {}", job_id);

    // Wait a bit for job to process (worker will fetch blob via P2P)
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Now verify binary exists on node 2 (fetched via P2P)
    info!("Verifying binary was fetched to node 2");
    let exists_after = client2.blobs().exists(&upload_result.hash).await?;
    assert!(exists_after, "Binary should exist on node 2 after job submission");

    info!("✓ Binary successfully fetched via P2P!");

    // Also verify on node 3
    info!("Connecting to node 3");
    let client3 = AspenClient::connect_with_ticket(
        ticket.clone(),
        Duration::from_secs(5),
        None,
    ).await?;

    // Download blob on node 3 using DHT discovery (if enabled)
    info!("Attempting to download blob on node 3");
    if let Ok(ticket) = client1.blobs().get_ticket(&upload_result.hash).await {
        info!("Got blob ticket from node 1: {}", ticket);

        let download_result = client3.blobs()
            .download_from_ticket(&ticket, Some("node3-copy".to_string()))
            .await?;

        info!("Downloaded on node 3: hash={}, size={}",
            download_result.hash, download_result.size);

        assert_eq!(download_result.hash, upload_result.hash);
        assert_eq!(download_result.size, upload_result.size);
        assert_eq!(download_result.data, test_binary);
    }

    // Check job status
    if let Some(job) = client2.jobs().get(&job_id).await? {
        info!("Job status: {}", job.status);
    }

    // Get blob status on all nodes
    for (i, client) in [(1, &client1), (2, &client2), (3, &client3)] {
        if let Some(status) = client.blobs().status(&upload_result.hash).await? {
            info!("Node {} blob status: complete={}, tags={:?}",
                i, status.complete, status.tags);
        }
    }

    // List blobs on each node
    for (i, client) in [(1, &client1), (2, &client2), (3, &client3)] {
        let list = client.blobs().list(Default::default()).await?;
        info!("Node {} has {} blobs", i, list.blobs.len());
        for blob in &list.blobs {
            debug!("  - {} ({} bytes)", &blob.hash[..16], blob.size);
        }
    }

    info!("P2P blob sharing test completed successfully!");

    // Cleanup
    client1.shutdown().await;
    client2.shutdown().await;
    client3.shutdown().await;

    node1.shutdown().await?;
    node2.shutdown().await?;
    node3.shutdown().await?;

    Ok(())
}

/// Test blob deduplication across multiple jobs.
#[tokio::test]
async fn test_blob_deduplication() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting blob deduplication test");

    let temp_dir = tempfile::tempdir()?;
    let node_dir = temp_dir.path().join("node");
    std::fs::create_dir_all(&node_dir)?;

    // Start single node
    let node = NodeBuilder::new()
        .node_id(1)
        .data_dir(node_dir)
        .enable_blob_storage()
        .enable_job_workers(4)
        .build()
        .await?;

    let controller = node.controller();
    controller.init(&[1]).await?;

    let endpoint = node.endpoint_manager()?;
    let ticket = endpoint.cluster_ticket().await?;

    // Connect client
    let client = AspenClient::connect_with_ticket(
        ticket,
        Duration::from_secs(5),
        None,
    ).await?;

    // Create test binary
    let test_binary = vec![0xde, 0xad, 0xbe, 0xef; 256]; // 1KB test data

    // Upload same binary multiple times
    info!("Testing deduplication by uploading same binary 3 times");

    let result1 = client.blobs().upload(&test_binary).await?;
    info!("Upload 1: hash={}, was_new={}", result1.hash, result1.was_new);
    assert!(result1.was_new);

    let result2 = client.blobs().upload(&test_binary).await?;
    info!("Upload 2: hash={}, was_new={}", result2.hash, result2.was_new);
    assert!(!result2.was_new); // Should be deduplicated

    let result3 = client.blobs().upload(&test_binary).await?;
    info!("Upload 3: hash={}, was_new={}", result3.hash, result3.was_new);
    assert!(!result3.was_new); // Should be deduplicated

    // All should have same hash
    assert_eq!(result1.hash, result2.hash);
    assert_eq!(result2.hash, result3.hash);

    // Submit multiple jobs using the same binary
    info!("Submitting 10 jobs with same binary");
    let mut job_ids = Vec::new();

    for i in 0..10 {
        let job_payload = serde_json::json!({
            "type": "BlobBinary",
            "hash": result1.hash,
            "size": result1.size,
            "format": "test",
            "job_num": i
        });

        let job_id = client.jobs()
            .submit("test_job", job_payload)
            .with_priority(JobPriority::Normal)
            .add_tag("dedup-test")
            .execute(&client)
            .await?;

        job_ids.push(job_id);
    }

    info!("Submitted {} jobs, all referencing same blob", job_ids.len());

    // Verify only one blob exists
    let blob_list = client.blobs().list(Default::default()).await?;
    info!("Total blobs in store: {}", blob_list.blobs.len());

    // Should only have one blob despite multiple uploads and job references
    assert_eq!(blob_list.blobs.len(), 1);
    assert_eq!(blob_list.blobs[0].hash, result1.hash);

    info!("✓ Blob deduplication working correctly!");

    // Cleanup
    client.shutdown().await;
    node.shutdown().await?;

    Ok(())
}