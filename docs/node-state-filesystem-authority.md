# Node-state filesystem authority

Molten treats an operator-selected node state directory as one explicit filesystem authority. The host path is accepted only by public CLI or daemon shells; reusable lifecycle, control, ingress, identity, and target-job logic receives `DirectoryRoot` or a narrower derived view.

## Crate ownership

`crates/molten-node-host` owns the shared error type, `DirectoryRoot`, derived node namespaces, typed local-store roots, and their capability filesystem effects. The root `molten` crate re-exports these exact definitions through `molten::error`, `molten::node_state`, and `molten::local_store`.

CLI parsing, operator presentation, daemon orchestration, service semantics, workload semantics, NixOS validation, test harness orchestration, and release policy remain outside `molten-node-host`. This first extraction does not claim that the complete daemon moved.

## Authority boundary

r[impl molten.node.cap_std_state_root] `DirectoryRoot::open` and `DirectoryRoot::open_existing` are the reviewed ambient bootstrap operations. The root owns an open `cap_std::fs::Dir`, and clones retain that authority rather than reopening the host path. Renaming or replacing the bootstrap pathname therefore does not redirect later descendant operations.

Public compatibility APIs that still accept `&Path` are imperative shells. They validate the operator input, open one root, and immediately delegate to a root-bearing implementation. Host paths may be returned for diagnostics or legacy display fields, but they do not determine canonical receipt identity.

## Namespaces and locators

r[impl molten.node.cap_std_namespaces] `RelativePath` accepts only bounded relative components and rejects parent traversal, absolute paths, platform prefixes, URI-like locators, and empty paths before I/O. Fixed namespaces are derived from the open root for:

- identity and persisted secrets;
- ledger and nested artifact, chunk, and delivery stores;
- control inbox, outbox, ingress, idempotency, and service state;
- services, receipts, registry, storage, and other fixed runtime directories.

Directory enumeration is bounded, sorted, and rejects non-UTF-8 names. Enumerated entries bind the exact root, namespace, and subdirectory view that produced them. Reads are bounded, no-follow, and regular-file-only. Writes and removals reject symlinks and non-regular leaves.

## Identity secrets

r[impl molten.node.cap_std_identity_secret] Endpoint secrets are created and loaded through the identity namespace. Unix creation requests owner-only mode. Existing secret leaves are opened without following links; file type, permissions, size, and bytes are observed through the same acquired handle so ambient leaf replacement cannot redirect the read. Receipt evidence binds redacted source metadata and permission decisions, never secret bytes or a host pathname.

## Control request lifecycle

r[impl molten.node.cap_std_request_lifecycle] Pending requests are represented by `Entry` values returned by the inbox capability. Entries bind their originating root and exact namespace view. Dispatch reopens the logical entry through that namespace, archives through the outbox namespace, and removes through the inbox namespace. The legacy `Path` compatibility API accepts either one normal UTF-8 entry component or the exact diagnostic path under the selected state root's inbox, converts it to an entry identity, and never treats the supplied host path as a deletion target. Wrong-root entries, links, non-regular leaves, malformed names, and stale canonical bindings deny without lexical host-path authorization.

Ingress publication uses `DirectoryRoot::write_new`: it creates a regular leaf without
replacement in an existing capability-owned parent, synchronizes the payload, and
opens that same parent for synchronization. An error retains partial bytes for
reconciliation rather than removing or overwriting uncertain evidence. Local-store
`create_database_file` similarly requires no-replace creation, distinguishing a new
Redb file from an existing empty or repair-requiring database. Existing stores cannot
be reinitialized as fresh delivery history. These mechanics do not prove hardware
durability or authorize a control operation.

## Async and nested-store authority

r[impl molten.node.cap_std_async_authority] Live listeners, senders, ingress delivery, service loops, supervisor work, and target-side job execution carry the `molten-node-host` open root across async boundaries. Inner functions do not accept a host root path. Artifact, ledger, chunk, and delivery-idempotency adapters receive capability-derived store roots; Redb files are opened through those roots.

Node job execution reads the admitted DAG and recomputes its closure through the registry capability. Inline stages remain pure, and chunk source/materialization effects use the chunk capability. Typed-storage job stages fail closed until supplied with a capability-aware typed-storage adapter; they are never redirected through an ambient descendant path.

## Structural gate and evidence limits

r[impl molten.node.cap_std_regression_gate] The `node-state-ambient-filesystem-call` rule blocks direct ambient descendant I/O in converted node paths. The `node-state-root-reacquisition` rule blocks ambient root reopening in inner node and target-job code. Both rules have positive and negative fixtures and are exercised by the `node-state-authority` Nix check.

