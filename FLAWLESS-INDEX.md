# Flawless Analysis - Complete Documentation Index

This analysis comprehensively documents how Flawless is set up, started, and connected in the mvm-ci control plane containers. It provides detailed guidance for making Flawless available to external workers.

## Documents Generated

### 1. FLAWLESS-SETUP.md (Main Reference)
**Comprehensive technical analysis with code references**

Contains:
- Executive summary
- Detailed startup procedures (Docker containers, development, testing)
- Configuration details
- Port binding information
- Control plane connection method
- Module deployment process
- Worker connection model
- Network architecture diagrams
- Environment variables reference
- Startup timeline
- Critical dependencies
- Potential issues for workers
- Summary table

**Use this for:** Understanding the complete Flawless setup, troubleshooting connection issues, understanding architecture decisions.

### 2. FLAWLESS-WORKER-GUIDE.md (Implementation Guide)
**Step-by-step solutions for making Flawless available to workers**

Contains:
- Problem statement and visual diagrams
- Four solution options with pros/cons:
  1. Expose Flawless port (simplest)
  2. Use container DNS/IP (better)
  3. Shared Flawless instance (recommended)
  4. Run workers in same container (current pattern)
- Code modifications needed for each option
- Testing procedures
- Configuration notes

**Use this for:** Implementing worker connectivity, choosing the right approach for your deployment, modifying configuration files.

## Quick Navigation

### I need to understand...

**How Flawless is started?**
→ See FLAWLESS-SETUP.md: "How Flawless is Started" section

**What port Flawless listens on?**
→ See FLAWLESS-SETUP.md: "Port Binding" section
→ Quick answer: Port 27288, localhost only

**How the control plane connects to Flawless?**
→ See FLAWLESS-SETUP.md: "How the Control Plane Connects" section
→ Files: src/main.rs (lines 42-49)

**How workers should connect to Flawless?**
→ See FLAWLESS-WORKER-GUIDE.md: "Solution Overview"
→ Files: src/bin/worker.rs (lines 22-32), src/worker_flawless.rs

**What environment variables to set?**
→ See FLAWLESS-SETUP.md: "Environment Variables Summary" section

**Why external workers can't reach Flawless?**
→ See FLAWLESS-WORKER-GUIDE.md: "The Problem"
→ Quick answer: Flawless binds to localhost, not exposed in docker-compose.yml

**How to fix worker connectivity?**
→ See FLAWLESS-WORKER-GUIDE.md: "Solution Overview" with 4 options

**What startup sequence happens in container?**
→ See FLAWLESS-SETUP.md: "Startup Timeline" section

**How much time until API is ready?**
→ See FLAWLESS-SETUP.md: "Startup Timeline"
→ Quick answer: ~8 seconds from container start

---

## Key Findings Summary

| Finding | Value |
|---------|-------|
| **Flawless Command** | `flawless up` |
| **Port** | 27288 |
| **Interface** | localhost (local only) |
| **Configuration** | None required (uses defaults) |
| **Binary Location** | `/bin/flawless` in container |
| **Log Location** | `/var/log/flawless.log` |
| **Startup Order** | Before mvm-ci binary |
| **Per-Container?** | Yes, each container has own Flawless |
| **Exposed Externally?** | No (critical issue for external workers) |
| **Environment Variable** | `FLAWLESS_URL=http://localhost:27288` |
| **Module** | workflows/module1 (WASM) |
| **Connection Method** | HTTP REST via flawless_utils::Server |

---

## Critical Issue for Workers

**Problem:** Flawless runs on `localhost:27288` inside each container. External workers cannot reach it because:
- Port 27288 is NOT exposed in docker-compose.yml
- `localhost` only refers to container's loopback interface
- Workers outside the container have no network path to Flawless

**Solution:** Choose one from FLAWLESS-WORKER-GUIDE.md:
1. Expose port 27288 (simplest)
2. Use container DNS/IP (better)
3. Shared Flawless instance (recommended for production)
4. Run workers in same container (current testing pattern)

---

## Code File References

