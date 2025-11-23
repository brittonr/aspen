# Flawless Setup and Startup Analysis - mvm-ci Control Plane

## Executive Summary

Flawless is a WASM-based workflow execution framework that runs as a **standalone server process** in the control plane containers. The control plane connects to it via HTTP on localhost port 27288. Each control plane node has its own Flawless instance, and workers connect to the same Flawless server to execute jobs.

---

## How Flawless is Started

### In Docker Containers (Primary Setup)

**File:** `/home/brittonr/git/mvm-ci/docker-entrypoint.sh`

```bash
# Line 22-27
echo "Starting flawless server..."
cd /data
flawless up > /var/log/flawless.log 2>&1 &
FLAWLESS_PID=$!
echo "✓ Flawless started (PID: $FLAWLESS_PID)"
```

**Startup Sequence:**
1. **Command:** `flawless up`
2. **Working Directory:** `/data` (shared volume mount)
3. **Output Redirection:** Logs to `/var/log/flawless.log`
4. **Background Process:** Runs as background daemon with `&`
5. **Timing:** Started BEFORE the control plane (`mvm-ci` binary)

**Startup Order in Container:**
1. Create directories: `/data/hiqlite`, `/data/iroh`, `/var/log`, `/tmp`
2. Generate hiqlite config from template
3. Start Flawless (background)
4. Sleep 3 seconds (grace period for Flawless to start)
5. Start mvm-ci control plane (main process)

### In Development/Testing

```bash
# TESTING.md examples
flawless up > /tmp/flawless-single.log 2>&1 &
```

Identical command, just logs to a different location.

---

## Flawless Configuration

### Location and Access

- **Binary Source:** Nix flake downloads from `https://downloads.flawless.dev/1.0.0-beta.3/x64-linux/flawless`
- **Binary Version:** 1.0.0-beta.3
- **Container Location:** `/bin/flawless` (added by Nix layers)
- **Included In:** All container layers via Nix `dockerTools.buildLayeredImage`

### Configuration Details

**Flawless does NOT require explicit configuration files.** The `flawless up` command:
- Uses default configuration
- Binds to `localhost:27288` (hardcoded/default)
- Creates data directories as needed
- Auto-initializes with no setup required

**Key Behavior:**
- Single-process, single-machine server
- No distributed configuration needed
- Listens only on localhost (NOT accessible from outside container by default)

---

## Port Binding

### Flawless Server Port

| Property | Value |
|----------|-------|
| **Port** | `27288` |
| **Interface** | `localhost` / `127.0.0.1` (local only) |
| **Protocol** | HTTP |
| **Access Scope** | Container-local only |
| **Connectable From** | Same container only |

### In Docker Compose

**Port Exposure:** Flawless port is NOT exposed externally in `docker-compose.yml`
- Only these ports are mapped:
  - `3020:3020` - mvm-ci HTTP server (control plane API)
  - `9000:9000` - Hiqlite Raft consensus
  - `9001:9001` - Hiqlite API

Flawless port `27288` is not exposed because:
- Workers must be in the same container or on the same network
- Workflows (WASM) run inside Flawless process
- No external access needed

---

## How the Control Plane Connects

### Connection Method

**File:** `/home/brittonr/git/mvm-ci/src/main.rs` (lines 42-49)

```rust
// Each node can have its own flawless server
let flawless_url = std::env::var("FLAWLESS_URL")
    .unwrap_or_else(|_| "http://localhost:27288".to_string());

tracing::info!("Connecting to flawless server at {}", flawless_url);
let flawless = flawless_utils::Server::new(&flawless_url, None);
let flawless_module = flawless_utils::load_module_from_build!("module1");
let module = flawless.deploy(flawless_module).await.unwrap();
```

### Connection Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| **URL** | `http://localhost:27288` | Default, configurable via env |
| **Environment Variable** | `FLAWLESS_URL` | Set in docker-compose.yml |
| **HTTP Method** | HTTP/REST | `flawless_utils::Server::new()` |
| **TLS** | None | Plain HTTP to localhost |
| **Auth** | None | Localhost trusted boundary |