r[verify molten.node.cap_std_validation] Positive and negative tests cover bounded locators, sorted namespace I/O, wrong-root entries, missing parents, symlink and non-regular denial, restricted secret permissions, nested stores, and host-path replacement after root opening.

These checks establish scoped filesystem-authority threading and structural containment. They do not prove crash atomicity, distributed consistency, semantic job correctness, credential trust, peer authorization, durability, or release readiness.

## Startup source evidence and operator refresh

Fresh normal startup intentionally fails closed without independently authenticated exact-candidate evidence. It no longer invents a passing Octet receipt, treats package version as executable identity, or consumes prior clean-shutdown evidence before source admission. Both path and capability-rooted startup wrappers use the same loader. Source rejection leaves the active lock, startup/adapter receipts and prior shutdown evidence untouched.

The product producer profile is `nix/startup-source-binding.nix`, authored with `config/startup-source-profile.ncl`. Its reviewed same-source builders must compile the candidate and run the real primary/library Octet cohort from one immutable source. The producer reruns strict receipt evaluation and measures the actual ELF, source and compiler NAR bytes, complete candidate derivation input closure, policy/profile and retained cohort bytes. It emits a detached `binding.json`; the executable does not embed its own hash. Direct invocation of the measurement script creates identity data only, never authoritative evidence.

`build-closure` retains exactly six canonical graph categories: candidate build/compiler/runtime and gate build/compiler/runtime. The gate compiler is the actual distinct pinned Octet compiler, not an assumed alias of the candidate compiler. Each raw Nix-exported graph is retained and bound; the producer records a sorted deduplicated union of every referenced store root with its measured BLAKE3 NAR identity. Startup independently reconstructs that union, rejects missing categories or paths, and rehashes every actual NAR through the approved verifier (`nix hash path --type blake3 --format base16`, explicit `blake3-hashes` feature). Merely recording graph references or compiler labels is insufficient. The signed output also contains every production-built candidate `bin/` member as a regular executable ELF, including the native-extension and collaboration binaries; symlinks, wrappers and unsigned test-build substitutions are rejected.

Credential/bundle authoring can call `node_daemon::verify_selected_node_output_with_root` before executing a selected binary. This uses the same independently trusted signatures, complete measured closures and artifact checks, but hashes the immutable selected `bin/molten` rather than the authoring process. It emits only `node-selected-source-binding-validation-v1` evidence and an `expected_startup_validation_ref`; it cannot construct a current-process startup token. The expected reference is a future executable binding, not evidence that startup occurred. Normal daemon/source admission remains hardwired to `/proc/self/exe` with no selected-output fallback, and must independently obtain the matching actual startup token.

The signer must be independently authorized for this reviewed producer profile. Have that authority sign the complete realized **input-addressed** output, using the existing Nix signing workflow, then provision `source-gate-trust.json` in the initialized node root through the operator's normal protected configuration channel. Do not copy a key from the evidence or automatically authorize a locally generated signing key. The trust record has schema `molten-node-source-trust-v1`, absolute approved immutable verifier path `nix`, exact `/nix/store/...` producer `output`, one full `public_key` (`name:base64`), and `selection` containing the exact BLAKE3 byte `binding_ref`, `policy_ref`, and `profile_ref`. These identities are measured from the realized binding and selected independently of untrusted evidence. The trust record itself is operator authority, not an artifact supplied by an evidence sender.

The loader clears inherited verifier environment, substituters, extra public keys, local secret-key trust and plugins. It rejects content-addressed trust bypass, requires the selected key's actual signature and invokes full NAR content verification with an explicit signature threshold. It then remeasures `/proc/self/exe`, gate cohorts and receipts and performs bounded pure candidate admission. This trusts the approved Nix verifier, store/kernel integrity during observation and the selected signer's adherence to the reviewed producer profile; it does not establish compiler correctness. A content hash, an unsigned local build or a key name alone cannot satisfy that boundary.

When executable, source, dependencies, compiler, policy, profile or gate artifacts change, rerun the real producer, obtain its independently authorized signature, and replace the selected binding/cohort identities through the same protected operator channel. Merely editing references or signing an arbitrary caller-composed manifest is not refresh. For a controlled positive test, a separately provisioned explicitly experimental signing authority may be used; record it as experimental, never as production operator approval.

The producer binding and source-binding validation receipt have explicit version-1 boundaries. Incompatible producer or cohort changes require a new version. Existing startup receipt fields remain readable; accepted fresh receipts retain the exact binding in version refs and a `node-source-binding-validation-v1` reference. Legacy synthetic receipts remain historical diagnostics and never acquire real execution or authentication during replay. Explicit cfg(test) fixtures emit `node-source-fixture-validation-v1` and are unavailable to normal startup. Neither source-gate acceptance nor a positive experiment flips production admission or grants activation authority.
