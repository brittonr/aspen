# Deployment Strategy for Workers with Flawless WASM Execution

## Executive Summary

The mvm-ci project has a clear separation between **control plane** and **worker plane** with P2P communication via iroh+h3. This document analyzes four deployment strategies for providing workers with access to Flawless WASM execution, recommending **Option 4 (Shared Control Plane Flawless for testing) + Option 1 (Distributed Flawless for production)** as the best long-term approach.

**Key Finding:** The current codebase already has 90% of what's needed. Workers are a separate binary (`worker`) that connects to the control plane via P2P and claims jobs. The missing piece is deciding how to deploy Flawless servers.

---

## Current Architecture

### Control Plane (`mvm-ci` binary)
- **Role:** API server + job queue manager
- **Listeners:**
  - `localhost:3020` (HTTP) for local WASM workflows
  - P2P listener (iroh+h3) for distributed coordination
- **Services:**
  - Flawless server on `localhost:27288` (hardcoded)
  - Hiqlite distributed state store
  - iroh gossip and blob storage
  - Work queue API

### Worker (`worker` binary)
- **Role:** Job execution + result reporting
- **Connections:**
  - P2P to control plane via iroh+h3 (using CONTROL_PLANE_TICKET env var)
  - HTTP to Flawless server (using FLAWLESS_URL env var, defaults to `localhost:27288`)
- **Behavior:**
  - Claims jobs from control plane via work queue API
  - Executes using FlawlessWorker backend
  - Updates job status via P2P API

### Work Queue Architecture
- Control plane hosts hiqlite database
- API endpoints: `/queue/claim`, `/queue/publish`, `/queue/status/{job_id}`
- WorkQueueClient over iroh+h3 for remote workers
- Job workflow: Claim → Execute → Update Status → Repeat

---

## Deployment Option Analysis

### OPTION 1: Each Worker Runs Its Own Flawless Instance

```
┌─────────────────────────┐
│   Control Plane         │
│  ┌───────────────────┐  │
│  │ Flawless Server   │  │
│  │ :27288 (localhost)│  │
│  └───────────────────┘  │
│  Work Queue API (P2P)   │
└─────────────────────────┘
           ▲
           │ P2P (iroh+h3)
           │
    ┌──────┴──────┐
    │             │
┌───┴────┐    ┌──┴──────┐
│Worker 1 │    │Worker 2 │
│┌──────┐ │    │┌──────┐ │
││Flaw. │ │    ││Flaw. │ │
││:5000 │ │    ││:5000 │ │
│└──────┘ │    │└──────┘ │
└─────────┘    └─────────┘
```

**Pros:**
- Complete isolation between workers
- No resource contention
- Failure in one worker doesn't affect others
- Simple scaling: add more worker containers
- Natural fit for containerized deployment
- Each worker is independently testable
- Aligns with etcd-style architecture (worker plane isolation)

**Cons:**
- Memory overhead (each Flawless instance ~50-100MB?)
- Network overhead (each worker downloads its own Flawless binary)
- Deployment complexity (Nix build + container startup per worker)
- Higher operational complexity (monitoring N Flawless instances)
- Cold start latency (deploy → download → initialize)

**Scalability:** Excellent (linear)
**Resource Usage:** High (N×Flawless memory + startup time)
**Operational Complexity:** Moderate-High
**Security:** Good (cryptographic isolation via iroh node IDs)

**When to Use:** Production MicroVM deployments with many workers (20+)

---

### OPTION 2: Workers Share Control Plane's Flawless

```
┌─────────────────────────────────────────┐
│         Control Plane                   │
│  ┌─────────────────────────────────┐   │
│  │ Flawless Server                 │   │
│  │ :27288 (localhost + P2P proxy)  │   │
│  └─────────────────────────────────┘   │
│  Work Queue API (P2P)                   │
│  - Proxies FLAWLESS_URL requests        │
└─────────────────────────────────────────┘
           ▲
           │ P2P (iroh+h3)
           │ FLAWLESS_URL points to
           │ control plane endpoint
    ┌──────┴──────┐
    │             │
┌───┴────┐    ┌──┴──────┐
│Worker 1 │    │Worker 2 │
│(no local)   │(no local)
│Flawless │    │Flawless │
└─────────┘    └─────────┘
```

