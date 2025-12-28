//! DNS Layer Integration Tests
//!
//! Tests the DNS layer with a real Aspen cluster.
//!
//! Only compiled when the `dns` feature is enabled.

#![cfg(feature = "dns")]

use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::time::Duration;

use aspen::api::ClusterController;
use aspen::api::ClusterNode;
use aspen::api::InitRequest;
use aspen::dns::AspenDnsStore;
use aspen::dns::DnsClientTicket;
use aspen::dns::DnsRecord;
use aspen::dns::DnsRecordData;
use aspen::dns::DnsStore;
use aspen::dns::MxRecord;
use aspen::dns::RecordType;
use aspen::dns::SrvRecord;
use aspen::dns::Zone;
use aspen::node::NodeBuilder;
use aspen::node::NodeId;
use aspen::raft::storage::StorageBackend;
use tempfile::TempDir;
use tokio::time::sleep;

const TEST_COOKIE: &str = "dns-integration-test";

/// Helper to print test progress
fn print_step(step: &str) {
    println!("\n\x1b[1;34m==> {}\x1b[0m", step);
}

fn print_success(msg: &str) {
    println!("\x1b[1;32m    ✓ {}\x1b[0m", msg);
}

fn print_record(record: &DnsRecord) {
    println!("\x1b[1;33m    📄 {} {} TTL={}\x1b[0m", record.domain, record.record_type(), record.ttl_seconds);
    match &record.data {
        DnsRecordData::A { addresses } => {
            for addr in addresses {
                println!("       → {}", addr);
            }
        }
        DnsRecordData::AAAA { addresses } => {
            for addr in addresses {
                println!("       → {}", addr);
            }
        }
        DnsRecordData::MX { records } => {
            for mx in records {
                println!("       → {} (priority {})", mx.exchange, mx.priority);
            }
        }
        DnsRecordData::TXT { strings } => {
            for s in strings {
                println!("       → \"{}\"", s);
            }
        }
        DnsRecordData::CNAME { target } => {
            println!("       → {}", target);
        }
        DnsRecordData::SRV { records } => {
            for srv in records {
                println!("       → {}:{} (priority {}, weight {})", srv.target, srv.port, srv.priority, srv.weight);
            }
        }
        _ => {
            println!("       → {:?}", record.data);
        }
    }
}