### Startup & Configuration
- `/home/brittonr/git/mvm-ci/docker-entrypoint.sh` - Container entrypoint, starts Flawless
- `/home/brittonr/git/mvm-ci/flake.nix` - Flawless binary download and Docker image build
- `/home/brittonr/git/mvm-ci/docker-compose.yml` - Container configuration (note: port 27288 not exposed)

### Control Plane Connection
- `/home/brittonr/git/mvm-ci/src/main.rs` (lines 42-49) - Flawless server connection
- `/home/brittonr/git/mvm-ci/src/main.rs` (lines 48-49) - Module deployment

### Worker Connection
- `/home/brittonr/git/mvm-ci/src/bin/worker.rs` (lines 22-32) - Worker FLAWLESS_URL handling
- `/home/brittonr/git/mvm-ci/src/worker_flawless.rs` (lines 22-35) - FlawlessWorker implementation
- `/home/brittonr/git/mvm-ci/src/worker_trait.rs` - WorkerBackend trait definition

### Documentation
- `/home/brittonr/git/mvm-ci/TESTING.md` - Development/testing examples
- `/home/brittonr/git/mvm-ci/CLAUDE.md` - Project overview and Flawless framework reference

---

## Flawless Specifications

**Version:** 1.0.0-beta.3
**Source:** https://downloads.flawless.dev/1.0.0-beta.3/x64-linux/flawless
**SHA256:** 0p11baphc2s8rjhzn9v2sai52gvbn33y1xlqg2yais6dmf5mj4dm

**Framework:** WASM workflow execution
**Module:** workflows/module1 (Rust compiled to WebAssembly)
**Default Port:** 27288
**Default Interface:** localhost (127.0.0.1)

---

## Architecture Summary

### Each Control Plane Container
```
[Container mvm-ci-nodeN]
├── Flawless Server (localhost:27288) - NOT exposed
│   ├── Loads module1.wasm at startup
│   ├── Executes workflows on demand
│   └── Lives for container lifetime
├── mvm-ci Control Plane (0.0.0.0:3020)
│   ├── Connects to local Flawless (http://localhost:27288)
│   ├── Deploys module to Flawless at startup
│   └── Executes jobs or proxies to workers
├── Hiqlite (0.0.0.0:9000, 9001)
│   ├── Work queue storage
│   └── Raft replication to other nodes
└── iroh (P2P networking)
    └── P2P communication between nodes and workers
```

### Multi-Node Cluster (Current)
```
Node1           Node2           Node3
├─ Flawless     ├─ Flawless     ├─ Flawless
├─ mvm-ci       ├─ mvm-ci       ├─ mvm-ci
└─ Hiqlite ←→ Hiqlite ←→ Hiqlite
  (Raft replication of work queue)
```

**Problem:** Workers outside containers can't reach Flawless

### Recommended Architecture (Option 3)
```
Flawless (Shared)
     ↑↓
┌────┴───────────────────────┐
│                             │
Node1 (mvm-ci)          Node2 (mvm-ci)
├─ Control Plane      ├─ Control Plane
└─ Hiqlite           └─ Hiqlite
        ↓                    ↓
   Worker 1            Worker 2
   Worker 3            Worker 4
```

---

## Next Steps

### For Immediate Testing
1. Read FLAWLESS-WORKER-GUIDE.md Option 1 (expose port)
2. Modify docker-compose.yml to add port 27288 mapping
3. Set FLAWLESS_URL in worker environment
4. Test worker connectivity

### For Production
1. Implement Option 3 (shared Flawless instance) from FLAWLESS-WORKER-GUIDE.md
2. Add health check for Flawless startup in docker-entrypoint.sh
3. Configure proper logging and monitoring
4. Document in ops runbook

### For Future Improvements
1. Check if Flawless supports configuration via environment variables
2. Implement automatic service discovery (DNS/Consul)
3. Add Flawless HA/clustering if available
4. Integrate Flawless metrics into monitoring system

---

## Additional Resources

### Flawless Framework
- Documentation: https://flawless.dev/docs/
- For API details, use Context7 MCP tool with "/flawless" or search "flawless framework"

### mvm-ci Project
- CLAUDE.md: Project overview and context
- TESTING.md: Development and testing examples

---

Generated: 2025-11-22
Analysis Depth: Complete with code references and implementation guidance
