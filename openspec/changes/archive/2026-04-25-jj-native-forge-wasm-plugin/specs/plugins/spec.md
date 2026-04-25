## ADDED Requirements

### Requirement: Protocol-session permissions are host-enforced

The host SHALL deny any protocol-session access to KV prefixes, blob operations, or protocol capabilities that are not declared in the plugin manifest.

#### Scenario: Undeclared storage access is denied

- **WHEN** a protocol-capable plugin attempts to read or write a KV/blob capability that it did not declare
- **THEN** the host denies that access at runtime
- **AND** the denied operation does not mutate host state

#### Scenario: Undeclared protocol capability is denied

- **WHEN** a plugin attempts to open or reuse a protocol-session capability that is not declared for it
- **THEN** the host rejects that access with a permission error
- **AND** the plugin remains confined to its declared capabilities

## MODIFIED Requirements

### Requirement: Plugin Lifecycle

The system SHALL manage plugin lifecycle: loading, initialization, request execution, protocol-session execution, and teardown. Plugins SHALL be registered in the cluster's KV store together with the request families, protocol identifiers, permissions, and resource limits they declare. For protocol-capable plugins, the declared protocol identifier SHALL be the network-visible QUIC transport identifier advertised to clients and admitted by the host. Protocol identifiers claimed by plugins SHALL be unique within a node's active plugin set.

#### Scenario: Plugin registration

- GIVEN a plugin binary and its manifest
- WHEN an administrator registers the plugin
- THEN the plugin metadata SHALL be stored in the cluster's KV store under `plugins/<name>`
- AND the plugin binary SHALL be stored as a content-addressed blob
- AND the manifest SHALL record any declared request families, protocol identifiers, permissions, and resource limits
- AND any declared protocol identifier SHALL be the transport identifier the host advertises and admits for that plugin's protocol sessions

#### Scenario: Plugin invocation

- GIVEN a registered plugin `"my-transform"`
- WHEN a client or hook triggers the plugin
- THEN the host SHALL load the plugin, create an execution context, and call its entry point
- AND the result SHALL be returned to the caller

#### Scenario: Protocol session routing

- GIVEN a registered WASM plugin that declares a repo protocol identifier
- WHEN the host accepts an incoming protocol session for that identifier
- THEN the host SHALL route the bounded session to that plugin
- AND the plugin SHALL execute with its configured permissions and resource limits

#### Scenario: Protocol identifier collision

- GIVEN one active plugin already claims a protocol identifier
- WHEN an administrator registers or loads another plugin that claims the same identifier
- THEN the host SHALL reject the conflicting registration or activation
- AND the existing protocol routing SHALL remain unchanged

#### Scenario: Protocol session exceeds limits

- GIVEN a protocol-capable plugin session has declared size, time, or concurrency limits
- WHEN an incoming session exceeds one of those enforced limits
- THEN the host SHALL terminate or reject the session with a limit error
- AND the host SHALL clean up any per-session resources before returning control

#### Scenario: Plugin teardown

- GIVEN a running plugin instance
- WHEN execution completes (or times out)
- THEN all plugin resources (memory, file handles, protocol-session state) SHALL be released
- AND the execution context SHALL be destroyed
