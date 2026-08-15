## ADDED Requirements

### Requirement: Tests acquire isolated capability-rooted workspaces
r[molten.testing.cap_std_workspace] Molten test and harness shells SHOULD create temporary workspaces through a shared capability-rooted RAII abstraction rather than constructing process-id or counter paths under the process-wide temporary directory. A workspace MUST own its lifetime and MUST NOT pre-delete an ambient path selected only by a predictable name.

#### Scenario: Concurrent workspaces remain isolated
- GIVEN multiple tests create workspaces concurrently
- WHEN each test reads and mutates its own logical state
- THEN each workspace MUST have distinct authority and storage
- AND one test MUST NOT delete or reuse another test's root by name.

#### Scenario: Predictable stale path is not pre-deleted
- GIVEN an unrelated host object has a name matching an old Molten temp naming pattern
- WHEN a new workspace is created
- THEN normal test setup MUST NOT scan for or remove that object by ambient prefix.

### Requirement: Workspace subroots carry narrow test authority
r[molten.testing.cap_std_subroots] Molten test workspaces MUST provide capability-derived logical subroots for the fixture roles they expose, including state, input, output, transport, ledger, cache, and adversarial setup where applicable. The system under test MUST receive only the roots or ports required by its fixture role.

#### Scenario: Store fixture receives a narrow state root
- GIVEN a chunk-store fixture needs local chunk state and an output artifact
- WHEN the test constructs its inputs
- THEN the store MUST receive the chunk-state capability and the output shell MUST receive the output capability
- AND neither MUST receive unrelated adversarial or transport authority.

#### Scenario: Adversarial setup does not leak into production API
- GIVEN a negative fixture uses a setup handle to create a symlink or corrupt a file
- WHEN the production operation is invoked
- THEN it MUST receive only its normal target capability
- AND the setup authority MUST remain confined to test orchestration.

### Requirement: Workspace cleanup is local and explicit
r[molten.testing.cap_std_cleanup] Molten test cleanup MUST target only workspace objects owned by the current fixture. Normal tests MUST NOT scan the process-wide temporary directory or remove entries by a shared name prefix. Selected failure artifacts MAY be retained only through an explicit export or operator-selected preservation mode, and retained host paths MUST remain diagnostic-only.

#### Scenario: Normal completion cleans owned workspace
- GIVEN a test workspace completes without explicit preservation
- WHEN its owner is dropped
- THEN best-effort cleanup MUST target only that workspace and its descendants.

#### Scenario: Abrupt termination does not justify broad deletion
- GIVEN a previous test process may have left temporary state
- WHEN another test starts
- THEN it MUST NOT perform broad prefix cleanup
- AND any residue maintenance MUST require a separate explicit operator policy and authority boundary.

### Requirement: Child-process path conversion is a thin shell bridge
r[molten.testing.cap_std_process_bridge] Molten CLI and multiprocess tests MAY convert an owned workspace subroot to a host path only at the process-spawn boundary when the existing command surface requires a path string. The parent harness MUST confine child roots to owned subroots, and canonical receipts MUST bind logical labels, refs, and observations rather than temporary host paths.

#### Scenario: Child node runs inside an owned subroot
- GIVEN a multiprocess fixture needs to pass `--state-root` to a child
- WHEN the parent shell spawns the child
- THEN the path MUST identify an owned workspace subroot
- AND the fixture's canonical evidence MUST use logical node and artifact identity instead of the host path.

#### Scenario: External child root is rejected
- GIVEN a fixture attempts to spawn a child with a state root outside its owned workspace policy
- WHEN process planning runs
- THEN the harness MUST deny before spawn or classify the root as separately supplied explicit authority.

### Requirement: Converted test helpers have a scoped ambient-temp regression gate
r[molten.testing.cap_std_regression_gate] Molten MUST maintain a syntax-aware blocking gate for converted test-helper scopes that rejects new process-id or counter temporary-root construction, process-wide stale-prefix scans, and broad prefix deletion. The gate MUST include positive prohibited fixtures and negative fixtures for the shared workspace constructor and explicit operator artifact export.

#### Scenario: Hand-rolled temp helper fails validation
- GIVEN a converted suite adds a helper that joins `std::env::temp_dir` with a process id or counter and deletes any existing path
- WHEN the test-authority gate runs
- THEN the gate MUST fail with an ambient-temp diagnostic.

#### Scenario: Shared workspace bootstrap passes
- GIVEN the reviewed shared test shell acquires a capability-rooted temporary workspace
- WHEN the gate runs
- THEN the constructor fixture MUST pass without permitting ambient temp access in ordinary tests.

### Requirement: Workspace validation covers isolation and failure
r[molten.testing.cap_std_validation] Molten MUST include positive tests for concurrent isolation, typed subroots, async fixtures, child-process execution, cleanup, and explicit export, plus negative tests for symlink escape, wrong-root substitution, cross-workspace access, replaced-entry cleanup, export denial, and temporary host-path leakage into canonical evidence.

#### Scenario: Representative workspace suite passes
- GIVEN representative store, node, transport, CLI, and multiprocess fixtures use the shared workspace
- WHEN positive, negative, and structural checks run
- THEN valid isolated workflows MUST pass and every declared escape, cross-root, cleanup, or evidence-leak class MUST deny or remain explicitly diagnostic.

#### Scenario: Only happy-path cleanup is tested
- GIVEN workspace creation and normal cleanup pass but no symlink, wrong-root, or cross-workspace negative fixture exists
- WHEN the change is evaluated for archive
- THEN closeout MUST remain blocked with the missing failure class identified.
