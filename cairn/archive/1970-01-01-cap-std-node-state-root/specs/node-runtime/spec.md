## ADDED Requirements

### Requirement: Node state is rooted in one explicit capability
r[molten.node.cap_std_state_root] Molten MUST open or create an operator-selected node state root at the outer CLI or daemon shell and MUST represent descendant node-state filesystem authority with a typed capability root. Long-lived node operations MUST consume that authority rather than reopening the ambient state-root path.

#### Scenario: Node lifecycle reuses one root authority
- GIVEN an operator starts a node with an explicit state root
- WHEN initialization, startup, control, status, service, and shutdown operations access node state
- THEN those operations MUST use the bootstrapped node-state capability or a derived namespace view
- AND MUST NOT reacquire ambient authority for descendants.

#### Scenario: Replaced pathname does not redirect an open node root
- GIVEN the host pathname used to bootstrap a running node is renamed or replaced
- WHEN the node performs a later descendant operation
- THEN the operation MUST remain bound to its opened node-state authority or fail safely
- AND MUST NOT silently follow the replacement ambient path.

### Requirement: Node-state namespaces are capability-derived
r[molten.node.cap_std_namespaces] Molten MUST derive identity, secret, ledger, control inbox, control outbox, ingress, service, receipt, and nested-store authority from the node-state capability using fixed or validated relative locators. Peer ids, content refs, operation refs, ticket fields, and receipt fields MUST NOT become ambient paths.

#### Scenario: Valid namespace operation stays in its view
- GIVEN a validated control request ref and a node inbox namespace view
- WHEN the request is persisted
- THEN the leaf MUST be derived as a bounded relative locator and written through the inbox capability.

#### Scenario: Peer-derived locator is rejected
- GIVEN a peer-controlled identifier contains parent traversal, an absolute path, a platform prefix, separators outside its canonical encoding, or a remote locator
- WHEN node-state locator derivation runs
- THEN derivation MUST deny before filesystem access
- AND no other node namespace MUST be opened from that value.

### Requirement: Persisted endpoint secrets use capability-relative secure file operations
r[molten.node.cap_std_identity_secret] Molten MUST create, load, inspect, and rotate the fixed persisted endpoint-secret leaf through the node identity capability. On platforms with enforceable owner-only modes, creation MUST request the reviewed restricted mode and load MUST deny symlinks, non-regular files, or unsafe permission state before secret bytes are accepted.

#### Scenario: Restricted persisted secret is reused
- GIVEN a regular in-root secret file has the reviewed restricted permission state
- WHEN node identity resolution selects persisted-file loading
- THEN Molten MUST read it through the identity capability and bind only redacted source metadata into receipts.

#### Scenario: Secret substitution denies before load
- GIVEN the secret leaf or an intermediate component is a symlink, non-regular object, out-of-root target, or has unsafe permissions
- WHEN identity resolution attempts to load or rotate it
- THEN the operation MUST deny before accepting the secret bytes or replacing the file.

### Requirement: Control request lifecycle uses relative entry identity
r[molten.node.cap_std_request_lifecycle] Molten MUST represent pending control requests with bounded relative entry identities obtained from the inbox capability. Dispatch, archival, and removal MUST reopen the selected entry through that capability and MUST NOT authorize deletion with lexical host-path prefix checks.

#### Scenario: Pending request archives and removes safely
- GIVEN an inbox capability enumerates a valid pending request entry
- WHEN dispatch succeeds and the request is archived
- THEN Molten MUST write the archive through the outbox capability and remove the original through the inbox capability.

#### Scenario: Substituted request entry cannot delete an arbitrary file
- GIVEN a pending entry is replaced, renamed, symlinked, or presented from another root between discovery and mutation
- WHEN archival attempts cleanup
- THEN cleanup MUST fail safely or affect only the capability-relative inbox entry
- AND MUST NOT delete a host path selected by the substituted value.

### Requirement: Async node work carries authority objects
r[molten.node.cap_std_async_authority] Molten MUST pass node-state capability objects or narrow derived ports into live ingress, sender, listener, supervisor, and control-loop tasks. Ambient display paths MAY be retained only in the imperative shell and MUST NOT determine canonical receipt identity or descendant access.

#### Scenario: Live listener writes through supplied authority
- GIVEN a live listener receives a valid ingress envelope
- WHEN it persists envelope and transport evidence
- THEN it MUST use its supplied ingress and receipt authority views
- AND transport identifiers MUST NOT grant filesystem authority.

#### Scenario: Missing task authority fails closed
- GIVEN an async node task has identifiers but no matching node-state capability
- WHEN it needs descendant filesystem access
- THEN the task MUST fail before persistence rather than opening an ambient root from those identifiers.

### Requirement: Node-state adapters have a scoped ambient-filesystem regression gate
r[molten.node.cap_std_regression_gate] Molten MUST maintain a syntax-aware blocking gate for converted node-state modules that rejects direct ambient descendant filesystem calls and ambient root reacquisition. The gate MUST distinguish reviewed node-root bootstrap and separate explicit CLI input reads from node-state adapter effects.

#### Scenario: Ambient node-state mutation fails validation
- GIVEN a converted node module adds direct ambient read, write, list, metadata, or removal of a state-root descendant
- WHEN the node authority gate runs
- THEN the gate MUST fail with a scoped node-state diagnostic.

#### Scenario: Explicit external input remains a shell effect
- GIVEN a node CLI command reads a separately supplied request or policy artifact before passing parsed data inward
- WHEN the gate runs
- THEN the reviewed shell fixture MUST pass without permitting ambient access in reusable node-state logic.

### Requirement: Node-state capability conversion has positive and negative evidence
r[molten.node.cap_std_validation] Molten MUST verify node initialization, lifecycle, queue, lock, ledger, nested-store, endpoint identity, and live-ingress capability behavior with positive tests and negative tests for traversal, symlinks, non-regular files, unsafe permissions, root replacement, wrong-root handles, stale entries, and replacement races.

#### Scenario: Node capability suite passes
- GIVEN node-state APIs use capability-derived authority throughout the lifecycle
- WHEN focused positive and negative tests run
- THEN valid in-root workflows MUST pass and every declared escape or substitution class MUST deny before unauthorized access or mutation.

#### Scenario: Secret or destructive negative coverage is absent
- GIVEN ordinary node lifecycle tests pass but secret substitution or request-deletion negative coverage is missing
- WHEN the change is evaluated for archive
- THEN closeout MUST remain blocked and identify the uncovered authority class.
