## ADDED Requirements

### Requirement: Capability-rooted node host has a crate boundary

r[molten.node_host.crate_boundary] Molten MUST define `molten-node-host` as the internal owner of the shared error type, capability-rooted node-state authority, and local-store authority. The crate MUST depend on `molten-core` and MUST NOT depend on the root `molten` package, Clap, operator presentation, test harnesses, NixOS validation, or release-policy code.

#### Scenario: Node host opens admitted local authority
- GIVEN an explicit operator-selected state root and valid relative state paths
- WHEN node state or a typed local store opens through `molten-node-host`
- THEN the operation uses capability-rooted filesystem authority and returns the existing public types.

#### Scenario: Forbidden host dependency enters the crate
- GIVEN a `molten-node-host` manifest that adds CLI, presentation, harness, NixOS, release-policy, or root-package dependencies
- WHEN the crate boundary gate runs
- THEN validation MUST deny before the dependency is accepted.

### Requirement: Root node host facades preserve compatibility

r[molten.node_host.facade_compatibility] Molten MUST preserve `molten::error`, `molten::node_state`, and `molten::local_store` as explicit compatibility re-exports of `molten-node-host`. Existing error variants, public types, path validation, namespace layout, constants, and capability behavior MUST remain unchanged.

#### Scenario: Existing root path is used
- GIVEN a caller compiled against an existing root module path
- WHEN it opens and uses node state or a local store
- THEN the caller observes the same type identity and behavior as the new crate path.

#### Scenario: Invalid state locator is used through either path
- GIVEN an absolute, parent-traversing, platform-prefixed, remote, or content-addressed locator
- WHEN either the new crate path or old root path parses it
- THEN both paths MUST deny with the same diagnostic class before filesystem mutation.

### Requirement: Compatibility bridges do not add ambient authority

r[molten.node_host.bridge_authority] Cross-crate compatibility bridges MUST consume already-open capability directories. They MUST NOT accept new ambient filesystem paths, parse CLI arguments, execute processes, contact networks, or make release decisions.

#### Scenario: Root internals derive a store from node state
- GIVEN an already-open `NodeStateRoot`
- WHEN root daemon code requests a ledger, artifact, chunk, or delivery store
- THEN the bridge derives the store from the existing capability without reopening the host path.
