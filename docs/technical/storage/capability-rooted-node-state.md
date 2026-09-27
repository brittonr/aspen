# Capability-rooted node state

Node-state containment is about preserving an acquired authority across operations, not proving that a pathname looks safe. This article explains the distinction for readers familiar with the [node-state authority contract](../../node-state-filesystem-authority.md), and connects root acquisition, namespace derivation, entry identity, and asynchronous work. The governing contract remains authoritative; this is a [Technical companion](../README.md), not a new admission policy.

## Acquisition changes the meaning of a root

An operator-selected host path is input to an imperative bootstrap shell. After `NodeStateRoot::open` or `open_existing`, the operative resource is an open `cap_std::fs::Dir`. In the [authority implementation](../../../crates/molten-node-host/src/node/state/authority.rs), `NodeStateRoot` contains an `Arc<NodeStateInner>`, and that inner object owns the directory. Cloning the root shares that acquired authority; it does not resolve the bootstrap pathname again.

This distinction matters when the host directory tree changes. A textual root can be renamed and replaced by a different directory without changing its spelling. Code that later performs ambient I/O using the old spelling can reach the replacement. Code carrying the acquired directory continues to operate relative to its retained authority. The latter is the contract's relevant containment property, not a claim that all descendants are immutable or that concurrent writers have disappeared.

The bootstrap itself remains an effectful boundary. `open` validates the bootstrap path and metadata, creates a missing directory, and delegates to `open_existing`; `open_existing` performs the ambient open. These are not deterministic laws in `molten-core`. The shared definitions live in `molten-node-host`, while the root crate supplies public re-exports and the surrounding application semantics.

## Namespace identity is more than a prefix

The root derives named views such as identity, ledger, control inbox, and control outbox. A `NodeStateNamespace` carries the root identity, namespace kind, logical scope, and its own directory handle. Its methods route bounded reads, restricted writes, enumeration, and removal through that view. See the [namespace implementation](../../../crates/molten-node-host/src/node/state/namespace.rs).

The namespace kind is not equivalent to a unique physical directory name. For example, the inspected mapping sends both `Identity` and `Secrets` to `identity`, and both `ControlIngress` and `Ingress` to `control/iroh-ingress`. Nevertheless, entry validation compares namespace kinds as well as root identity and scope. A physical-directory coincidence is therefore insufficient to establish that an enumerated entry belongs to another logical view.

`NodeStateEntry` records its originating root, namespace, scope, logical path, name, and observed kind. Before `read_entry` or `remove_entry`, `validate_entry` checks shared-root pointer identity, namespace equality, and scope equality. Regular-file operations additionally reject entries whose recorded kind is not regular, and then use the namespace filesystem operations. Enumeration is not a blanket authorization to act through any root with a matching filename.

This process-local pointer comparison is authority provenance, not canonical artifact identity. It is neither a stable serialization format nor a Rust-layout identity claim. Canonical receipts do not derive identity from host addresses or diagnostic paths.

## Illustrative replacement and dispatch scenario

Suppose a shell opens an operator-selected state directory and obtains its control inbox view. It enumerates a regular entry named `request-a`. Before dispatch, another process renames the host directory and creates a fresh directory at the original pathname.

The reusable dispatcher should carry the already-open root, not reopen the original pathname. The entry remains associated with the original root and inbox scope. The documented lifecycle archives through the outbox namespace and removes through the inbox namespace; a caller-provided diagnostic path is not the deletion target. A second independently opened root cannot borrow the first root's entry merely because its inbox also contains `request-a`.

This example establishes which authority the operation uses. It does not establish that the request bytes stayed unchanged between enumeration and a later read. The [governing contract](../../node-state-filesystem-authority.md) separately describes malformed names, stale canonical bindings, symlinks, and non-regular leaves as denial cases. Filesystem containment and request-content validation answer different questions.

## Review and verification guidance

A useful review follows authority through a complete lifecycle: shell acquisition, derived namespace, queued task, dispatch, archive, and removal. At each asynchronous boundary, inspect the captured value. Retaining the root or a derived namespace preserves the intended boundary; retaining a path for later reacquisition does not.

Suggested verification includes opening a root before host-path replacement, attempting cross-root and cross-scope entry reuse, and replacing regular leaves with symlinks. Identity-secret review should check that permissions, size, type, and bytes come from the acquired file observation rather than separate ambient pathname lookups. These are proposed review exercises, not executions reported by this article.

The structural regression gate described by the contract is useful for finding direct ambient descendant I/O and inner root reacquisition. Its syntax-level result supplements, rather than replaces, runtime adversarial cases. Typed nested stores also need the [local-store boundary](../../local-filesystem-authority.md); a node root is not a reason to bypass narrower store authority.

## Limits

The boundary establishes scoped local filesystem authority. It does not establish durability, crash atomicity, credential trust, peer authorization, correct job semantics, distributed consistency, or release readiness. A safe root acquisition cannot turn an untrusted request into an admitted operation, and carrying the right directory cannot substitute for canonical receipt validation.

## Sources

- [Node-state filesystem authority](../../node-state-filesystem-authority.md)
- [Local filesystem authority](../../local-filesystem-authority.md)
- [Node root and namespace-kind implementation](../../../crates/molten-node-host/src/node/state/authority.rs)
- [Namespace operations and entry provenance](../../../crates/molten-node-host/src/node/state/namespace.rs)
- [Technical companion](../README.md)