**Pros:**
- Minimal memory overhead
- Single Flawless instance to manage
- Fast deployment (no binary download)
- Good for development/testing
- Simplest integration effort
- Single point for monitoring

**Cons:**
- **Resource bottleneck:** Single Flawless can saturate with many workers
- **Failure domain:** Control plane Flawless failure stops all workers
- **Contention:** Many concurrent jobs compete for CPU/memory
- **Latency:** Network hop for each job (P2P routing)
- **Not production-ready:** No redundancy or load distribution
- **Couples planes:** Violates control plane / worker plane separation
- **Network complexity:** Requires control plane to proxy Flawless URLs

**Scalability:** Poor (bottleneck at single Flawless)
**Resource Usage:** Low
**Operational Complexity:** Low
**Security:** Good (but requires careful network isolation)

**When to Use:** Quick validation, development, POC testing (5-10 workers max)

---

### OPTION 3: Dedicated Flawless Service Containers

```
┌──────────────────────────────┐
│   Control Plane              │
│  Work Queue API (P2P)        │
└──────────────────────────────┘
           ▲
           │ P2P
           │
    ┌──────┴──────┬──────────┐
    │             │          │
┌───┴────┐    ┌──┴──────┐ ┌─┴──────┐
│Worker 1 │    │Worker 2 │ │Worker 3 │
└──┬──────┘    └──┬──────┘ └──┬─────┘
   │              │           │
   └──────┬───────┴───────┬───┘
          │               │
      ┌───┴────┐    ┌─────┴──┐
      │Flawless │    │Flawless│
      │Service 1│    │Service 2│
      │:5000    │    │:5000    │
      └─────────┘    └─────────┘
      (3-5 replicas via load balancer)
```

**Pros:**
- Load distribution across multiple Flawless instances
- Better scalability than shared control plane
- Redundancy (if one fails, others continue)
- Dedicated resources for WASM execution
- Can scale independently from workers
- Good monitoring per Flawless instance

**Cons:**
- Significant operational complexity
- Requires service discovery / load balancing
- Workers need to discover Flawless endpoints
- Session affinity concerns (if any stateful workflows)
- More containers to manage and monitor
- Kubernetes-like orchestration needed for production
- Complex deployment logic (which worker → which Flawless?)

**Scalability:** Good (with load balancing)
**Resource Usage:** Medium (M×Flawless, but better shared than 1:1)
**Operational Complexity:** High
**Security:** Good (separate service tier)

**When to Use:** Large-scale production (100+ workers), when you have Kubernetes/orchestration

---

### OPTION 4: Workers Use Control Plane's Flawless (With Localhost Proxy)

**Best approach for short-term testing:**

```
┌─────────────────────────────────────────┐
│         Control Plane Container         │
│  ┌─────────────────────────────────┐   │
│  │ mvm-ci (API server)             │   │
│  │ - localhost:3020 (HTTP)         │   │
│  │ - iroh P2P listener             │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │ Flawless Server                 │   │
│  │ - localhost:27288 (HTTP)        │   │
│  │ - Exposed via container network │   │
│  └─────────────────────────────────┘   │
│  Hiqlite + iroh blob storage            │
└─────────────────────────────────────────┘
           ▲
           │ P2P (iroh+h3)
           │
    ┌──────┴──────┐
    │             │
┌───┴────┐    ┌──┴──────┐
│Worker 1 │    │Worker 2 │
│Container│    │Container│
│         │    │         │
│FLAWLESS_URL= │FLAWLESS_URL=
│control_plane:27288
└─────────┘    └─────────┘
```

**Implementation:**
1. Run control plane in a container with Flawless server on `127.0.0.1:27288`
2. Expose Flawless on container network interface: `0.0.0.0:27288`
3. Workers in same network reference it as `http://control-plane:27288`
4. Workers connect to control plane API via P2P for work queue

