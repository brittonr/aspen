# Aspen Plugin System Architecture

> Planning document for implementing a plugin system that enables heterogeneous clusters with different capabilities to communicate seamlessly.

**Status:** Complete (Phase 1–5 implemented; plugin registry deferred)
**Created:** 2026-02-04
**Last Updated:** 2026-02-23

## Table of Contents

1. [Goals and Requirements](#goals-and-requirements)
2. [Architecture Overview](#architecture-overview)
3. [Plugin Architecture Options](#plugin-architecture-options)
4. [Recommended Approach: Hybrid System](#recommended-approach-hybrid-system)
5. [Capability-Aware Federation](#capability-aware-federation)
6. [Plugin API Design](#plugin-api-design)
7. [WASM Plugin Runtime](#wasm-plugin-runtime)
8. [Memory and Performance](#memory-and-performance)
9. [FoundationDB Layer Concept](#foundationdb-layer-concept)
10. [Prior Art and References](#prior-art-and-references)
11. [Implementation Roadmap](#implementation-roadmap)

---

## Goals and Requirements

### Primary Goals

1. **Modular Deployment**: Different Aspen clusters can run different plugins
   - Cluster A: Core + Forge + CI
   - Cluster B: Core + SNIX
   - Cluster C: Core + Forge + SNIX + CI

2. **Federation Compatibility**: Clusters with different plugins can still communicate
   - Core protocol remains universal
   - Graceful degradation when capability is missing
   - Optional request routing to capable clusters

3. **Plugin Examples**:
   - `aspen-forge`: Decentralized Git hosting
   - `aspen-snix`: Distributed Nix binary cache
   - `aspen-ci`: CI/CD pipeline system
   - Future third-party plugins

### Requirements

| Requirement | Priority | Notes |
|-------------|----------|-------|
| Hot-reload without node restart | High | For WASM plugins |
| Sandboxed execution | High | For untrusted code |
| Near-native performance for core plugins | High | Forge, SNIX |
| Language-agnostic plugins | Medium | Via WASM |
| Cross-cluster capability routing | Medium | Federation feature |
| Plugin dependency management | Medium | Plugin A requires Plugin B |
| Resource limits per plugin | High | Tiger Style compliance |

---

## Architecture Overview

```
                           Aspen Plugin Architecture

┌─────────────────────────────────────────────────────────────────────────┐
│                           Aspen Node                                     │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                      Plugin Registry                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │ │
│  │  │ Native      │  │ WASM        │  │ gRPC        │                 │ │
│  │  │ Plugins     │  │ Plugins     │  │ Plugins     │                 │ │
│  │  │ (compile)   │  │ (wasmtime)  │  │ (subprocess)│                 │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                    Handler Registry                                 │ │
│  │  Dispatches requests to appropriate plugin handlers                 │ │
│  └────────────────────────────────────────────────────────────────────┘ │
│                              │                                           │
│  ┌───────────────────────────┼───────────────────────────────────────┐  │
│  │                           ▼                                        │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐               │  │
│  │  │ KV Store    │  │ Blob Store  │  │ Cluster     │  Core         │  │
│  │  │ (Raft)      │  │ (iroh-blobs)│  │ (gossip)    │  Services     │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘               │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────────────┐ │
│  │                      Iroh Transport                                 │ │
│  │  QUIC + ALPN protocol routing                                       │ │
│  └────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Federation Layer                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │ Cluster A       │  │ Cluster B       │  │ Cluster C       │         │
│  │ [Forge, CI]     │  │ [SNIX]          │  │ [Forge, SNIX]   │         │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
│                              │                                           │
│            Capability Advertisement + Request Routing                    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Plugin Architecture Options

### Option 1: Compile-Time Traits (Current Pattern)

Formalize existing `RequestHandler` pattern with explicit plugin traits.

```rust
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> &PluginManifest;
    async fn init(&mut self, ctx: PluginContext) -> Result<()>;
    async fn ready(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    fn request_handlers(&self) -> Vec<Arc<dyn RequestHandler>>;
    fn protocol_handlers(&self) -> Vec<(AlpnProtocol, Arc<dyn ProtocolHandler>)>;
}
```

| Pros | Cons |
|------|------|
| Zero runtime overhead | Requires recompilation |
| Full type safety | All plugins known at build time |
| Rust ecosystem access | Plugin authors need Rust expertise |
| Simple debugging | No hot-reload |

**Best for:** Core extensions (Forge, SNIX, CI) where performance matters.

### Option 2: WebAssembly Plugins (wasmtime)

Sandboxed plugins compiled to WASM, running in wasmtime.

```wit
package aspen:plugin@0.1.0;

world plugin {
    import kv-store;
    import blob-store;
    import logging;

    export name: func() -> string;
    export handles: func() -> list<string>;
    export handle-request: func(request: list<u8>) -> result<list<u8>, string>;
}
```

| Pros | Cons |
|------|------|
| Strong sandboxing | 1.5-3x performance overhead |
| Hot-reload | Data copying at boundary |
| Language-agnostic | Async limitations (until WASI 0.3) |
| Resource limits | More complex development |

**Best for:** User extensions, third-party plugins, untrusted code.

### Option 3: gRPC/IPC Plugins (HashiCorp Style)

Plugins as separate processes communicating via gRPC.

```protobuf
service Plugin {
    rpc GetInfo(Empty) returns (PluginInfo);
    rpc HandleRequest(RpcRequest) returns (RpcResponse);
    rpc Shutdown(Empty) returns (Empty);
}
```

| Pros | Cons |
|------|------|
| Complete crash isolation | Higher latency (IPC) |
| Language-agnostic | Serialization overhead |
| Independent resource limits | Complex deployment |
| Hot restart | Multiple processes to manage |

**Best for:** Heavy integrations, microservice-style extensions.

### Option 4: Dynamic Loading (C ABI)

Load `.so`/`.dylib` files at runtime using `libloading` + `abi_stable`.

| Pros | Cons |
|------|------|
| Near-native performance | Requires unsafe code |
| Hot reload possible | ABI stability challenges |
| Full Rust ecosystem | Plugin crashes can crash host |

**Best for:** Trusted first-party plugins needing maximum performance.

---

## Recommended Approach: Hybrid System

Based on analysis, we recommend a **three-tier hybrid approach**:

### Tier 1: Compile-Time Plugins (Core Extensions)

For tightly integrated features developed by the Aspen team:

- `aspen-forge`
- `aspen-snix`
- `aspen-ci`
- `aspen-secrets`

These have direct access to iroh, optimal performance, and ship with official builds.

### Tier 2: WASM Plugins (User Extensions)

For sandboxed, hot-reloadable extensions:

- User-defined functions (validators, transformers)
- Third-party integrations
- Custom request handlers

### Tier 3: gRPC Plugins (Optional, Heavy Integrations)

For heavyweight external integrations:

- External CI systems
- Monitoring integrations
- Database connectors

### Decision Matrix

| Plugin Type | Performance | Safety | Hot-Reload | Use When |
|-------------|-------------|--------|------------|----------|
| Compile-time | Excellent | N/A (trusted) | No | Core features |
| WASM | Good | Excellent | Yes | Third-party, UDFs |
| gRPC | Fair | Excellent | Yes | External services |

---

## Capability-Aware Federation

### The Challenge

Different clusters have different plugins, but need to communicate:

```
┌─────────────────────┐     ┌─────────────────────┐
│   Cluster A         │     │   Cluster B         │
│   [Core, Forge]     │◄───►│   [Core, SNIX]      │
└─────────────────────┘     └─────────────────────┘
        │                           │
        └───── Both understand Core protocol
               Forge requests fail on B (no Forge)
               SNIX requests fail on A (no SNIX)
```

### Solution: Capability Advertisement

Each node advertises capabilities via gossip/federation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterCapabilities {
    /// Core protocol version (always present)
    pub core_version: Version,

    /// Loaded plugins with their versions
    pub plugins: BTreeMap<PluginId, PluginCapability>,

    /// Timestamp for cache invalidation
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapability {
    pub name: String,
    pub version: Version,
    /// Request types this plugin handles
    pub handles: Vec<String>,  // e.g., ["ForgeCreateRepo", "ForgeGetRef"]
    /// Whether this cluster can serve requests
    pub can_serve: bool,
}
```

### Request Classification

Requests declare their required capability:

```rust
impl ClientRpcRequest {
    pub fn required_capability(&self) -> Option<PluginId> {
        match self {
            // Core requests - no special capability
            Self::Ping | Self::KvRead { .. } => None,

            // Plugin requests
            Self::ForgeCreateRepo { .. } => Some(PluginId::new("forge")),
            Self::SnixGetPathInfo { .. } => Some(PluginId::new("snix")),
            Self::CiTriggerPipeline { .. } => Some(PluginId::new("ci")),
        }
    }
}
```

### Graceful Degradation

When capability is missing, return helpful response:

```rust
pub enum ClientRpcResponse {
    // ... existing variants ...

    /// Capability not available on this cluster
    CapabilityUnavailable {
        capability: PluginId,
        available_at: Vec<ClusterHint>,  // Clusters that have it
        message: String,
    },
}
```

### Optional Cross-Cluster Proxying

Clusters can optionally proxy requests to capable peers:

```rust
impl HandlerRegistry {
    pub async fn dispatch(&self, request: ClientRpcRequest, ctx: &Context) -> Result<Response> {
        // Try local handlers first
        for handler in &self.handlers {
            if handler.can_handle(&request) {
                return handler.handle(request, ctx).await;
            }
        }

        // No local handler - check if we can proxy
        if let Some(cap) = request.required_capability() {
            if ctx.config.proxy_to_capable_clusters {
                return self.proxy_to_capable_cluster(request, cap, ctx).await;
            }

            // Return graceful error with hints
            let hints = self.find_capable_clusters(cap, ctx).await;
            return Ok(ClientRpcResponse::CapabilityUnavailable {
                capability: cap,
                available_at: hints,
                message: format!("Plugin '{}' not loaded", cap),
            });
        }

        Err(anyhow!("No handler for request"))
    }
}
```

---

## Plugin API Design

### Core Crate: `aspen-plugin-api`

```rust
//! aspen-plugin-api/src/lib.rs

use std::sync::Arc;
use async_trait::async_trait;

/// Unique plugin identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub String);

/// Plugin metadata and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: PluginId,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub min_api_version: Version,

    /// Required core services
    pub required_services: Vec<RequiredService>,

    /// Request types this plugin handles
    pub handles_requests: Vec<RequestPattern>,

    /// Custom ALPN protocols
    pub alpn_protocols: Vec<AlpnProtocol>,

    /// KV prefixes used (for namespace isolation)
    pub kv_prefixes: Vec<KvPrefix>,

    /// Dependencies on other plugins
    pub dependencies: Vec<PluginDependency>,

    /// Configuration schema (JSON Schema)
    pub config_schema: Option<serde_json::Value>,
}

/// Services a plugin can request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequiredService {
    KeyValue { read: bool, write: bool },
    BlobStore { read: bool, write: bool },
    RaftPropose,
    ClusterInfo,
    Network,
    BackgroundTasks,
}

/// Main plugin trait
#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> &PluginManifest;

    async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError>;
    async fn ready(&mut self) -> Result<(), PluginError>;
    async fn shutdown(&mut self) -> Result<(), PluginError>;

    fn health(&self) -> HealthStatus { HealthStatus::Healthy }

    fn request_handlers(&self) -> Vec<Arc<dyn RequestHandler>> { vec![] }
    fn protocol_handlers(&self) -> Vec<(AlpnProtocol, Arc<dyn ProtocolHandler>)> { vec![] }
    fn event_subscriptions(&self) -> Vec<EventSubscription> { vec![] }
}
```

### Plugin Context (Dependency Injection)

```rust
/// Context provided to plugins
#[derive(Clone)]
pub struct PluginContext {
    pub plugin_id: PluginId,
    pub node_id: u64,

    /// Namespaced KV store (automatically prefixed)
    pub kv: NamespacedKvStore,

    /// Blob store (if requested)
    pub blobs: Option<Arc<dyn BlobStore>>,

    /// Cluster information
    pub cluster: Arc<dyn ClusterInfo>,

    /// Event bus for inter-plugin communication
    pub events: Arc<dyn EventBus>,

    /// Task spawner for background work
    pub tasks: Arc<dyn TaskSpawner>,

    /// Plugin configuration
    pub config: PluginConfig,

    /// Scoped logger
    pub logger: PluginLogger,

    /// Scoped metrics
    pub metrics: PluginMetrics,
}

/// Namespaced KV prevents plugins from clobbering each other
pub struct NamespacedKvStore {
    inner: Arc<dyn KeyValueStore>,
    prefix: String,  // e.g., "/plugins/forge/"
}

impl NamespacedKvStore {
    pub async fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.inner.read(&self.prefixed_key(key)).await
    }

    pub async fn write(&self, key: &[u8], value: Vec<u8>) -> Result<()> {
        self.inner.write(&self.prefixed_key(key), value).await
    }
}
```

### Example: Forge as Plugin

```rust
//! aspen-forge/src/plugin.rs

pub struct ForgePlugin {
    manifest: PluginManifest,
    ctx: Option<PluginContext>,
    forge_node: Option<Arc<ForgeNode>>,
    config: ForgeConfig,
}

impl ForgePlugin {
    pub fn new(config: ForgeConfig) -> Self {
        Self {
            manifest: Self::build_manifest(),
            ctx: None,
            forge_node: None,
            config,
        }
    }

    fn build_manifest() -> PluginManifest {
        PluginManifest {
            id: PluginId::new("forge"),
            name: "Aspen Forge".into(),
            version: Version::new(0, 1, 0),
            description: "Decentralized Git hosting".into(),

            required_services: vec![
                RequiredService::KeyValue { read: true, write: true },
                RequiredService::BlobStore { read: true, write: true },
                RequiredService::RaftPropose,
                RequiredService::BackgroundTasks,
            ],

            handles_requests: vec![
                RequestPattern("ForgeCreateRepo".into()),
                RequestPattern("ForgeGetRef".into()),
                RequestPattern("ForgeSetRef".into()),
                // ... more
            ],

            alpn_protocols: vec![
                AlpnProtocol::new("aspen-git-upload-pack/1"),
                AlpnProtocol::new("aspen-git-receive-pack/1"),
            ],

            kv_prefixes: vec![
                KvPrefix { prefix: "/forge/repos/".into(), description: "Repository metadata".into() },
                KvPrefix { prefix: "/forge/refs/".into(), description: "Git references".into() },
            ],

            dependencies: vec![],
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "default_branch": { "type": "string", "default": "main" },
                    "max_repo_size_bytes": { "type": "integer" }
                }
            })),
            ..Default::default()
        }
    }
}

#[async_trait]
impl Plugin for ForgePlugin {
    fn manifest(&self) -> &PluginManifest { &self.manifest }

    async fn init(&mut self, ctx: PluginContext) -> Result<(), PluginError> {
        let blobs = ctx.blobs.as_ref().ok_or(PluginError::MissingService("blob_store"))?;

        // Create Forge components
        let git_blobs = GitBlobStore::new(blobs.clone());
        let ref_store = RefStore::new(ctx.kv.clone());

        self.forge_node = Some(Arc::new(ForgeNode::new(git_blobs, ref_store, self.config.clone())));
        self.ctx = Some(ctx);

        Ok(())
    }

    async fn ready(&mut self) -> Result<(), PluginError> {
        let ctx = self.ctx.as_ref().unwrap();
        let forge = self.forge_node.as_ref().unwrap();

        // Start background GC task
        ctx.tasks.spawn("forge-gc", {
            let forge = forge.clone();
            async move { forge.run_gc_loop().await }
        });

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        if let Some(forge) = &self.forge_node {
            forge.shutdown().await;
        }
        Ok(())
    }

    fn request_handlers(&self) -> Vec<Arc<dyn RequestHandler>> {
        let forge = self.forge_node.as_ref().unwrap().clone();
        vec![
            Arc::new(ForgeRepoHandler::new(forge.clone())),
            Arc::new(ForgeRefHandler::new(forge.clone())),
            Arc::new(ForgeObjectHandler::new(forge)),
        ]
    }

    fn protocol_handlers(&self) -> Vec<(AlpnProtocol, Arc<dyn ProtocolHandler>)> {
        let forge = self.forge_node.as_ref().unwrap().clone();
        vec![
            (AlpnProtocol::new("aspen-git-upload-pack/1"), Arc::new(GitUploadPackHandler::new(forge.clone()))),
            (AlpnProtocol::new("aspen-git-receive-pack/1"), Arc::new(GitReceivePackHandler::new(forge))),
        ]
    }
}
```

---

## WASM Plugin Runtime

### Compatibility Assessment

| Aspect | Status | Notes |
|--------|--------|-------|
| wasmtime + Aspen deps | Compatible | No conflicts |
| tokio async | Compatible | `Config::async_support(true)` |
| Component Model | Available | wasmtime 41+ |
| iroh in WASM | Not directly | No UDP in WASI; use host functions |

### WIT Interface Definition

```wit
// wit/aspen-plugin.wit
package aspen:plugin@0.1.0;

// ============ Core Storage ============
interface kv-store {
    get: func(key: string) -> option<list<u8>>;
    put: func(key: string, value: list<u8>) -> result<_, string>;
    delete: func(key: string) -> result<_, string>;
    scan: func(prefix: string, limit: u32) -> list<tuple<string, list<u8>>>;
    batch: func(ops: list<kv-op>) -> result<_, string>;
}

interface blob-store {
    get: func(hash: list<u8>) -> option<list<u8>>;
    put: func(data: list<u8>) -> list<u8>;
    has: func(hash: list<u8>) -> bool;
    size: func(hash: list<u8>) -> option<u64>;

    // Streaming for large blobs
    open-reader: func(hash: list<u8>) -> option<blob-reader>;
    open-writer: func() -> blob-writer;
}

resource blob-reader {
    read: func(max-size: u32) -> option<list<u8>>;
    remaining: func() -> u64;
}

resource blob-writer {
    write: func(data: list<u8>) -> result<_, string>;
    finish: func() -> list<u8>;  // Returns hash
    abort: func();
}

// ============ Protocol Handling ============
interface protocol {
    resource stream-pair {
        read: func(max-size: u32) -> option<list<u8>>;
        write: func(data: list<u8>) -> result<_, string>;
        close: func();
    }
}

// ============ Background Tasks ============
interface scheduler {
    schedule-periodic: func(name: string, interval-ms: u64);
    schedule-once: func(name: string, delay-ms: u64);
    cancel: func(name: string);
}

interface events {
    watch-prefix: func(prefix: string) -> subscription-id;
    unwatch: func(id: subscription-id);
}

// ============ Plugin World ============
world plugin {
    import kv-store;
    import blob-store;
    import protocol;
    import scheduler;
    import events;
    import logging;

    export name: func() -> string;
    export version: func() -> string;
    export handles: func() -> list<string>;
    export alpn-protocols: func() -> list<string>;

    export init: func(config: list<u8>) -> result<_, string>;
    export handle-request: func(request: list<u8>) -> result<list<u8>, string>;
    export handle-protocol: func(alpn: string, stream: protocol.stream-pair) -> result<_, string>;
    export on-scheduled: func(name: string);
    export on-event: func(event: kv-event);
    export shutdown: func();
}
```

### Host Implementation

```rust
//! aspen-wasm-runtime/src/lib.rs

use wasmtime::component::*;
use wasmtime_wasi::WasiCtxBuilder;

pub struct WasmPluginEngine {
    engine: Engine,
    linker: Linker<PluginState>,
}

impl WasmPluginEngine {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.async_support(true);
        config.consume_fuel(true);  // CPU limiting
        config.wasm_component_model(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);

        // Add WASI (limited subset)
        wasmtime_wasi::add_to_linker_async(&mut linker)?;

        // Add Aspen host functions
        Self::add_kv_functions(&mut linker)?;
        Self::add_blob_functions(&mut linker)?;
        Self::add_logging_functions(&mut linker)?;

        Ok(Self { engine, linker })
    }

    fn add_kv_functions(linker: &mut Linker<PluginState>) -> Result<()> {
        linker.func_wrap_async("kv-store", "get", |caller: Caller<'_, PluginState>, key: String| {
            Box::new(async move {
                let state = caller.data();
                state.kv.read(key.as_bytes()).await.ok().flatten()
            })
        })?;

        linker.func_wrap_async("kv-store", "put", |caller: Caller<'_, PluginState>, key: String, value: Vec<u8>| {
            Box::new(async move {
                let state = caller.data();
                state.kv.write(key.as_bytes(), value).await
                    .map_err(|e| e.to_string())
            })
        })?;

        Ok(())
    }
}

pub struct PluginState {
    pub kv: NamespacedKvStore,
    pub blobs: Option<Arc<dyn BlobStore>>,
    pub fuel_limit: u64,
}
```

### WASM Plugin Example (Forge)

```rust
//! forge-wasm-plugin/src/lib.rs
//! Compiled to wasm32-wasip2

wit_bindgen::generate!("aspen-plugin");

struct ForgePlugin;

impl Guest for ForgePlugin {
    fn name() -> String { "forge".into() }
    fn version() -> String { "0.1.0".into() }

    fn handles() -> Vec<String> {
        vec![
            "ForgeCreateRepo".into(),
            "ForgeGetRef".into(),
            "ForgeSetRef".into(),
        ]
    }

    fn alpn_protocols() -> Vec<String> {
        vec![
            "aspen-git-upload-pack/1".into(),
            "aspen-git-receive-pack/1".into(),
        ]
    }

    fn handle_request(request: Vec<u8>) -> Result<Vec<u8>, String> {
        let req: ForgeRequest = postcard::from_bytes(&request)
            .map_err(|e| e.to_string())?;

        match req {
            ForgeRequest::GetRef { repo, ref_name } => {
                let key = format!("/refs/{}/{}", repo, ref_name);
                let value = kv_store::get(&key);
                // ...
            }
            ForgeRequest::PutObject { data } => {
                // Use streaming for large objects
                let mut writer = blob_store::open_writer();
                for chunk in data.chunks(64 * 1024) {
                    writer.write(chunk)?;
                }
                let hash = writer.finish();
                // ...
            }
        }
    }

    fn handle_protocol(alpn: String, stream: StreamPair) -> Result<(), String> {
        match alpn.as_str() {
            "aspen-git-upload-pack/1" => git_upload_pack(stream),
            "aspen-git-receive-pack/1" => git_receive_pack(stream),
            _ => Err("Unknown protocol".into())
        }
    }
}
```

---

## Memory and Performance

### Zero-Copy Considerations

| Approach | Zero-Copy? | Status |
|----------|------------|--------|
| Core WASM linear memory | Yes (unsafe) | Available now |
| Component Model (WIT) | No | Copies during lift/lower |
| Component Model streams | Reduced | Chunked transfer |
| Shared memory proposal | Yes | Future (proposal stage) |

### Recommended Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                        Plugin Data Transfer                      │
│                                                                  │
│   ┌─────────────────────────────────────┐                       │
│   │     Component Model (WIT)            │                       │
│   │     - Type-safe API                  │  Use for: metadata,  │
│   │     - KV ops, small data (<64KB)     │  requests, configs   │
│   └─────────────────────────────────────┘                       │
│                                                                  │
│   ┌─────────────────────────────────────┐                       │
│   │     Streaming Interface              │                       │
│   │     - blob-reader, blob-writer       │  Use for: blobs,     │
│   │     - Chunked (64KB-1MB)             │  NARs, packfiles     │
│   └─────────────────────────────────────┘                       │
│                                                                  │
│   ┌─────────────────────────────────────┐                       │
│   │     Direct Memory (opt-in)           │                       │
│   │     - bulk_* functions               │  Use for: extreme    │
│   │     - Zero-copy, unsafe              │  performance needs   │
│   └─────────────────────────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

### Performance Estimates

| Operation | Native | WASM + WIT | WASM + Stream | WASM + Direct |
|-----------|--------|------------|---------------|---------------|
| KV read 1KB | ~100us | ~150us | N/A | ~100us |
| KV write 1KB | ~2ms | ~2.1ms | N/A | ~2ms |
| Blob read 1MB | ~5ms | ~15ms | ~8ms | ~5ms |
| Blob read 100MB | ~500ms | ~1.5s | ~800ms | ~500ms |

For Raft-based operations, overhead is negligible (consensus dominates).

### Resource Limits (Tiger Style)

```rust
fn create_plugin_store(&self, ctx: &PluginContext) -> Store<PluginState> {
    let mut store = Store::new(&self.engine, PluginState { ... });

    // CPU: fuel-based limiting
    store.set_fuel(100_000).unwrap();  // ~100k instructions per request

    // Memory: wasmtime's built-in limits
    // (configured via MemoryType or ResourceLimiter)

    // Wall-clock: epoch interruption
    store.epoch_deadline_trap();

    store
}
```

---

## FoundationDB Layer Concept

FoundationDB's architecture provides strong validation for Aspen's plugin approach. FDB pioneered the "unbundled database" concept where the core provides a minimal, robust foundation and higher-level features are implemented as stateless "layers" on top.

### FDB Architecture Philosophy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        FoundationDB Architecture                             │
│                                                                              │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                         Layer Applications                          │    │
│   │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌────────────┐ │    │
│   │  │ Record Layer │ │ Document DB  │ │ Time Series  │ │ Your App   │ │    │
│   │  │ (SQL-like)   │ │              │ │              │ │            │ │    │
│   │  └──────────────┘ └──────────────┘ └──────────────┘ └────────────┘ │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                    │                                         │
│                                    ▼                                         │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                    Layer Interface (Stateless)                      │    │
│   │  - All state lives in FDB                                           │    │
│   │  - Layers are pure transformations                                  │    │
│   │  - Key prefix isolation (/record/, /document/, etc.)               │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                    │                                         │
│                                    ▼                                         │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                      FoundationDB Core                              │    │
│   │  - Ordered KV store                                                 │    │
│   │  - ACID transactions                                                │    │
│   │  - Distributed consensus                                            │    │
│   │  - Automatic sharding                                               │    │
│   └────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key FDB Concepts

**Unbundled Database**
: Traditional databases bundle everything (SQL, indexes, replication, transactions) into one monolith. FDB unbundles: the core handles only ordering, transactions, and replication. Everything else is a layer.

**Stateless Layers**
: Layers contain no state of their own. All persistent state goes through FDB's transactional KV store. This means layers can be restarted, scaled, or replaced without data migration.

**Key Prefix Isolation**
: Each layer uses a distinct key prefix (`/record/`, `/document/`, `/timeseries/`). Multiple layers can coexist on the same FDB cluster without interference.

**Extreme Multi-Tenancy**
: Apple runs "thousands of tenants" on shared FDB clusters. Isolation is achieved through key prefixes and transaction limits, not separate clusters.

### Comparison: FDB Layers vs Aspen Plugins

| Aspect | FDB Layers | Aspen Plugins |
|--------|-----------|---------------|
| **Core primitive** | Ordered KV + transactions | Ordered KV (Raft) + blobs (iroh) |
| **State location** | All in FDB | All in Raft KV / iroh-blobs |
| **Isolation** | Key prefix + tenant isolation | Key prefix + namespaced stores |
| **Transport** | FDB client library | Iroh QUIC |
| **Deployment** | Stateless app servers | Plugin in Aspen node or external |
| **Hot reload** | Yes (stateless) | Yes for WASM; restart for native |
| **Multi-tenancy** | First-class (tenant prefixes) | Via namespaced KV stores |

### What Aspen Learns from FDB

**1. Plugins Should Be Stateless**

Like FDB layers, Aspen plugins should store all state through the core primitives:

```rust
// Good: Plugin uses core KV for state
impl Plugin for ForgePlugin {
    async fn init(&mut self, ctx: PluginContext) -> Result<()> {
        // All state in ctx.kv and ctx.blobs
        // Plugin itself is stateless and restartable
        self.ctx = Some(ctx);
        Ok(())
    }
}

// Avoid: Plugin with internal state that would be lost on restart
impl Plugin for BadPlugin {
    async fn init(&mut self, _ctx: PluginContext) -> Result<()> {
        self.internal_cache = HashMap::new();  // Lost on restart!
        Ok(())
    }
}
```

**2. Key Prefix Discipline**

FDB's strict prefix isolation enables multi-tenancy:

```rust
pub struct PluginManifest {
    // Each plugin declares its key prefixes
    pub kv_prefixes: Vec<KvPrefix>,  // e.g., ["/forge/", "/forge-cobs/"]
    // ...
}

// NamespacedKvStore enforces isolation automatically
impl NamespacedKvStore {
    pub fn new(inner: Arc<dyn KeyValueStore>, plugin_id: &PluginId) -> Self {
        Self {
            inner,
            prefix: format!("/plugins/{}/", plugin_id.0),
        }
    }
}
```

**3. Version-Based Change Tracking**

FDB's Record Layer uses "meta-data version stamps" to track schema changes. Aspen can adopt similar patterns:

```rust
pub struct PluginMetadata {
    pub plugin_id: PluginId,
    pub version: Version,
    pub schema_version: u64,
    pub last_migration: u64,
}
```

**4. Composability Over Features**

FDB's philosophy: build simple primitives, let layers compose features. Applied to Aspen:

- Core: KV, blobs, consensus, transport
- Plugin: Combines primitives for domain-specific features
- Don't add domain logic to core just because it's convenient

### Aspen vs FDB: Key Differences

| FDB Approach | Aspen Difference | Rationale |
|--------------|------------------|-----------|
| Transaction-first (all ops in tx) | Raft consensus for linearizable ops | Simpler model, sufficient for our use cases |
| Stateless layer servers | Plugin embedded in node or WASM | Reduces operational complexity |
| Single-cluster focus | Federation between clusters | Different deployment model (edge, multi-region) |
| Language-agnostic (bindings) | Rust-first, WASM for others | Performance and safety |

### Implications for Plugin Architecture

Based on FDB learnings, we should emphasize:

1. **Stateless plugin design**: Document and enforce that plugins must be restartable
2. **Strict key prefix allocation**: Registry prevents prefix conflicts
3. **Schema versioning**: Built-in migration support for plugin data
4. **Transactional semantics**: Plugins should use batch operations for consistency
5. **Clear core/plugin boundary**: Core stays minimal, features go in plugins

---

## Prior Art and References

### Systems Analyzed

| System | Relevance | Key Pattern |
|--------|-----------|-------------|
| [FoundationDB](https://apple.github.io/foundationdb/) | Unbundled database, layer architecture | Stateless layers, key prefix isolation |
| [Apollo Federation](https://www.apollographql.com/docs/federation) | Capability-based routing | Subgraph capabilities, intelligent router |
| [Vector](https://github.com/vectordotdev/vector) | Rust component architecture | Trait-based sources/transforms/sinks |
| [OpenTelemetry Collector](https://opentelemetry.io/docs/collector/architecture/) | Pipeline model | Receivers, processors, exporters |
| [Tremor PDK](https://www.tremor.rs/rfc/accepted/plugin-development-kit/) | Dynamic loading | C ABI, no unloading |
| [Envoy WASM](https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/advanced/wasm) | WASM extensions | Proxy-Wasm ABI |
| [Vitess Operator](https://github.com/planetscale/vitess-operator) | K8s federation | Shared lockserver pattern |
| [RabbitMQ](https://www.rabbitmq.com/docs/plugins) | Plugin enable/disable | Explicit enablement |

### Key Patterns Adopted

1. **Stateless layers** (FDB): Plugins store all state in core KV/blobs
2. **Key prefix isolation** (FDB): Namespaced stores prevent conflicts
3. **Unbundled architecture** (FDB): Core stays minimal, features in plugins
4. **Capability declaration** (Apollo): Plugins declare handled requests
5. **Intelligent routing** (Apollo Router): Route to capable nodes
6. **Trait-based components** (Vector): `Plugin`, `RequestHandler` traits
7. **Configuration-driven** (OTel): TOML config enables plugins
8. **Feature flags** (Vector): Compile-time plugin selection
9. **No unloading** (Tremor): Restart to change plugins
10. **Shared coordination** (Vitess): Raft for capability discovery

---

## Implementation Roadmap

### Phase 1: Plugin Trait Formalization ✅

**Goal:** Formalize existing patterns without breaking changes.

**Tasks:**

- [x] Create `aspen-plugin-api` crate (`crates/aspen-plugin-api/`)
- [x] Define `Plugin`, `PluginManifest`, `PluginContext` types
- [x] Add `PluginRegistry` for lifecycle management (`LivePluginRegistry` in `aspen-wasm-plugin`)
- [x] Migrate `aspen-forge` as reference implementation (30 ops → WASM plugin)
- [x] Update `HandlerRegistry` to use plugin handlers (`ArcSwap` hot-reload, `WasmPluginHandler`)

**Deliverables:**

- `crates/aspen-plugin-api/` ✅
- Forge working as WASM plugin (`crates/aspen-forge-plugin/`) ✅

### Phase 2: Capability Advertisement ✅

**Goal:** Enable heterogeneous cluster federation.

**Tasks:**

- [x] Add `ClusterCapabilities` to federation identity (`AppManifest`, `AppRegistry`, `ClusterAnnouncement`)
- [x] Implement `required_capability()` for requests (`app_id()` on handlers)
- [x] Add `CapabilityUnavailable` response type
- [x] Update gossip to propagate capabilities (federation discovery)
- [x] Add capability hints to error responses

**Deliverables:**

- Clusters advertise their plugins ✅
- Graceful errors with routing hints ✅

### Phase 3: WASM Runtime ✅

**Goal:** Support sandboxed WASM plugins.

**Tasks:**

- [x] Create `aspen-wasm-plugin` crate (uses `hyperlight-wasm` runtime)
- [x] Define host function ABI (23 functions, documented in `HOST_ABI.md`)
- [x] Implement host function bindings (`aspen-wasm-guest-sdk`)
- [x] Add `WasmPluginHandler` to registry
- [x] Support loading from filesystem and blob store
- [x] Add resource limits (execution timeout, memory limits, KV prefix isolation)

**Deliverables:**

- `crates/aspen-wasm-plugin/` ✅
- `crates/aspen-wasm-guest-sdk/` ✅
- Example + production WASM plugins ✅
- Plugin loading from CLI (`plugin install`) ✅

### Phase 4: Cross-Cluster Routing ✅

**Goal:** Transparent request routing to capable clusters.

**Tasks:**

- [x] Add `ProxyConfig` with `is_enabled` and `max_connections`
- [x] Implement proxy logic in `HandlerRegistry` (dispatch tries proxy before `CapabilityUnavailable`)
- [x] Add request tracing for proxied requests (`proxy_hops` on `AuthenticatedRequest`)
- [x] Handle proxy loops (max hops)

**Deliverables:**

- Optional request proxying ✅
- Transparent capability routing ✅
- CLI proxy commands (`proxy start`, `proxy forward`) ✅

### Phase 5: Plugin Ecosystem ✅ (registry deferred)

**Goal:** Make plugin development easy.

**Tasks:**

- [x] Create `cargo-aspen-plugin` scaffolding tool (`crates/cargo-aspen-plugin/`)
- [x] Write plugin development guide (`docs/PLUGIN_DEVELOPMENT.md`)
- [ ] Create plugin registry (optional central index — deferred)
- [x] Add plugin signing/verification (`crates/aspen-plugin-signing/`)
- [x] Add example plugins: kv-counter, audit-logger, scheduled-cleanup
- [x] Add `PluginSignatureInfo` to `PluginManifest`

**Deliverables:**

- Plugin SDK ✅
- Documentation ✅
- Example plugins ✅

### Current Handler Architecture (as of 2026-02-23)

Three dispatch tiers, ordered by priority:

| Tier | Abstraction | Priority | Use Case | Examples |
|------|-------------|----------|----------|----------|
| 1 | `RequestHandler` (native) | 100–299 | Tightly-coupled control plane | cluster-handler, core-essentials (core, lease, watch) |
| 2 | `ServiceExecutor` → `ServiceHandler` | 500–600 | Domain services with typed dispatch | blob, ci, docs, forge, job, secrets |
| 3 | `AspenPlugin` → `WasmPluginHandler` | 900–999 | Sandboxed third-party plugins | forge-plugin, coordination, kv, dns, sql, hooks, secrets-plugin, automerge, service-registry |

**11 handlers migrated to WASM** (~12K lines). **6 handlers use ServiceExecutor**, **2 remain direct RequestHandler**.

Native handlers (all use ServiceExecutor unless noted):

| Handler | Tier | Priority | Reason native |
|---------|------|----------|---------------|
| `aspen-core-essentials-handler` | 1 (RequestHandler) | 100–210 | Raft metrics, leases, watches — tightly coupled to control plane |
| `aspen-cluster-handler` | 1 (RequestHandler) | 120 | Raft membership, snapshots — tightly coupled to control plane |
| `aspen-blob-handler` | 2 (ServiceExecutor) | 520 | iroh-blobs, DHT, replication |
| `aspen-docs-handler` | 2 (ServiceExecutor) | 530 | iroh-docs sync, peer federation |
| `aspen-forge-handler` | 2 (ServiceExecutor) | 540 | Federation + git bridge (15 ops) |
| `aspen-secrets-handler` | 2 (ServiceExecutor) | 580 | PKI/X.509 crypto, Nix cache signing |
| `aspen-ci-handler` | 2 (ServiceExecutor) | 600 | Pipeline orchestration, artifacts, logs |
| `aspen-job-handler` | 2 (ServiceExecutor) | 560 | Distributed job queue, worker coordination |

---

## Configuration

### Node Configuration

```toml
# aspen.toml

[node]
id = 1
data_dir = "/var/lib/aspen"

[cluster]
cookie = "my-cluster-secret"

# Plugin configuration
[plugins]
# Directory for WASM plugins
wasm_dir = "/var/lib/aspen/plugins"

# Enable cross-cluster capability proxying
proxy_to_capable_clusters = false
max_proxy_hops = 2

# Compile-time plugins (feature flags)
[plugins.forge]
enabled = true
default_branch = "main"
max_repo_size_bytes = 1073741824

[plugins.snix]
enabled = true

[plugins.ci]
enabled = false  # Disabled on this cluster

# WASM plugins loaded at runtime
[[plugins.wasm]]
name = "custom-validator"
path = "/var/lib/aspen/plugins/validator.wasm"
config = { strict = true }
```

### Plugin Manifest (for WASM plugins)

```toml
# plugin.toml (packaged with .wasm)

[plugin]
id = "custom-validator"
name = "Custom Request Validator"
version = "1.0.0"
description = "Validates requests against custom rules"
min_api_version = "0.1.0"

[capabilities]
requires = ["kv-read"]
handles = ["*"]  # Intercepts all requests

[config]
schema = "config-schema.json"
```

---

## Open Questions

1. **Plugin versioning**: How to handle breaking changes in plugin API?
2. **Plugin discovery**: Central registry vs. distributed?
3. **Hot-reload semantics**: What happens to in-flight requests?
4. **Inter-plugin dependencies**: How to express and resolve?
5. **Metrics aggregation**: Per-plugin metrics vs. unified?

---

## References

- [Wasmtime Documentation](https://docs.wasmtime.dev/)
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
- [WIT Specification](https://component-model.bytecodealliance.org/)
- [WASI Preview 2](https://github.com/WebAssembly/WASI/blob/main/preview2)
- [Zero-Copy Proposal](https://github.com/WebAssembly/component-model/issues/398)
- [Extism Plugin Framework](https://extism.org/)
- [HashiCorp go-plugin](https://github.com/hashicorp/go-plugin)
