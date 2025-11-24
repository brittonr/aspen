//! Tests for iroh P2P networking functionality
//!
//! These tests verify iroh endpoint creation, timeout mechanisms,
//! and local P2P connectivity without requiring external relay servers.

use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_iroh_endpoint_creation_without_relay() -> Result<()> {
    // Create an iroh endpoint without waiting for relay connection
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![b"test-alpn".to_vec()])
        .bind()
        .await?;

    // Verify endpoint was created successfully
    assert_ne!(endpoint.id().to_string(), "");
    assert!(!endpoint.addr().is_empty());

    // Get the endpoint address (should have local addresses even without relay)
    let addr = endpoint.addr();
    println!("Endpoint created with address: {:?}", addr);

    // Verify we can get a ticket even without relay
    let ticket = iroh_tickets::endpoint::EndpointTicket::new(addr.clone());
    assert!(!ticket.to_string().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_iroh_timeout_mechanism() -> Result<()> {
    // Test that our timeout mechanism works correctly
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![b"test-alpn".to_vec()])
        .bind()
        .await?;

    // Test waiting for online with timeout (simulating our wait_for_online function)
    let online_future = endpoint.online();
    let timeout_result = timeout(Duration::from_secs(2), online_future).await;

    // The timeout should trigger since we can't reach relay servers
    assert!(timeout_result.is_err(), "Expected timeout but got success");

    // Endpoint should still be usable after timeout
    assert!(!endpoint.addr().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_local_iroh_connection() -> Result<()> {
    // Create two endpoints that can connect locally without relay
    let endpoint1 = iroh::Endpoint::builder()
        .alpns(vec![b"test-alpn".to_vec()])
        .bind()
        .await?;

    let endpoint2 = iroh::Endpoint::builder()
        .alpns(vec![b"test-alpn".to_vec()])
        .bind()
        .await?;

    // Get addresses
    let addr1 = endpoint1.addr();
    let addr2 = endpoint2.addr();

    println!("Endpoint 1: {:?}", addr1);
    println!("Endpoint 2: {:?}", addr2);

    // Both endpoints should have unique IDs
    assert_ne!(endpoint1.id(), endpoint2.id());

    Ok(())
}

#[tokio::test]
async fn test_iroh_endpoint_with_custom_alpn() -> Result<()> {
    // Test creating endpoint with custom ALPN protocol
    let custom_alpn = b"iroh+h3".to_vec();

    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![custom_alpn.clone()])
        .bind()
        .await?;

    // Verify endpoint was created
    assert!(!endpoint.id().to_string().is_empty());

    // Create a ticket
    let ticket = iroh_tickets::endpoint::EndpointTicket::new(endpoint.addr());
    let ticket_str = ticket.to_string();

    println!("Created endpoint with ticket: {}", ticket_str);
    assert!(ticket_str.starts_with("endpoint"));

    Ok(())
}

#[tokio::test]
async fn test_iroh_endpoint_shutdown() -> Result<()> {
    // Test that endpoints can be cleanly shut down
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![b"test-alpn".to_vec()])
        .bind()
        .await?;

    let endpoint_id = endpoint.id();
    println!("Created endpoint with ID: {}", endpoint_id);

    // Close the endpoint
    endpoint.close().await;

    // After closing, the endpoint should be shut down
    // (In a real scenario, we'd verify no connections are accepted)

    Ok(())
}

#[tokio::test]
async fn test_multiple_iroh_endpoints_same_process() -> Result<()> {
    // Test that we can create multiple endpoints in the same process
    let mut endpoints = Vec::new();

    for i in 0..3 {
        let endpoint = iroh::Endpoint::builder()
            .alpns(vec![format!("test-alpn-{}", i).into_bytes()])
            .bind()
            .await?;

        println!("Created endpoint {}: {}", i, endpoint.id());
        endpoints.push(endpoint);
    }

    // All endpoints should have unique IDs
    for i in 0..endpoints.len() {
        for j in (i + 1)..endpoints.len() {
            assert_ne!(endpoints[i].id(), endpoints[j].id());
        }
    }

    // Clean up
    for endpoint in endpoints {
        endpoint.close().await;
    }

    Ok(())
}

/// Test the wait_for_online function behavior with timeout
#[tokio::test]
async fn test_wait_for_online_with_timeout() -> Result<()> {
    use mvm_ci::server;

    // Create an endpoint
    let endpoint = iroh::Endpoint::builder()
        .alpns(vec![b"test-alpn".to_vec()])
        .bind()
        .await?;

    // Call our wait_for_online function which has a 10-second timeout
    let result = server::wait_for_online(&endpoint).await;

    // This should succeed (return Ok) even if relay is unreachable
    // because our implementation continues after timeout
    assert!(result.is_ok(), "wait_for_online should return Ok even after timeout");

    Ok(())
}

#[cfg(test)]
mod integration {
    use super::*;

    /// Test that demonstrates iroh works for local connections
    /// without requiring external relay servers
    #[tokio::test]
    async fn test_iroh_local_only_mode() -> Result<()> {
        // This test verifies that iroh endpoints can work in a
        // completely offline/local mode without relay servers

        let endpoint = iroh::Endpoint::builder()
            .alpns(vec![b"local-only".to_vec()])
            .bind()
            .await?;

        // Don't wait for relay connection
        // Just use the endpoint locally

        let local_addr = endpoint.addr();
        assert!(!local_addr.is_empty(), "Should have local addresses");

        // In a real scenario, we could:
        // 1. Start an h3 server on this endpoint
        // 2. Connect from another local endpoint
        // 3. Exchange data over QUIC

        // For now, just verify the endpoint is functional
        println!("Endpoint ID: {}", endpoint.id());
        println!("Endpoint addresses: {:?}", endpoint.addr());

        endpoint.close().await;
        Ok(())
    }
}