**Pros:**
- Simple to implement (minimal changes needed)
- Good for testing architecture (10-20 workers)
- Control plane and Flawless co-located (natural)
- Clear separation between work distribution (P2P) and execution (localhost HTTP)
- Matches current codebase design philosophy
- Easy Docker Compose setup

**Cons:**
- Still has resource bottleneck at single Flawless
- Not suitable for large-scale deployment (100+ workers)
- Requires network configuration (container network setup)
- Workers need DNS/service discovery
- Testing only (not production-ready at scale)

**Scalability:** Poor (like Option 2, single bottleneck)
**Resource Usage:** Low
**Operational Complexity:** Low
**Security:** Good (same container network, firewall-able)

**When to Use:** Short-term testing, validation, CI/CD, development

---

## Current Code Support Analysis

### What's Already Implemented

1. **Worker Binary** (`src/bin/worker.rs`)
   - Connects to control plane via `CONTROL_PLANE_TICKET`
   - Uses `FlawlessWorker` backend
   - Implements job claim/execute/update loop ✓

2. **WorkQueueClient** (`src/work_queue_client.rs`)
   - HTTP/3 over iroh P2P communication ✓
   - Claims work, updates status ✓

3. **FlawlessWorker** (`src/worker_flawless.rs`)
   - Connects to Flawless server via `FLAWLESS_URL`
   - Executes jobs and returns results ✓

4. **Work Queue API** (`src/main.rs`)
   - `/queue/claim` ✓
   - `/queue/status/{job_id}` ✓
   - `/queue/publish` ✓
   - `/queue/stats` ✓

### What's Missing (By Option)

| Feature | Option 1 | Option 2 | Option 3 | Option 4 |
|---------|----------|----------|----------|----------|
| Worker binary | ✓ | ✓ | ✓ | ✓ |
| P2P work queue | ✓ | ✓ | ✓ | ✓ |
| Flawless deployment | Manual | Shared | Load-balanced | Shared |
| Container networking | Simple | Simple | Complex | Simple |
| Service discovery | None | DNS | Required | DNS |
| Production-ready | Yes | No | Yes | No |
| Test effort | High | Low | High | Low |
| Scaling workers | Linear | Sublinear | Sublinear | Sublinear |

---

## Recommended Strategy

### Short-Term (Testing & Validation): Option 4

**Goal:** Validate that worker architecture works correctly

**Implementation:**
```bash
# Start control plane with Flawless exposed
docker run -d \
  --name control-plane \
  --network mvm \
  -p 3020:3020 \
  -v /tmp/cp-data:/data \
  mvm-ci:latest \
  /app/mvm-ci

# Get endpoint ticket
TICKET=$(docker exec control-plane cat /data/endpoint.ticket)

# Start multiple workers (same network)
for i in {1..5}; do
  docker run -d \
    --name worker-$i \
    --network mvm \
    -e CONTROL_PLANE_TICKET="$TICKET" \
    -e FLAWLESS_URL="http://control-plane:27288" \
    mvm-ci:latest \
    /app/worker
done

# Monitor
docker logs -f control-plane
docker logs -f worker-1
```

**Testing Checklist:**
- [ ] Control plane starts and exports endpoint ticket
- [ ] Workers connect to control plane successfully
- [ ] Work queue claim/release works across workers
- [ ] Job execution via Flawless completes
- [ ] Status updates propagate back to control plane
- [ ] Multiple workers don't interfere with each other
- [ ] P2P connectivity works (test with firewall rules)
- [ ] Gossip/blob storage syncs correctly

**Duration:** 1-2 weeks

---

### Medium-Term (Pilot/Limited Deployment): Option 4 → Option 1

**Goal:** Move toward production-ready architecture

**Phase 1 (Weeks 3-6):** Keep Option 4, add monitoring
- Add Prometheus metrics for Flawless instance
- Monitor CPU/memory/concurrency per worker
- Find bottleneck threshold (e.g., max 20 workers per Flawless)
- Document resource limits