### In Docker Compose

**File:** `/home/brittonr/git/mvm-ci/docker-compose.yml` (lines 14, 37, 60)

```yaml
environment:
  - FLAWLESS_URL=http://localhost:27288
```

All three nodes use the SAME environment variable, pointing to localhost.

**Important:** Each container has its own Flawless instance because:
- `localhost` refers to the container's loopback interface
- Node1 container talks to Node1's Flawless
- Node2 container talks to Node2's Flawless
- Node3 container talks to Node3's Flawless

---

## Module Deployment

### Workflow Module Deployment

**File:** `/home/brittonr/git/mvm-ci/src/main.rs` (lines 48-49)

```rust
let flawless_module = flawless_utils::load_module_from_build!("module1");
let module = flawless.deploy(flawless_module).await.unwrap();
```

**Module Path:** `workflows/module1/` (Rust + WASM)

**Deployment Flow:**
1. Load compiled WASM module from build artifacts
2. Deploy to Flawless server via HTTP
3. Get `DeployedModule` handle for execution
4. Module stays deployed for life of control plane process

### Module Execution

When a job is executed by the control plane (or worker):

```rust
// From src/worker_flawless.rs
self.module
    .start::<module1::start_crawler>(module1::Job { id, url })
    .await
```

- Workflow is invoked within Flawless
- WASM runtime handles execution
- Result returned to caller

---

## Worker Connection Model

### How Workers Connect to Flawless

**File:** `/home/brittonr/git/mvm-ci/src/bin/worker.rs` (lines 22-32)

```rust
let flawless_url = std::env::var("FLAWLESS_URL")
    .unwrap_or_else(|_| "http://localhost:27288".to_string());

let worker = FlawlessWorker::new(&flawless_url).await?;
worker.initialize().await?;
```

**Worker Flawless Backend:** `/home/brittonr/git/mvm-ci/src/worker_flawless.rs` (lines 22-35)

```rust
pub async fn new(flawless_url: &str) -> Result<Self> {
    tracing::info!(flawless_url = %flawless_url, "Connecting to Flawless server");
    
    let flawless = Server::new(flawless_url, None);
    let flawless_module = flawless_utils::load_module_from_build!("module1");
    let module = flawless
        .deploy(flawless_module)
        .await
        .map_err(|e| anyhow!("Failed to deploy Flawless module: {}", e))?;
    
    tracing::info!("Flawless module deployed successfully");
    Ok(Self { module })
}
```

### Worker Startup Flow

1. Worker connects to **control plane** via iroh+h3 ticket (not Flawless directly)
2. Worker gets jobs from control plane via work queue API
3. Worker independently connects to **Flawless server** (same port 27288)
4. Worker executes job via deployed module

**Key Point:** Workers need their own Flawless connection but use the same port.

---

## Network Architecture

### Single Container Model

```
[Container mvm-ci-node1]
├── Flawless Server (localhost:27288)
├── mvm-ci Control Plane (0.0.0.0:3020)
└── Hiqlite (0.0.0.0:9000, 9001)
    ├── Can call Flawless ✓
    └── Port 27288 not exposed outside container ✓
```

### Multi-Node Cluster Model

```
[Container node1]               [Container node2]               [Container node3]
├── Flawless (localhost:27288)  ├── Flawless (localhost:27288)  ├── Flawless (localhost:27288)
├── mvm-ci (0.0.0.0:3020)       ├── mvm-ci (0.0.0.0:3020)       ├── mvm-ci (0.0.0.0:3020)
├── Hiqlite (Raft cluster)      ├── Hiqlite (Raft cluster)      ├── Hiqlite (Raft cluster)
└── iroh (P2P)                  └── iroh (P2P)                  └── iroh (P2P)
         │
         └─────── Work Queue Replication via Raft ───────┘

Each node has independent Flawless + mvm-ci processes
Work queue is replicated across all nodes
```

