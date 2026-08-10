# Node-state filesystem authority

Molten treats an operator-selected node state directory as one explicit filesystem authority. The host path is accepted only by public CLI or daemon shells; reusable lifecycle, control, ingress, identity, and target-job logic receives `NodeStateRoot` or a narrower derived view.

## Crate ownership

`crates/molten-node-host` owns the shared error type, `NodeStateRoot`, derived node namespaces, typed local-store roots, and their capability filesystem effects. The root `molten` crate re-exports these exact definitions through `molten::error`, `molten::node_state`, and `molten::local_store`.

CLI parsing, operator presentation, daemon orchestration, service semantics, workload semantics, NixOS validation, test harness orchestration, and release policy remain outside `molten-node-host`. This first extraction does not claim that the complete daemon moved.

## Authority boundary

r[impl molten.node.cap_std_state_root] `NodeStateRoot::open` and `NodeStateRoot::open_existing` are the reviewed ambient bootstrap operations. The root owns an open `cap_std::fs::Dir`, and clones retain that authority rather than reopening the host path. Renaming or replacing the bootstrap pathname therefore does not redirect later descendant operations.

Public compatibility APIs that still accept `&Path` are imperative shells. They validate the operator input, open one root, and immediately delegate to a root-bearing implementation. Host paths may be returned for diagnostics or legacy display fields, but they do not determine canonical receipt identity.

## Namespaces and locators

r[impl molten.node.cap_std_namespaces] `NodeStatePath` accepts only bounded relative components and rejects parent traversal, absolute paths, platform prefixes, URI-like locators, and empty paths before I/O. Fixed namespaces are derived from the open root for:

- identity and persisted secrets;
- ledger and nested artifact, chunk, and delivery stores;
- control inbox, outbox, ingress, idempotency, and service state;
- services, receipts, registry, storage, and other fixed runtime directories.

Directory enumeration is bounded, sorted, and rejects non-UTF-8 names. Enumerated entries bind the exact root, namespace, and subdirectory view that produced them. Reads are bounded, no-follow, and regular-file-only. Writes and removals reject symlinks and non-regular leaves.

## Identity secrets

r[impl molten.node.cap_std_identity_secret] Endpoint secrets are created and loaded through the identity namespace. Unix creation requests owner-only mode. Existing secret leaves are opened without following links; file type, permissions, size, and bytes are observed through the same acquired handle so ambient leaf replacement cannot redirect the read. Receipt evidence binds redacted source metadata and permission decisions, never secret bytes or a host pathname.

## Control request lifecycle

r[impl molten.node.cap_std_request_lifecycle] Pending requests are represented by `NodeStateEntry` values returned by the inbox capability. Entries bind their originating root and exact namespace view. Dispatch reopens the logical entry through that namespace, archives through the outbox namespace, and removes through the inbox namespace. The legacy `Path` compatibility API accepts either one normal UTF-8 entry component or the exact diagnostic path under the selected state root's inbox, converts it to an entry identity, and never treats the supplied host path as a deletion target. Wrong-root entries, links, non-regular leaves, malformed names, and stale canonical bindings deny without lexical host-path authorization.

## Async and nested-store authority

r[impl molten.node.cap_std_async_authority] Live listeners, senders, ingress delivery, service loops, supervisor work, and target-side job execution carry the `molten-node-host` open root across async boundaries. Inner functions do not accept a host root path. Artifact, ledger, chunk, and delivery-idempotency adapters receive capability-derived store roots; Redb files are opened through those roots.

Node job execution reads the admitted DAG and recomputes its closure through the registry capability. Inline stages remain pure, and chunk source/materialization effects use the chunk capability. Typed-storage job stages fail closed until supplied with a capability-aware typed-storage adapter; they are never redirected through an ambient descendant path.

## Structural gate and evidence limits

r[impl molten.node.cap_std_regression_gate] The `node-state-ambient-filesystem-call` rule blocks direct ambient descendant I/O in converted node paths. The `node-state-root-reacquisition` rule blocks ambient root reopening in inner node and target-job code. Both rules have positive and negative fixtures and are exercised by the `node-state-authority` Nix check.

r[verify molten.node.cap_std_validation] Positive and negative tests cover bounded locators, sorted namespace I/O, wrong-root entries, missing parents, symlink and non-regular denial, restricted secret permissions, nested stores, and host-path replacement after root opening.

These checks establish scoped filesystem-authority threading and structural containment. They do not prove crash atomicity, distributed consistency, semantic job correctness, credential trust, peer authorization, durability, or release readiness.