**Phase 2 (Weeks 7-10):** Implement Option 1 infrastructure
- Create Nix package for standalone worker binary
- Write Dockerfile for single-worker container with embedded Flawless
- Test MicroVM deployment with 5-10 workers
- Verify P2P connectivity in isolated network

**Phase 3 (Weeks 11-14):** Production hardening
- Add health checks (Flawless + worker)
- Implement graceful shutdown (drain jobs, cleanup)
- Add metrics export (Prometheus scrape)
- Write deployment documentation

---

### Long-Term (Production): Option 1 + Option 3 Hybrid

**Recommended for large-scale MicroVM deployment:**

```
┌─────────────────────────────────────────┐
│    Control Plane (1 instance)           │
│  - Work queue API (hiqlite)             │
│  - Iroh gossip coordination             │
│  - Job distribution logic               │
│  (No local Flawless)                    │
└─────────────────────────────────────────┘
           │
           ├─────────────────────┬────────────────────┐
           │                     │                    │
    ┌──────▼────┐         ┌──────▼────┐        ┌─────▼─────┐
    │ Worker 1   │         │ Worker 2   │        │ Worker N   │
    │ MicroVM    │         │ MicroVM    │        │ MicroVM    │
    │┌────────┐  │         │┌────────┐  │        │┌────────┐  │
    ││Flawless│  │         ││Flawless│  │        ││Flawless│  │
    ││:27288  │  │         ││:27288  │  │        ││:27288  │  │
    │└────────┘  │         │└────────┘  │        │└────────┘  │
    │ Job loop   │         │ Job loop   │        │ Job loop   │
    │ Worker bin │         │ Worker bin │        │ Worker bin │
    └────────────┘         └────────────┘        └────────────┘
```

**Architecture Principles:**
1. **Worker plane is fully distributed:** Each worker has its own Flawless instance
2. **Control plane is minimal:** Only manages work queue and gossip coordination
3. **Isolation by design:** Worker failures don't affect others
4. **Scale horizontally:** Add workers as needed
5. **Resource predictable:** Each worker uses ~X MB, latency ~Y ms

**Deployment Stack:**
- Nix flakes for reproducible builds
- MicroVM manager (e.g., systemd-nspawn, qemu-kvm, or Firecracker)
- Container registry for worker image caching
- etcd for distributed state (or extend hiqlite)
- Prometheus for metrics
- Control plane behind load balancer (if HA needed)

**Worker Lifecycle:**
```
New MicroVM instance
  ↓
Boot from Nix closure (2-5s)
  ↓
Start Flawless server (2-3s)
  ↓
Start worker binary
  ↓
Connect to control plane via CONTROL_PLANE_TICKET
  ↓
Begin claiming jobs
  ↓
Job → Execute → Status Update → Repeat
  ↓
(Eventually) Shutdown signal
  ↓
Drain jobs (finish current, reject new)
  ↓
Cleanup resources
  ↓
Terminate instance
```

---

## Technical Deep Dive: Option 4 Implementation

### Docker Compose Example

```yaml
version: '3.8'
services:
  control-plane:
    build:
      context: .
      dockerfile: Dockerfile.control-plane
    container_name: mvm-cp
    networks:
      - mvm
    ports:
      - "3020:3020"
    volumes:
      - cp-data:/data
    environment:
      FLAWLESS_URL: "http://127.0.0.1:27288"  # Local
      HTTP_PORT: "3020"
      IROH_BLOBS_PATH: "/data/iroh-blobs"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3020"]
      interval: 10s
      timeout: 5s
      retries: 3

  worker-1:
    build:
      context: .
      dockerfile: Dockerfile.worker
    container_name: mvm-worker-1
    depends_on:
      control-plane:
        condition: service_healthy
    networks:
      - mvm
    environment:
      CONTROL_PLANE_TICKET: "${CONTROL_PLANE_TICKET}"
      FLAWLESS_URL: "http://control-plane:27288"
      RUST_LOG: "info"
    healthcheck:
      test: ["CMD", "pgrep", "-f", "worker"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  cp-data:

networks:
  mvm:
    driver: bridge
```

### Dockerfile.control-plane