/// Test DNS CRUD operations with a single-node cluster.
#[tokio::test]
#[ignore = "requires network access - run with: cargo nextest run --run-ignored ignored-only -E 'test(dns_crud)'"]
async fn test_dns_crud_single_node() {
    print_step("Starting single-node Aspen cluster for DNS testing");

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("dns-node-1");

    let service = NodeBuilder::new(NodeId(1), &data_dir)
        .with_storage(StorageBackend::InMemory)
        .with_cookie(TEST_COOKIE)
        .with_gossip(false)
        .with_mdns(false)
        .start()
        .await
        .expect("failed to start node");

    print_success(&format!("Node 1 started with endpoint: {}", service.endpoint_addr().id));

    // Initialize cluster
    print_step("Initializing single-node cluster");
    let raft_node = service.raft_node().clone();
    let endpoint_addr = service.endpoint_addr();

    raft_node
        .init(InitRequest {
            initial_members: vec![ClusterNode {
                id: 1,
                addr: endpoint_addr.id.to_string(),
                raft_addr: None,
                iroh_addr: Some(endpoint_addr.clone()),
            }],
        })
        .await
        .expect("failed to init cluster");

    // Wait for leader election
    sleep(Duration::from_millis(500)).await;
    print_success("Cluster initialized, waiting for leader election...");

    // Create DNS store
    let dns_store = AspenDnsStore::new(raft_node.clone());

    // Test 1: Create A records
    print_step("Creating DNS records");

    let a_record = DnsRecord::new(
        "api.example.com",
        300,
        DnsRecordData::A {
            addresses: vec![Ipv4Addr::new(192, 168, 1, 10), Ipv4Addr::new(192, 168, 1, 11)],
        },
    );
    dns_store.set_record(a_record.clone()).await.expect("failed to set A record");
    print_record(&a_record);
    print_success("A record created");

    // Test 2: Create AAAA record
    let aaaa_record = DnsRecord::new(
        "api.example.com",
        300,
        DnsRecordData::AAAA {
            addresses: vec![
                "2001:db8::1".parse::<Ipv6Addr>().unwrap(),
                "2001:db8::2".parse::<Ipv6Addr>().unwrap(),
            ],
        },
    );
    dns_store.set_record(aaaa_record.clone()).await.expect("failed to set AAAA record");
    print_record(&aaaa_record);
    print_success("AAAA record created");

    // Test 3: Create MX records
    let mx_record = DnsRecord::new(
        "example.com",
        3600,
        DnsRecordData::MX {
            records: vec![
                MxRecord::new(10, "mail1.example.com"),
                MxRecord::new(20, "mail2.example.com"),
                MxRecord::new(30, "mail3.example.com"),
            ],
        },
    );
    dns_store.set_record(mx_record.clone()).await.expect("failed to set MX record");
    print_record(&mx_record);
    print_success("MX records created");

    // Test 4: Create TXT record (SPF)
    let txt_record = DnsRecord::new(
        "example.com",
        3600,
        DnsRecordData::TXT {
            strings: vec!["v=spf1 include:_spf.example.com ~all".to_string()],
        },
    );
    dns_store.set_record(txt_record.clone()).await.expect("failed to set TXT record");
    print_record(&txt_record);
    print_success("TXT record (SPF) created");

    // Test 5: Create SRV record
    let srv_record = DnsRecord::new(
        "_ldap._tcp.example.com",
        3600,
        DnsRecordData::SRV {
            records: vec![
                SrvRecord::new(10, 5, 389, "ldap1.example.com"),
                SrvRecord::new(10, 5, 389, "ldap2.example.com"),
            ],
        },
    );
    dns_store.set_record(srv_record.clone()).await.expect("failed to set SRV record");
    print_record(&srv_record);
    print_success("SRV record created");

    // Test 6: Create wildcard record
    let wildcard_record = DnsRecord::new(
        "*.example.com",
        300,
        DnsRecordData::A {
            addresses: vec![Ipv4Addr::new(192, 168, 1, 100)],
        },
    );
    dns_store.set_record(wildcard_record.clone()).await.expect("failed to set wildcard record");
    print_record(&wildcard_record);
    print_success("Wildcard A record created");

    // Test 7: Read back records
    print_step("Reading DNS records back");

    let fetched = dns_store.get_record("api.example.com", RecordType::A).await.expect("failed to get A record");
    assert!(fetched.is_some());
    print_record(&fetched.unwrap());
    print_success("A record retrieved");

    // Test 8: Get all records for a domain
    print_step("Getting all records for api.example.com");
    let all_records = dns_store.get_records("api.example.com").await.expect("failed to get all records");
    println!("    Found {} record(s):", all_records.len());
    for record in &all_records {
        print_record(record);
    }
    assert_eq!(all_records.len(), 2); // A and AAAA
    print_success("Retrieved all records for domain");

    // Test 9: Wildcard resolution
    print_step("Testing wildcard resolution");
    let resolved = dns_store.resolve("random.example.com", RecordType::A).await.expect("failed to resolve wildcard");
    println!("    Resolving random.example.com (should match *.example.com):");
    for record in &resolved {
        print_record(record);
    }
    assert!(!resolved.is_empty());
    print_success("Wildcard resolution works");

    // Test 10: Exact match takes precedence over wildcard
    print_step("Testing exact match precedence over wildcard");
    let resolved_exact = dns_store.resolve("api.example.com", RecordType::A).await.expect("failed to resolve exact");
    println!("    Resolving api.example.com (should match exact, not wildcard):");
    for record in &resolved_exact {
        print_record(record);
    }
    // Should have 2 IPs from exact match, not 1 from wildcard
    if let Some(record) = resolved_exact.first()
        && let DnsRecordData::A { addresses } = &record.data
    {
        assert_eq!(addresses.len(), 2, "Expected exact match with 2 IPs");
        print_success("Exact match takes precedence over wildcard");
    }

    // Test 11: Zone management
    print_step("Testing zone management");
    let zone = Zone::new("example.com").with_description("Production zone for example.com");
    dns_store.set_zone(zone.clone()).await.expect("failed to set zone");
    println!(
        "    Created zone: {} ({})",
        zone.name,
        zone.metadata.description.as_deref().unwrap_or("no description")
    );
    print_success("Zone created");

    let fetched_zone = dns_store.get_zone("example.com").await.expect("failed to get zone");
    assert!(fetched_zone.is_some());
    print_success("Zone retrieved");

    // Test 12: Scan records
    print_step("Scanning all DNS records");
    let all_dns = dns_store.scan_records("", 100).await.expect("failed to scan records");
    println!("    Total records in store: {}", all_dns.len());
    for record in &all_dns {
        print_record(record);
    }
    print_success(&format!("Scanned {} DNS records", all_dns.len()));

    // Test 13: Delete a record
    print_step("Deleting api.example.com AAAA record");
    let deleted = dns_store.delete_record("api.example.com", RecordType::AAAA).await.expect("failed to delete record");
    assert!(deleted);
    print_success("AAAA record deleted");

    let after_delete =
        dns_store.get_record("api.example.com", RecordType::AAAA).await.expect("failed to check deletion");
    assert!(after_delete.is_none());
    print_success("Confirmed AAAA record no longer exists");

    // Test 14: DNS ticket serialization
    print_step("Testing DNS client ticket");
    let ticket = DnsClientTicket::new("test-cluster", "namespace123", vec![service.endpoint_addr()])
        .with_priority(5)
        .with_zone_filter(vec!["example.com".to_string()])
        .expect("failed to create ticket with zone filter");

    let serialized = ticket.serialize();
    println!("    Ticket: {}...", &serialized[..50.min(serialized.len())]);

    let parsed = DnsClientTicket::deserialize(&serialized).expect("failed to parse ticket");
    assert_eq!(parsed.cluster_id, "test-cluster");
    assert_eq!(parsed.priority, 5);
    print_success("DNS ticket roundtrip successful");

    // Cleanup
    print_step("Shutting down cluster");
    service.shutdown().await.expect("failed to shutdown");
    print_success("Cluster shutdown complete");

    println!("\n\x1b[1;32m✅ All DNS integration tests passed!\x1b[0m\n");
}

