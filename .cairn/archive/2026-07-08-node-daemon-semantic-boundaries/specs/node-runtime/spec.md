# Node Runtime Delta: Daemon Semantic Boundaries

### Requirement: Node daemon responsibilities are semantically separated
r[molten.node_runtime.modularity.daemon_modules] Node daemon implementation SHOULD be organized into semantic ownership boundaries for config, identity, locks, inbox, ingress, dispatch, supervision, live workflow, receipts, and shell orchestration.

#### Scenario: Node module name reveals responsibility
- GIVEN a node daemon behavior is moved during modularity cleanup
- WHEN reviewers inspect the new file or module name
- THEN the name identifies the daemon responsibility rather than an ordinal shard sequence

#### Scenario: Existing node API remains stable
- GIVEN a documented `molten node` command or compatibility module path
- WHEN daemon internals are reorganized
- THEN the command or path remains available unless a separate compatibility change owns the break

### Requirement: Node daemon decisions have pure cores
r[molten.node_runtime.modularity.pure_daemon_core] Node daemon decisions for locks, duplicate requests, ingress admission, dispatch routing, supervisor policy, and workflow gating SHOULD be expressed as pure functions over typed inputs.

#### Scenario: Duplicate enqueue decision is pure
- GIVEN an existing request ref and a new request input represented in memory
- WHEN the duplicate decision core evaluates the input
- THEN it returns enqueue, replay-prior, deny, or diagnostic output without reading the state root or writing queue files

#### Scenario: Supervisor denial is pure
- GIVEN restart history and supervisor policy represented in memory
- WHEN the supervisor decision core evaluates a failed service
- THEN it returns restart or deny output without reading clocks, sockets, files, or live transport state

### Requirement: Node shell owns IO and transport
r[molten.node_runtime.modularity.shell_boundary] Node daemon shells MUST own state-root filesystem IO, service locks, control sockets, live Iroh sessions, process lifecycle, and receipt file writes.

#### Scenario: Shell executes admitted node plan
- GIVEN a pure node decision returns an admitted plan
- WHEN the node shell executes it
- THEN filesystem, lock, or transport effects occur through the shell and canonical node evidence is recorded

#### Scenario: Denied node plan does not perform IO
- GIVEN a pure node decision returns deny
- WHEN the node shell receives the decision
- THEN no queue write, lock mutation, live send, or operation side effect is performed

### Requirement: Node daemon extraction has positive and negative tests
r[molten.node_runtime.modularity.tests] Node daemon boundary refactors SHOULD include positive and negative tests for the extracted decision or shell boundary.

#### Scenario: Node boundary tests cover pass and deny
- GIVEN a node daemon decision boundary is extracted
- WHEN reviewers inspect the tests
- THEN at least one admitted path and one denied or malformed path are covered