```dockerfile
FROM nixos/nix:latest

COPY . /src
WORKDIR /src

RUN nix build .#mvm-ci && mkdir -p /app && \
    cp result/bin/mvm-ci /app/ && \
    chmod +x /app/mvm-ci

# Include Flawless binary
RUN nix build .#flawless && \
    cp result/bin/flawless /app/ && \
    chmod +x /app/flawless

# Start both flawless and mvm-ci
ENTRYPOINT ["sh", "-c", "/app/flawless --port 27288 & /app/mvm-ci"]
```

### Dockerfile.worker

```dockerfile
FROM nixos/nix:latest

COPY . /src
WORKDIR /src

RUN nix build .#worker && mkdir -p /app && \
    cp result/bin/worker /app/ && \
    chmod +x /app/worker

ENTRYPOINT ["/app/worker"]
```

### Startup Script

```bash
#!/bin/bash
set -e

# Build images
docker-compose build

# Start control plane
docker-compose up -d control-plane

# Wait for control plane to be ready
sleep 5

# Extract ticket
TICKET=$(docker-compose exec -T control-plane \
  sh -c 'cat /data/endpoint.ticket' || echo "")

if [ -z "$TICKET" ]; then
  echo "Failed to get control plane ticket"
  exit 1
fi

echo "Control Plane Ticket: $TICKET"

# Start workers with ticket
export CONTROL_PLANE_TICKET="$TICKET"
docker-compose up -d --scale worker=5

echo "Cluster started with 1 control plane + 5 workers"
docker-compose logs -f control-plane
```

---

## Migration Path

### Week 1-2: Setup & Testing (Option 4)
- [ ] Create Dockerfile for control plane with Flawless
- [ ] Create Dockerfile for worker
- [ ] Write docker-compose file
- [ ] Test with 5 workers
- [ ] Verify job execution works end-to-end
- [ ] Document testing procedure

### Week 3-4: Monitoring & Bottleneck Analysis (Option 4)
- [ ] Add Prometheus metrics to Flawless/worker
- [ ] Run load tests (100, 500, 1000 concurrent jobs)
- [ ] Identify bottleneck threshold
- [ ] Document resource requirements per worker
- [ ] Create capacity planning guide

### Week 5-8: Production Infrastructure (Option 1)
- [ ] Write Nix package for standalone worker
- [ ] Create MicroVM image with embedded Flawless
- [ ] Test single MicroVM deployment
- [ ] Test multi-worker MicroVM deployment (5-10)
- [ ] Verify P2P connectivity across VMs
- [ ] Document deployment procedure

### Week 9+: Optimization & Scale Testing
- [ ] Load test with 50+ workers
- [ ] Optimize Flawless caching/warmup
- [ ] Implement worker health checks
- [ ] Add graceful shutdown handling
- [ ] Production readiness checklist

---

## Comparison Matrix

| Criteria | Option 1 | Option 2 | Option 3 | Option 4 |
|----------|----------|----------|----------|----------|
| **Development Effort** | Medium | Low | High | Low |
| **Testing Effort** | High | Low | High | Low |
| **Deployment Complexity** | Low | Low | High | Low |
| **Production Ready** | Yes | No | Yes (complex) | No |
| **Max Workers (Testing)** | 100+ | 5 | 50 | 20 |
| **Max Workers (Production)** | 1000+ | 50 | 500 | N/A |
| **Memory per Worker** | 100MB | 0 | 20MB | 0 |
| **Failure Isolation** | Complete | None | Partial | None |
| **Scalability** | Linear | Logarithmic | Linear | Logarithmic |
| **Operational Overhead** | Medium | Low | High | Low |
| **Container Count at 100 workers** | 101 | 1 | 10-20 | 101 |

---

## Decision Matrix: Which Option to Choose?

```
Are you testing the worker architecture?
├─ YES → Option 4 (Option 2 if under 5 workers)
│
Are you deploying to production with <20 workers?
├─ YES → Option 4 + monitoring, plan for Option 1
│
Are you deploying to production with 20-100 workers?
├─ YES → Option 1 (each worker in container)
│
Are you deploying to production with 100+ workers?
├─ YES → Option 1 with orchestration (Kubernetes / custom scheduler)
│
Do you have Kubernetes and want managed Flawless?
├─ YES → Option 3 (dedicated service + workers)
```

