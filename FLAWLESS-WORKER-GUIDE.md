# Flawless Worker Access Guide

## Quick Reference for Making Flawless Available to Workers

### The Problem

Currently, Flawless runs on `localhost:27288` **inside each container**. External workers cannot reach it.

```
Container (localhost:27288)          External Worker
  ┌──────────────────────┐           ┌──────────────┐
  │ Flawless Server      │           │ Worker       │
  │ localhost:27288  ────×─── BLOCKED ── Cannot     │
  │                      │           │   connect    │
  └──────────────────────┘           └──────────────┘
```

### Solution Overview

You have 4 options to make Flawless available to workers:

---

## Option 1: Expose Flawless Port (Simplest)

Add port mapping to `docker-compose.yml`:

```yaml
services:
  node1:
    image: mvm-ci-cluster:latest
    ports:
      - "3020:3020"    # mvm-ci API
      - "9000:9000"    # Hiqlite Raft
      - "9001:9001"    # Hiqlite API
      - "27288:27288"  # <- Add this line for Flawless
```

Worker configuration:
```bash
# External worker can now reach Flawless on host
export FLAWLESS_URL=http://localhost:27288
# OR if on different machine:
export FLAWLESS_URL=http://docker-host-ip:27288
```

**Pros:**
- Simplest to implement (one line change)
- Works immediately

**Cons:**
- Exposes Flawless to entire network
- Each node has separate Flawless (port collision if running multiple nodes)
- Port 27288 needs different mapping per node

---

## Option 2: Use Container DNS/IP (Better)

Modify `docker-compose.yml` to expose Flawless on all interfaces:

```yaml
services:
  node1:
    image: mvm-ci-cluster:latest
    container_name: mvm-ci-node1
    hostname: mvm-ci-node1
    ports:
      - "3020:3020"
      - "27288:27288"  # Map port
    networks:
      cluster:
        ipv4_address: 192.168.100.11
```

Update `docker-entrypoint.sh` to bind Flawless to all interfaces:

```bash
# Instead of just:
flawless up > /var/log/flawless.log 2>&1 &

# Configure Flawless to listen on all interfaces
# (if Flawless supports config files or env vars)
export FLAWLESS_ADDR=0.0.0.0:27288
flawless up > /var/log/flawless.log 2>&1 &
```

Worker configuration:
```bash
# Workers can reach container by DNS name
export FLAWLESS_URL=http://mvm-ci-node1:27288
# OR by IP
export FLAWLESS_URL=http://192.168.100.11:27288
```

**Pros:**
- Better than localhost
- Can use DNS names
- Works across network

**Cons:**
- Requires Flawless to support bind address configuration
- Still exposes Flawless to network
- Need to manage DNS/IP resolution

---

## Option 3: Shared Flawless Instance (Recommended)

Run ONE Flawless server for entire cluster instead of one per node:

```yaml
services:
  flawless:
    image: flawless-server:latest
    container_name: flawless
    ports:
      - "27288:27288"
    networks:
      cluster:
        ipv4_address: 192.168.100.10
    volumes:
      - flawless-data:/data/flawless

  node1:
    image: mvm-ci-cluster:latest
    depends_on:
      - flawless
    environment:
      - FLAWLESS_URL=http://flawless:27288  # <- Points to shared instance
    networks:
      cluster:
        ipv4_address: 192.168.100.11
```

Update `docker-entrypoint.sh`:

```bash
# Don't start Flawless in entrypoint
# Instead, connect to external Flawless
echo "Connecting to Flawless at $FLAWLESS_URL..."
# Wait for Flawless to be ready
wait_for_service() {
  local url=$1
  local max_attempts=30
  for i in $(seq 1 $max_attempts); do
    if curl -s "$url/health" > /dev/null 2>&1; then
      echo "✓ Flawless ready"
      return 0
    fi
    echo "Waiting for Flawless..."
    sleep 1
  done
  echo "✗ Flawless not ready after $max_attempts attempts"
  exit 1
}
wait_for_service "$FLAWLESS_URL"

# Start mvm-ci (no longer starting Flawless)
exec mvm-ci
```

Worker configuration:
```bash
export FLAWLESS_URL=http://flawless:27288
export CONTROL_PLANE_TICKET=iroh+h3://...
./worker
```

**Pros:**
- Single Flawless for all workers
- Better resource utilization
- All nodes share same module deployment
- Simplified architecture

**Cons:**
- Requires new Dockerfile for Flawless
- Single point of failure (unless HA Flawless)
- Requires refactoring entrypoint

---

## Option 4: Run Workers in Same Container (Current Pattern)

Keep Flawless per-container, but run workers inside container:

```yaml
services:
  node1:
    image: mvm-ci-cluster:latest
    # Workers run inside this container as additional processes
    command: |
      /bin/sh -c '
        /bin/docker-entrypoint.sh &
        sleep 10
        CONTROL_PLANE_TICKET=iroh+h3://... worker
      '
```

