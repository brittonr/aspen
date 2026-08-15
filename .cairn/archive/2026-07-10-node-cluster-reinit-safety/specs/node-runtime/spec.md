# Node Runtime

## ADDED Requirements

### Requirement: Node init rejects existing lifecycle state

r[molten.node_runtime.init_lifecycle_collision_guard] Molten MUST deny node initialization when the target explicit state root already contains initialized, running, stopped, or inconsistent node lifecycle evidence, unless a separate explicit reset has removed that state first.

#### Scenario: Empty root can initialize

- GIVEN an explicit node state root with no config, identity receipt, startup receipt, shutdown receipt, or active node lock
- WHEN node initialization runs
- THEN initialization writes fresh config and identity evidence.

#### Scenario: Existing lifecycle state denies reinitialization

- GIVEN an explicit node state root already contains initialized, running, stopped, or inconsistent lifecycle evidence
- WHEN node initialization runs again
- THEN initialization denies before writing replacement lifecycle artifacts.

### Requirement: Cluster init reset is explicit and scoped

r[molten.node_runtime.cluster_init_reset_guard] Molten MUST deny non-force cluster initialization when a cluster manifest already exists or any planned node root already contains node lifecycle evidence, and MUST require explicit force/reset intent before replacing planned node roots.

#### Scenario: Existing cluster manifest denies non-force init

- GIVEN a cluster state root already contains the cluster manifest
- WHEN cluster initialization runs without force
- THEN initialization denies before writing node lifecycle artifacts or replacing the manifest.

#### Scenario: Force resets only planned node roots

- GIVEN a cluster init command names planned nodes and force reset is enabled
- WHEN cluster initialization runs
- THEN only the planned node root directories are removed before fresh node initialization and manifest writing.

### Requirement: Cluster state roots reject ambient path syntax

r[molten.node_runtime.cluster_state_root_guard] Molten MUST reject cluster state roots that are empty, current-directory, or parent-directory syntax before planning node paths or mutating filesystem state.

#### Scenario: Ambient current directory is denied

- GIVEN an operator supplies `.` as the cluster state root
- WHEN cluster planning runs
- THEN planning denies before any node path is derived.

#### Scenario: Parent directory is denied

- GIVEN an operator supplies `..` as the cluster state root
- WHEN cluster planning runs
- THEN planning denies before any node path is derived.
