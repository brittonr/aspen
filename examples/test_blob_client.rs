//! Test the blob client API with a running cluster

use anyhow::Result;
use aspen_client::{AspenClient, AspenClientBlobExt, AspenClusterTicket};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Get ticket from environment or use the one from our test cluster
    let ticket = std::env::var("ASPEN_TICKET").unwrap_or_else(|_| {
        "aspensevjxhbaztq2zakiwkvw67i4vnbcxymaeov5cdzh5sdbd3u7fzzacnzhjzxhjnwtkso7gzu6fcfz2xtrrqtzaqpbhd3zm7ydketto62fcvvws5dupewwg3dvon2gk4rngeytcnjzgeza".to_string()
    });

    println!("Connecting to cluster with ticket...");

    // Parse the ticket using deserialize method
    let cluster_ticket = AspenClusterTicket::deserialize(&ticket)?;

    // Connect using the parsed ticket
    let client = AspenClient::connect_with_ticket(cluster_ticket, Duration::from_secs(5), None).await?;

    println!("Connected! Testing blob operations...\n");

    // Upload a blob
    let test_data = b"Hello from the new blob client API!";
    println!("Uploading blob: {:?}", std::str::from_utf8(test_data)?);

    let upload_result = client.blobs().upload_with_tag(test_data, Some("api-test".to_string())).await?;

    println!("Upload successful!");
    println!("  Hash: {}", upload_result.hash);
    println!("  Size: {} bytes", upload_result.size);
    println!("  Was new: {}\n", upload_result.was_new);

    // Check if blob exists
    println!("Checking if blob exists...");
    let exists = client.blobs().exists(&upload_result.hash).await?;
    println!("  Exists: {}\n", exists);

    // Download the blob
    println!("Downloading blob...");
    if let Some(download) = client.blobs().download(&upload_result.hash).await? {
        println!("  Downloaded {} bytes", download.size);
        println!("  Content: {:?}\n", std::str::from_utf8(&download.data)?);
        assert_eq!(download.data, test_data);
    } else {
        println!("  Blob not found!\n");
    }

    // Get blob status
    println!("Getting blob status...");
    if let Some(status) = client.blobs().status(&upload_result.hash).await? {
        println!("  Hash: {}", status.hash);
        println!("  Size: {:?} bytes", status.size);
        println!("  Complete: {}", status.complete);
        println!("  Tags: {:?}\n", status.tags);
    }

    // List blobs
    println!("Listing blobs...");
    let list_result = client.blobs().list(Default::default()).await?;
    println!("  Found {} blobs", list_result.blobs.len());
    for blob in &list_result.blobs {
        println!("    - {} ({} bytes)", &blob.hash[..16], blob.size);
    }

    // Test deduplication
    println!("\nTesting deduplication...");
    let upload2 = client.blobs().upload(test_data).await?;
    println!("  Second upload was new: {} (should be false)", upload2.was_new);
    assert_eq!(upload2.hash, upload_result.hash);

    println!("\nAll tests passed!");

    Ok(())
}
