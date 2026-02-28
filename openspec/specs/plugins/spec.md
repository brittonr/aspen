# Plugins Specification

## Purpose

Extensible plugin system enabling third-party code to run safely within Aspen nodes. Supports three execution backends: Hyperlight micro-VMs (strongest isolation), WASM (portable sandboxing), and RPC (out-of-process via iroh). Plugins interact with the host through a defined ABI exposing KV, blob, and event capabilities.

## Requirements

### Requirement: Plugin Execution Backends

The system SHALL support three plugin execution backends behind the `plugins` feature flag. Each backend SHALL implement a common plugin interface.

#### Scenario: Hyperlight VM plugin

- GIVEN a plugin compiled as a Hyperlight micro-VM guest
- WHEN the plugin is loaded by the host
- THEN it SHALL execute in an isolated micro-VM with its own memory space
- AND it SHALL have no direct access to host memory or filesystem

#### Scenario: WASM plugin

- GIVEN a plugin compiled to WebAssembly
- WHEN the plugin is loaded by the host
- THEN it SHALL execute in a WASM sandbox
- AND it SHALL interact with the host only through imported functions

#### Scenario: RPC plugin

- GIVEN a plugin running as a separate process
- WHEN the host communicates with it
- THEN all communication SHALL occur over iroh QUIC
- AND the plugin SHALL use the same host ABI via RPC serialization

### Requirement: Host ABI

The system SHALL expose a stable Host ABI to plugins, providing controlled access to Aspen's distributed primitives. The ABI SHALL be the only interface between plugins and the host.

#### Scenario: KV read from plugin

- GIVEN a plugin with `kv:read` permission
- WHEN the plugin calls `host_kv_get(key)`
- THEN the host SHALL execute a linearizable read against the Raft cluster
- AND return the value (or null) to the plugin

#### Scenario: KV write from plugin

- GIVEN a plugin with `kv:write` permission
- WHEN the plugin calls `host_kv_put(key, value)`
- THEN the host SHALL propose the write through Raft consensus
- AND return success/failure to the plugin

#### Scenario: Blob storage from plugin

- GIVEN a plugin with `blob:write` permission
- WHEN the plugin calls `host_blob_put(data)`
- THEN the host SHALL store the data as a content-addressed blob (BLAKE3 hash)
- AND return the blob hash to the plugin

#### Scenario: Blob retrieval from plugin

- GIVEN a plugin with `blob:read` permission
- WHEN the plugin calls `host_blob_get(hash)`
- THEN the host SHALL retrieve the blob by its BLAKE3 hash
- AND return the data to the plugin

#### Scenario: Event emission from plugin

- GIVEN a plugin with `events:emit` permission
- WHEN the plugin calls `host_emit_event(topic, payload)`
- THEN the host SHALL broadcast the event to subscribers of that topic

#### Scenario: Log output from plugin

- GIVEN any plugin (no permission required)
- WHEN the plugin calls `host_log(level, message)`
- THEN the host SHALL record the log entry attributed to the plugin

### Requirement: Plugin Isolation and Sandboxing

The system SHALL enforce strict isolation between plugins and between plugins and the host. A misbehaving plugin SHALL NOT affect other plugins or the host node.

#### Scenario: Memory isolation (VM backend)

- GIVEN a Hyperlight VM plugin
- WHEN the plugin attempts to access memory outside its allocation
- THEN the micro-VM SHALL trap and terminate the plugin
- AND the host SHALL remain unaffected

#### Scenario: CPU time limits

- GIVEN a plugin with a configured execution timeout
- WHEN the plugin exceeds its CPU time budget
- THEN the host SHALL terminate the plugin execution
- AND return a timeout error to the caller

#### Scenario: Crash containment

- GIVEN a plugin that panics or crashes
- WHEN the crash occurs
- THEN only that plugin instance SHALL be affected
- AND the host node SHALL continue operating normally
- AND other plugins SHALL continue executing

### Requirement: Plugin Permissions

The system SHALL enforce capability-based permissions for each plugin. Plugins SHALL only access host functions they have been explicitly granted.

#### Scenario: Permission denied

- GIVEN a plugin with only `kv:read` permission
- WHEN the plugin calls `host_kv_put(key, value)`
- THEN the host SHALL reject the call with a permission error
- AND the plugin SHALL receive an error code

#### Scenario: Granular permissions

- GIVEN a plugin manifest declaring permissions `["kv:read", "blob:read", "events:emit"]`
- WHEN the plugin is loaded
- THEN exactly those three capabilities SHALL be available
- AND all other host functions SHALL return permission errors

### Requirement: Plugin Lifecycle

The system SHALL manage plugin lifecycle: loading, initialization, execution, and teardown. Plugins SHALL be registered in the cluster's KV store.

#### Scenario: Plugin registration

- GIVEN a plugin binary and its manifest
- WHEN an administrator registers the plugin
- THEN the plugin metadata SHALL be stored in the cluster's KV store under `plugins/<name>`
- AND the plugin binary SHALL be stored as a content-addressed blob

#### Scenario: Plugin invocation

- GIVEN a registered plugin `"my-transform"`
- WHEN a client or hook triggers the plugin
- THEN the host SHALL load the plugin, create an execution context, and call its entry point
- AND the result SHALL be returned to the caller

#### Scenario: Plugin teardown

- GIVEN a running plugin instance
- WHEN execution completes (or times out)
- THEN all plugin resources (memory, file handles) SHALL be released
- AND the execution context SHALL be destroyed

### Requirement: Plugin Discovery and Distribution

Plugin binaries SHALL be content-addressed (BLAKE3 hashed) and distributable via iroh-blobs. Any node in the cluster SHALL be able to fetch and execute a registered plugin.

#### Scenario: Plugin fetch on demand

- GIVEN plugin `"my-transform"` registered on node 1
- WHEN node 3 needs to execute the plugin but doesn't have the binary locally
- THEN node 3 SHALL fetch the plugin binary via iroh-blobs using its content hash
- AND cache it locally for subsequent invocations

#### Scenario: Plugin integrity verification

- GIVEN a plugin binary fetched from a remote node
- WHEN the binary is received
- THEN the host SHALL verify its BLAKE3 hash matches the registered hash
- AND reject the binary if the hash does not match

### Requirement: Plugin Configuration

Plugins SHALL accept configuration via a structured manifest that declares permissions, resource limits, entry points, and metadata.

#### Scenario: Manifest validation

- GIVEN a plugin manifest with invalid permissions
- WHEN registration is attempted
- THEN the system SHALL reject the registration with a descriptive error

#### Scenario: Resource limit configuration

- GIVEN a plugin manifest declaring `max_memory_bytes: 67108864` (64 MB) and `timeout_ms: 30000`
- WHEN the plugin is loaded
- THEN the execution environment SHALL enforce those limits