---

## Recommendations Summary

### Best for Short-Term Testing
**Option 4: Shared Control Plane Flawless**
- Minimal implementation effort
- Validates architecture quickly
- Good for 10-20 worker testing
- Identified bottleneck threshold
- Clear path to production (Option 1)

### Best for Production (MicroVM)
**Option 1: Each Worker Has Its Own Flawless**
- Complete isolation between workers
- Natural fit for MicroVM deployment
- Scale to thousands of workers
- Failure in one worker doesn't affect others
- Clear operational model (scale = add VMs)

### Why NOT Option 2
- Only works for tiny clusters (<5 workers)
- Single point of failure
- Resource bottleneck makes it not production-ready
- No advantage over Option 4 for testing

### Why NOT Option 3
- Adds unnecessary complexity for this architecture
- Load balancing + service discovery overhead
- Better to just run Flawless in each worker (Option 1)
- Makes sense only if: (a) Flawless is super expensive, or (b) you have Kubernetes

---

## Next Steps

1. **Immediate (This Sprint):**
   - Implement Option 4 setup (docker-compose)
   - Validate worker connectivity and job execution
   - Document testing procedure

2. **Short-term (Next 2-4 Weeks):**
   - Add monitoring/metrics to Option 4
   - Run load tests to find bottleneck
   - Begin Option 1 infrastructure work

3. **Medium-term (Weeks 5-10):**
   - Complete Option 1 implementation
   - Test with 50+ workers
   - Production hardening

4. **Long-term (Production):**
   - Deploy Option 1 at scale
   - Add orchestration layer as needed
   - Optimize based on real-world usage

---

## Files to Create/Modify

### For Option 4 Testing

```
Dockerfile.control-plane      # New
Dockerfile.worker            # New
docker-compose.yml           # New
scripts/start-cluster.sh      # New
docs/DEPLOYMENT.md           # New
tests/integration/docker.rs  # New
```

### For Option 1 Production

```
nix/worker.nix              # New
nix/worker-vm.nix           # New (MicroVM config)
docs/PRODUCTION_DEPLOY.md   # New
scripts/build-worker-image  # New
tests/integration/k8s.rs    # New (if using K8s)
```

### Existing Files to Modify

```
src/bin/worker.rs           # Add endpoint ticket validation
src/worker_flawless.rs      # Add connection pooling, error handling
Cargo.toml                  # Already complete
flake.nix                   # Add worker package
```

---

## Risk Analysis

### Option 4 Risks
- **Bottleneck at scale:** If load test shows Flawless can only handle 5-10 workers, need immediate pivot
- **Network contention:** P2P routing might add latency if control plane becomes I/O bound
- **Mitigation:** Run load tests early (week 1)

### Option 1 Risks
- **Deployment complexity:** Each MicroVM needs Nix closure, Flawless binary, worker binary
- **Cold start time:** Might be 5-10s per new worker
- **Memory overhead:** If 1000+ workers, might need 100GB+ for Flawless instances alone
- **Mitigation:** Pre-built Nix closures, Flawless binary caching, instance pooling

### Option 3 Risks
- **Operational overhead:** Load balancing + service discovery adds complexity
- **Session affinity:** If workflows have state, need sticky sessions
- **Mitigation:** Stateless workflow design, sticky sessions at load balancer

---

## Conclusion

**For short-term testing (next 2-4 weeks):** Implement **Option 4** with Docker Compose. It validates the architecture with minimal effort and provides clear metrics for the next phase.

**For long-term production (MicroVM deployment):** Implement **Option 1** with each worker having its own Flawless instance. This provides the best isolation, scalability, and aligns with the etcd-style architecture that mvm-ci is designed for.

The codebase is 90% ready. The missing piece is deciding on the deployment model for Flawless servers, not the worker architecture itself.

