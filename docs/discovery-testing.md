# Discovery Testing Guide

This document provides comprehensive strategies for testing Aspen's peer discovery features in realistic scenarios.

## Overview

Aspen supports multiple discovery mechanisms that work together:

| Method | Purpose | Test Environment | CI/Automation |
|--------|---------|------------------|---------------|
| **mDNS** | Local network discovery | Multi-machine LAN | ❌ Requires real network |
| **Gossip** | Raft metadata broadcast | All environments | ✅ Works with manual peers |
| **DNS** | Production discovery | Cloud/multi-region | ⚠️ Requires DNS service |
| **Pkarr** | DHT-based discovery | Production | ⚠️ Requires DHT relay |
| **Manual** | Explicit peer config | All environments | ✅ Always works |

## Testing Strategies

### 1. Unit Testing (CI-Friendly)

**What to test:** Gossip message handling, peer address parsing, network factory updates

**How:** Use manual peer configuration to establish connectivity, then verify gossip works

**Example:** `tests/gossip_auto_peer_connection_test.rs::test_gossip_disabled_uses_manual_peers`

```rust
#[tokio::test]
async fn test_gossip_with_manual_peers() {
    // Bootstrap nodes with manual peer exchange
    let (h1, h2, h3) = bootstrap_nodes_with_manual_peers().await?;

    // Verify gossip broadcasts Raft metadata
    sleep(Duration::from_secs(12)).await;

    // Check network factory was updated via gossip
    assert!(h1.network_factory.peer_addrs().contains_key(&2));
    assert!(h1.network_factory.peer_addrs().contains_key(&3));
}
```

**Pros:**

- ✅ Runs in CI without external services
- ✅ Deterministic and fast
- ✅ Tests gossip protocol logic

**Cons:**

- ❌ Doesn't test mDNS/DNS/Pkarr discovery
- ❌ Doesn't validate NAT traversal

---

### 2. Multi-Machine LAN Testing (mDNS)

**What to test:** Zero-config mDNS + gossip discovery on real network

**How:** Run nodes on different machines connected to the same LAN/subnet

**Setup:**

```bash
# Machine 1 (192.168.1.10)
aspen-node --node-id 1 --cookie "lan-test-cluster"

# Machine 2 (192.168.1.11)
aspen-node --node-id 2 --cookie "lan-test-cluster"

# Machine 3 (192.168.1.12)
aspen-node --node-id 3 --cookie "lan-test-cluster"
```

**Verification:**

```bash
# Check logs for mDNS discovery messages
grep "mDNS discovered peer" /var/log/aspen-node.log

# Check logs for gossip announcements
grep "gossip announcement" /var/log/aspen-node.log

# Query cluster state
curl http://192.168.1.10:8080/state | jq .
```

**Expected behavior:**

1. Within 5 seconds: mDNS discovers peers on the LAN
2. Within 12 seconds: Gossip announces Raft node IDs
3. Nodes can be added to Raft cluster without manual configuration

**Pros:**

- ✅ Tests real mDNS multicast discovery
- ✅ Validates zero-config experience
- ✅ Proves Iroh connectivity establishment

**Cons:**

- ❌ Requires physical/VM infrastructure
- ❌ Not automatable in CI
- ❌ Sensitive to firewall/network config

---

### 3. Production-Like Testing (DNS + Pkarr + Relay)

**What to test:** Cloud/multi-region discovery with DNS, Pkarr, and relay servers

**How:** Deploy test infrastructure mimicking production

**Required infrastructure:**