---

## Environment Variables Summary

### For Control Plane

| Variable | Default | Purpose | File |
|----------|---------|---------|------|
| `FLAWLESS_URL` | `http://localhost:27288` | Flawless server URL | `src/main.rs:44` |
| `HTTP_PORT` | `3020` | mvm-ci HTTP server port | `src/main.rs:129` |
| `IROH_BLOBS_PATH` | `./data/iroh-blobs` | Iroh blob storage | `src/main.rs:81` |
| `RUST_LOG` | `(not set)` | Log level | `docker-compose.yml` |

### For Workers

| Variable | Default | Purpose | File |
|----------|---------|---------|------|
| `FLAWLESS_URL` | `http://localhost:27288` | Flawless server URL | `src/bin/worker.rs:22` |
| `CONTROL_PLANE_TICKET` | **(required)** | iroh+h3 ticket to control plane | `src/bin/worker.rs:18` |

---

## Startup Timeline (Docker Container)

```
Time    Component                   Action
────────────────────────────────────────────────────
T+0     docker-entrypoint.sh        Begin execution
T+1     mkdir                       Create directories
T+2     envsubst                    Generate hiqlite.toml
T+3     flawless up                 Start Flawless (background)
T+3.5   (Flawless initializing)     Loading WASM runtime, binding port 27288
T+4     sleep 3                     Wait for Flawless startup
T+7     mvm-ci                      Start control plane (foreground/main)
T+7.5   (mvm-ci initializing)       Load Flawless module, init iroh, setup HTTP servers
T+8     (Ready)                     API accepting requests on 3020
```

---

## Critical Dependencies

### Flawless Binary in Container

**From flake.nix:**
```nix
flawless = pkgs.stdenv.mkDerivation {
  pname = "flawless";
  version = "1.0.0-beta.3";
  src = pkgs.fetchurl {
    url = "https://downloads.flawless.dev/1.0.0-beta.3/x64-linux/flawless";
    sha256 = "0p11baphc2s8rjhzn9v2sai52gvbn33y1xlqg2yais6dmf5mj4dm";
  };
  # ... auto-patching and installation
};
```

**Container Image Inclusion:**
```nix
contents = [
  mvm-ci
  flawless          # ← Added to image
  pkgs.bash
  pkgs.coreutils
  pkgs.gettext      # For envsubst in docker-entrypoint.sh
  pkgs.cacert
  # ...
];
```

---

## Potential Issues for Workers

### Problem: Workers Can't Reach Flawless

If a worker is outside the container or on a different host:
- Connecting to `localhost:27288` won't work
- Solution: Need to expose Flawless or use a shared Flawless instance

### Problem: Multiple Workers Same Instance

Currently:
- Each container has one Flawless
- Multiple workers in same container can connect to same Flawless
- Workers in different containers have different Flawless instances

### Current Limitation

From TESTING.md:
```bash
# All nodes must use the SAME flawless_url
FLAWLESS_URL=http://localhost:27288  # This is per-container localhost
```

Workers outside containers would need:
- Exposed Flawless port (requires network bridge)
- DNS/IP address instead of localhost
- OR shared Flawless instance accessible from all workers

---

## Summary Table

| Aspect | Details |
|--------|---------|
| **Start Command** | `flawless up` |
| **Working Dir** | `/data` |
| **Port** | `27288` |
| **Interface** | `localhost` (local only) |
| **Config** | None required (defaults used) |
| **Binary Location** | `/bin/flawless` |
| **Log Location** | `/var/log/flawless.log` |
| **Module** | Deployed via `flawless_utils::Server::new()` |
| **Startup Order** | Before mvm-ci binary |
| **Container Startup** | ~4 seconds before API ready |
| **Environment Var** | `FLAWLESS_URL=http://localhost:27288` |
| **Per-Container** | Each container has own Flawless |
| **Exposed Externally** | NO (not in docker-compose.yml) |
| **Worker Access** | Same container only (or needs network access) |