/// Test DNS with 3-node cluster for replication verification.
#[tokio::test]
#[ignore = "requires network access - run with: cargo nextest run --run-ignored ignored-only -E 'test(dns_replication)'"]
async fn test_dns_replication_three_nodes() {
    print_step("Starting 3-node Aspen cluster for DNS replication testing");

    let temp_dir = TempDir::new().unwrap();

    // Start 3 nodes
    let mut services = Vec::new();
    for i in 1..=3 {
        let data_dir = temp_dir.path().join(format!("dns-node-{}", i));
        let service = NodeBuilder::new(NodeId(i), &data_dir)
            .with_storage(StorageBackend::InMemory)
            .with_cookie(TEST_COOKIE)
            .with_gossip(false)
            .with_mdns(false)
            .start()
            .await
            .unwrap_or_else(|e| panic!("failed to start node {}: {}", i, e));
        print_success(&format!("Node {} started: {}", i, service.endpoint_addr().id));
        services.push(service);
    }

    // Initialize cluster with all 3 nodes
    print_step("Initializing 3-node cluster");
    let members: Vec<ClusterNode> = services
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let addr = s.endpoint_addr();
            ClusterNode {
                id: (i + 1) as u64,
                addr: addr.id.to_string(),
                raft_addr: None,
                iroh_addr: Some(addr),
            }
        })
        .collect();

    services[0]
        .raft_node()
        .init(InitRequest {
            initial_members: members,
        })
        .await
        .expect("failed to init cluster");

    // Wait for leader election and replication
    // Multi-node clusters need more time for Iroh connections + Raft election
    print_success("Waiting for leader election (this may take a few seconds)...");
    sleep(Duration::from_secs(5)).await;
    print_success("Cluster initialized with 3 voting members");

    // Create DNS store on leader (node 1)
    let dns_store_1 = AspenDnsStore::new(services[0].raft_node().clone());

    // Write DNS records through node 1
    print_step("Writing DNS records through node 1 (leader)");
    let record = DnsRecord::new(
        "service.cluster.local",
        60,
        DnsRecordData::A {
            addresses: vec![
                Ipv4Addr::new(10, 0, 0, 1),
                Ipv4Addr::new(10, 0, 0, 2),
                Ipv4Addr::new(10, 0, 0, 3),
            ],
        },
    );
    dns_store_1.set_record(record.clone()).await.expect("failed to set record on leader");
    print_record(&record);
    print_success("Record written to leader");

    // Give time for replication
    sleep(Duration::from_millis(500)).await;

    // Read from each follower
    print_step("Reading DNS records from all nodes");
    for (i, service) in services.iter().enumerate() {
        let dns_store = AspenDnsStore::new(service.raft_node().clone());
        let fetched = dns_store
            .get_record("service.cluster.local", RecordType::A)
            .await
            .expect("failed to read from follower");
        if let Some(r) = fetched {
            println!("    Node {}:", i + 1);
            print_record(&r);
            print_success(&format!("Node {} has the record", i + 1));
        } else {
            panic!("Node {} missing record!", i + 1);
        }
    }

    // Cleanup
    print_step("Shutting down all nodes");
    for (i, service) in services.into_iter().enumerate() {
        service.shutdown().await.expect("failed to shutdown");
        print_success(&format!("Node {} shutdown", i + 1));
    }

    println!("\n\x1b[1;32m✅ DNS replication test passed!\x1b[0m\n");
}