1. Iroh relay server (or use n0's public relay: `https://relay.iroh.link`)
2. DNS discovery service (or use n0's public DNS: `https://dns.iroh.link`)
3. Pkarr relay (or use n0's public Pkarr: `https://pkarr.iroh.link`)

**Setup:**

```bash
# Node 1 (us-east-1)
aspen-node \
  --node-id 1 \
  --cookie "prod-test-cluster" \
  --relay-url https://relay.iroh.link \
  --enable-dns-discovery \
  --dns-discovery-url https://dns.iroh.link \
  --enable-pkarr \
  --pkarr-relay-url https://pkarr.iroh.link

# Node 2 (eu-west-1)
aspen-node \
  --node-id 2 \
  --cookie "prod-test-cluster" \
  --relay-url https://relay.iroh.link \
  --enable-dns-discovery \
  --enable-pkarr

# Node 3 (ap-southeast-1)
aspen-node \
  --node-id 3 \
  --cookie "prod-test-cluster" \
  --relay-url https://relay.iroh.link \
  --enable-dns-discovery \
  --enable-pkarr
```

**Verification:**

```bash
# Check DNS discovery queries
curl -v https://dns.iroh.link/discover/<node-endpoint-id>

# Check relay connectivity
grep "relay connection established" /var/log/aspen-node.log

# Verify Pkarr DHT publishing
grep "pkarr published" /var/log/aspen-node.log

# Check peer discovery
curl http://node1:8080/state | jq '.nodes | length'  # Should be 3
```

**Expected behavior:**

1. Within 5 seconds: DNS discovery finds initial peers
2. Within 30-60 seconds: Pkarr DHT propagation completes
3. Within 12 seconds: Gossip announces Raft metadata
4. Relay facilitates NAT traversal for direct QUIC connections

**Pros:**

- ✅ Tests production configuration
- ✅ Validates cross-region connectivity
- ✅ Proves relay, DNS, Pkarr integration

**Cons:**

- ❌ Requires external services (or self-hosted infrastructure)
- ❌ Higher latency (DNS/DHT propagation)
- ❌ Complex to automate in CI

---

### 4. Failure Scenario Testing

**What to test:** Graceful degradation when discovery services fail

**Test cases:**

#### DNS Discovery Timeout

```rust
#[tokio::test]
async fn test_dns_discovery_timeout() {
    let config = ClusterBootstrapConfig {
        iroh: IrohConfig {
            enable_dns_discovery: true,
            dns_discovery_url: Some("http://unreachable.example.com".into()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Bootstrap should succeed (DNS is non-blocking)
    let handle = bootstrap_node(config).await?;

    // But DNS discovery won't find peers
    sleep(Duration::from_secs(10)).await;
    assert_eq!(handle.network_factory.peer_addrs().len(), 0);

    // Manual peers still work as fallback
    handle.network_factory.add_peer(2, peer2_addr);
    // Raft operations succeed
}
```

#### Pkarr Relay Failure

```rust
#[tokio::test]
async fn test_pkarr_relay_unavailable() {
    let config = ClusterBootstrapConfig {
        iroh: IrohConfig {
            enable_pkarr: true,
            pkarr_relay_url: Some("http://offline.example.com".into()),
            ..Default::default()
        },
        ..Default::default()
    };

    // Bootstrap should succeed (Pkarr failure is non-fatal)
    let handle = bootstrap_node(config).await?;

    // Other discovery methods still work (mDNS, DNS, gossip)
    // Verify fallback to other mechanisms
}
```

#### All Discovery Disabled (Manual Only)

```rust
#[tokio::test]
async fn test_manual_peers_only() {
    let config = ClusterBootstrapConfig {
        iroh: IrohConfig {
            enable_gossip: false,
            enable_mdns: false,
            enable_dns_discovery: false,
            enable_pkarr: false,
            ..Default::default()
        },
        peers: vec!["2@<endpoint-id>".into()],
        ..Default::default()
    };

    let handle = bootstrap_node(config).await?;

    // Only manually configured peers are known
    assert_eq!(handle.network_factory.peer_addrs().len(), 1);
    assert!(handle.network_factory.peer_addrs().contains_key(&2));
}
```

**Pros:**

- ✅ Can run in CI (mocked failures)
- ✅ Validates error handling
- ✅ Ensures graceful degradation

**Cons:**

- ❌ Doesn't test real service failures
- ❌ May not catch all edge cases

---

## Recommended Testing Matrix

| Test Type | Frequency | Environment | Purpose |
|-----------|-----------|-------------|---------|
| **Unit tests (manual peers)** | Every commit | CI | Validate gossip protocol logic |
| **Multi-machine LAN** | Weekly | Dev lab | Verify mDNS zero-config experience |
| **Production config** | Pre-release | Staging | Validate DNS/Pkarr/relay integration |
| **Failure scenarios** | Every commit | CI | Ensure graceful degradation |

## Docker Compose Testing Setup

For testing without real infrastructure, use Docker Compose with custom networks:

```yaml
version: '3.8'

services:
  relay:
    image: n0computer/iroh-relay:latest
    ports:
      - "3478:3478"
    networks:
      - aspen-net

  aspen-1:
    image: aspen:latest
    environment:
      - ASPEN_NODE_ID=1
      - ASPEN_COOKIE=docker-test-cluster
      - ASPEN_IROH_RELAY_URL=http://relay:3478
      - ASPEN_IROH_ENABLE_MDNS=false  # Docker bridge doesn't support mDNS
      - ASPEN_IROH_ENABLE_DNS_DISCOVERY=true
    networks:
      - aspen-net
    depends_on:
      - relay

  aspen-2:
    image: aspen:latest
    environment:
      - ASPEN_NODE_ID=2
      - ASPEN_COOKIE=docker-test-cluster
      - ASPEN_IROH_RELAY_URL=http://relay:3478
      - ASPEN_IROH_ENABLE_MDNS=false
      - ASPEN_IROH_ENABLE_DNS_DISCOVERY=true
    networks:
      - aspen-net
    depends_on:
      - relay
      - aspen-1

  aspen-3:
    image: aspen:latest
    environment:
      - ASPEN_NODE_ID=3
      - ASPEN_COOKIE=docker-test-cluster
      - ASPEN_IROH_RELAY_URL=http://relay:3478
      - ASPEN_IROH_ENABLE_MDNS=false
      - ASPEN_IROH_ENABLE_DNS_DISCOVERY=true
    networks:
      - aspen-net
    depends_on:
      - relay
      - aspen-1

networks:
  aspen-net:
    driver: bridge
```

Run tests:

```bash
docker-compose up -d
sleep 30  # Wait for discovery

# Verify cluster state
docker exec aspen-1 curl http://localhost:8080/state

# Check logs for discovery
docker-compose logs | grep "discovered peer"
```

## Kubernetes Testing

For cloud-native testing:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: aspen-discovery
spec:
  clusterIP: None  # Headless service for DNS discovery
  selector:
    app: aspen
  ports:
    - port: 8080
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: aspen
spec:
  serviceName: aspen-discovery
  replicas: 3
  selector:
    matchLabels:
      app: aspen
  template:
    metadata:
      labels:
        app: aspen
    spec:
      containers:
      - name: aspen
        image: aspen:latest
        env:
        - name: ASPEN_NODE_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.labels['statefulset.kubernetes.io/pod-name']
        - name: ASPEN_COOKIE
          value: k8s-test-cluster
        - name: ASPEN_IROH_RELAY_URL
          value: https://relay.iroh.link
        - name: ASPEN_IROH_ENABLE_DNS_DISCOVERY
          value: "true"
        - name: ASPEN_IROH_DNS_DISCOVERY_URL
          value: http://aspen-discovery:8080/discover
```

## Metrics to Monitor

During discovery testing, monitor these metrics:

1. **Discovery latency:** Time from node start to first peer discovered
2. **Peer count:** Number of discovered peers over time
3. **Connection success rate:** Successful Iroh connections / discovery attempts
4. **Gossip message rate:** Announcements sent/received per node
5. **DNS query failures:** Failed DNS discovery queries
6. **Pkarr publish failures:** Failed DHT publishes
7. **Relay connection time:** Time to establish relay connection

## Troubleshooting Discovery in Tests

### mDNS not working

**Symptoms:** Nodes on same network don't discover each other

**Checks:**

1. Nodes on same subnet? `ip addr show | grep inet`
2. Multicast enabled? `ip link show | grep MULTICAST`
3. Firewall allows UDP 5353? `iptables -L | grep 5353`
4. Using localhost? mDNS doesn't work on 127.0.0.1

**Solution:** Use actual network interfaces, not localhost

### DNS discovery timing out

**Symptoms:** Bootstrap hangs, DNS query logs show timeouts

**Checks:**

1. DNS service reachable? `curl -v https://dns.iroh.link`
2. Network connectivity? `ping dns.iroh.link`
3. Correct URL? Check for typos in `dns_discovery_url`

**Solution:** Verify DNS service health, check network config

### Pkarr not publishing

**Symptoms:** Nodes don't appear in DHT, Pkarr logs show errors

**Checks:**

1. Pkarr relay reachable? `curl https://pkarr.iroh.link`
2. Relay requires auth? Check relay documentation
3. DHT propagation time? Wait 60+ seconds

**Solution:** Verify relay config, allow time for DHT propagation

## Example Test Implementation

See `tests/discovery_integration_test.rs` for a complete integration test template demonstrating:

- Manual peer fallback testing
- Failure scenario simulation
- Discovery timing validation
- Metrics collection

## See Also

- `examples/README.md` - Discovery method configuration
- `examples/production_cluster.rs` - Production deployment example
- `tests/gossip_auto_peer_connection_test.rs` - Gossip protocol tests
- `src/cluster/gossip_discovery.rs` - Discovery implementation