Or manage workers via Kubernetes/orchestration:

```bash
# Deploy worker as pod in same node
apiVersion: v1
kind: Pod
metadata:
  name: mvm-ci-worker-1
spec:
  containers:
  - name: worker
    image: mvm-ci:latest
    command: ["/bin/worker"]
    env:
    - name: FLAWLESS_URL
      value: "http://localhost:27288"  # Works because pod shares network namespace
    - name: CONTROL_PLANE_TICKET
      valueFrom:
        configMapKeyRef:
          name: control-plane
          key: ticket
```

**Pros:**
- No network changes needed
- localhost resolution works
- Simple for local/single-node setup

**Cons:**
- Workers can't scale independently
- Resource contention with control plane
- Single failure domain

---

## Current Code Locations to Modify

### For Port Exposure (Option 1)

**File:** `/home/brittonr/git/mvm-ci/docker-compose.yml`

```yaml
# Add to each service's ports section
ports:
  - "3020:3020"
  - "9000:9000"
  - "9001:9001"
  - "27288:27288"  # <- Add this
```

### For Flawless Binding (Option 2)

**File:** `/home/brittonr/git/mvm-ci/docker-entrypoint.sh` (line 23-25)

```bash
# Current:
cd /data
flawless up > /var/log/flawless.log 2>&1 &

# Modified (if Flawless supports it):
cd /data
FLAWLESS_ADDR=0.0.0.0:27288 flawless up > /var/log/flawless.log 2>&1 &
```

### For Shared Flawless (Option 3)

**File:** `/home/brittonr/git/mvm-ci/docker-compose.yml`

Add new service and modify control plane:

```yaml
services:
  flawless:
    image: mvm-ci-cluster:latest
    command: /bin/flawless up
    container_name: flawless
    ports:
      - "27288:27288"
    networks:
      cluster:
        ipv4_address: 192.168.100.10
    volumes:
      - flawless-data:/data/flawless

  node1:
    image: mvm-ci-cluster:latest
    depends_on:
      - flawless
    environment:
      - FLAWLESS_URL=http://flawless:27288
```

**File:** `/home/brittonr/git/mvm-ci/docker-entrypoint.sh`

Modify to skip Flawless startup:

```bash
#!/bin/sh
set -e

echo "=== Starting Node ${NODE_ID} ==="

# Create directories
mkdir -p /data/hiqlite /data/iroh /var/log /tmp

# Generate hiqlite config from template
envsubst < /etc/hiqlite.toml.template > /data/hiqlite.toml
ln -s /data/hiqlite.toml /hiqlite.toml

# SKIP Flawless startup - it's shared
echo "Using shared Flawless at: $FLAWLESS_URL"

# Start mvm-ci (it will look for hiqlite.toml in /)
echo "Starting mvm-ci..."
cd /
exec mvm-ci
```

### For In-Container Workers (Option 4)

**File:** `/home/brittonr/git/mvm-ci/src/bin/worker.rs` (line 22)

Already supports FLAWLESS_URL environment variable. Just ensure:

```rust
let flawless_url = std::env::var("FLAWLESS_URL")
    .unwrap_or_else(|_| "http://localhost:27288".to_string());
```

---

## Testing Your Configuration

### Test 1: Verify Flawless is Running

```bash
# Inside container
curl http://localhost:27288/health

# From external worker (after exposing port)
curl http://docker-host:27288/health
```

### Test 2: Verify Worker Can Connect

```bash
# Set environment variables
export FLAWLESS_URL=http://target:27288
export CONTROL_PLANE_TICKET=iroh+h3://...

# Start worker
./target/debug/mvm-ci-worker

# Should see logs like:
# Connecting to Flawless server at http://target:27288
# Connecting to control plane via iroh+h3
```

### Test 3: Submit Job from Control Plane

```bash
curl -X POST http://localhost:3020/new-job \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://example.com"
```

### Test 4: Verify Worker Executes Job

```bash
# Check worker logs for:
# Claimed job from control plane
# Executing Flawless workflow
# Job completed successfully
```

---

## Recommendation

**For Development:** Option 4 (workers in same container)
- Simplest to test
- No network complexity
- Works immediately

**For Production:** Option 3 (shared Flawless instance)
- Single Flawless for cluster
- Workers scale independently
- Better resource utilization
- Most aligned with cluster architecture

**For Quick Prototype:** Option 1 (expose port)
- Fastest to implement
- One-line change to docker-compose.yml
- Good for testing worker connectivity

---

## Flawless Configuration Notes

The Flawless binary (`flawless up`) appears to:
- Use default configuration (port 27288, localhost binding)
- Accept data directory from working directory (/data)
- Not require explicit config files

To verify what configuration options Flawless supports:
```bash
flawless --help
flawless up --help
```

Check logs at `/var/log/flawless.log` for startup behavior and any error messages.
