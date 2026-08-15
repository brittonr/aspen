## Context

`src/node/parts/daemon/p000/body.rs` wraps ambient filesystem calls used across the node daemon. State layout, request scans, request archival, service-lock removal, receipt writes, and ledger imports all accept `&Path`. `archive_dispatched_request` uses lexical `starts_with` before deleting a supplied request path. Endpoint identity code uses ambient `OpenOptions`, metadata, reads, and writes for a sensitive persisted secret.

The node CLI legitimately starts with an operator-selected state-root path. After that bootstrap, the daemon has no reason to reacquire ambient filesystem authority for descendants.

## Decisions

### 1. One long-lived node-state capability

**Choice:** The outer node shell creates or opens the selected state root and constructs `NodeStateRoot`. Long-lived node services retain this authority object, or narrowly borrowed namespace views, rather than retaining the ambient path as their I/O authority.

**Rationale:** A stable open directory handle continues to name the intended root even when namespace paths are replaced, and it makes node authority explicit in signatures.

### 2. Node namespaces are typed relative views

**Choice:** Expose narrow views for identity, ledger, control inbox, control outbox, ingress, service, receipts, and secrets. Pure helpers derive fixed or validated relative locators from canonical refs and node identifiers; they do not return ambient paths.

**Rationale:** Namespace-specific views reduce accidental cross-subsystem mutation while preserving one bootstrap capability.

### 3. Queue entries carry relative identity, not host paths

**Choice:** Inbox scans return bounded logical entry names or typed relative request locators. Dispatch and archival reopen and remove the selected leaf through the inbox view. Eliminate lexical `starts_with` checks and arbitrary `request_path` deletion from reusable logic. The legacy path-shaped API remains an outer adapter: it accepts a single entry or the exact diagnostic inbox path, converts that value to a logical entry, and never deletes the supplied path.

**Rationale:** Lexical containment is not capability containment and can become stale between check and use.

### 4. Secret operations use the identity capability

**Choice:** Open or create the fixed endpoint-secret leaf through the identity root using capability-relative open options. On supported platforms, apply owner-only creation mode and inspect permissions or file type through the acquired file/entry rather than an unrelated ambient reopen. Deny symlinks, non-regular files, unsafe permissions, and replacement before load or rotation.

**Rationale:** The endpoint secret is the node filesystem object with the highest confidentiality and identity impact.

### 5. Backend and child subsystems receive derived authority

**Choice:** Node ledger, delivery-idempotency, artifact, chunk, and other local stores receive capability-derived subroots or ports. They must not be called with `state_root.join(...)` ambient paths. If a dependency requires `std::fs::File`, bridge an already-open capability-acquired handle.

**Rationale:** The node boundary is only useful if nested store calls preserve it.

### 6. Async tasks do not smuggle ambient paths

**Choice:** Async listener, sender, supervisor, and control-loop inputs carry borrowed or owned authority wrappers plus pure identifiers. Paths used only for operator display are explicitly non-authoritative and omitted from canonical receipt identity.

**Rationale:** Long-lived tasks are otherwise a common place for raw paths to spread beyond the shell.

### 7. Structural enforcement distinguishes CLI inputs from node state

**Choice:** Promote a node-state-specific blocking authority rule after positive and negative fixtures exist. Explicit CLI reads of separately supplied request or policy files remain shell effects; all descendant node-state I/O must use `NodeStateRoot`.

**Rationale:** A blanket `std::fs` ban would conflate explicit input files with reusable node state.

## Functional core / imperative shell

- **Pure core:** node namespace and leaf derivation, ref-to-filename mapping, queue ordering, state-layout plans, lock transitions, secret-source decisions, permission-policy decisions, and diagnostics.
- **Imperative shell:** root bootstrap, capability-relative directory and file operations, mode application, handle conversion, task orchestration, and display-path rendering.

## Migration order

1. Add `NodeStateRoot` and typed namespace views.
2. Convert node layout, receipts, ledger, and identity state.
3. Convert inbox/outbox and lock lifecycles, removing raw request paths.
4. Convert endpoint secret persistence and rotation.
5. Thread derived roots into nested stores and async tasks.
6. Enable node-state structural enforcement.

## Risks / Trade-offs

- Some CLI and test fixtures need host paths to spawn child processes. Keep that bridge in the process shell and pass logical root labels into deterministic evidence.
- Permission APIs vary by platform. Preserve explicit unsupported diagnostics rather than claiming equal host enforcement everywhere.
- `cap-std` limits pathname authority but is not a sandbox for native code that can still call ambient APIs.
