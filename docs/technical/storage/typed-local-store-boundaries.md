# Typed local-store boundaries

A local store is an authority-bearing adapter, not a directory convention. This article examines how typed roots, relative locators, enumeration, and backend file handles divide responsibility. It assumes the [local filesystem authority contract](../../local-filesystem-authority.md) and complements the [node-state boundary](../../node-state-filesystem-authority.md). It is part of the [Technical companion](../README.md); existing governing documents retain authority.

## Types select operations; handles supply authority

`ArtifactStoreRoot`, `ChunkStoreRoot`, `RetentionStoreRoot`, `DataspaceStoreRoot`, and `ExchangeStoreRoot` express which store a reusable operation expects. The inspected implementation also defines ledger, delivery, and durable roots. The [typed-root macro](../../../crates/molten-node-host/src/local_store/parts/mod/p001/body.rs) wraps a `LocalStoreRoot` containing a store kind and an open capability directory. Each typed root opens the appropriate kind and exposes a borrowed root for filesystem operations.

There are two distinct protections here. The typed wrapper prevents accidental substitution at typed call sites. The open directory provides operational containment. A type wrapper around a string, followed by ambient `std::fs` calls on reconstructed descendants, would provide the former appearance without the latter mechanism. Conversely, a generic directory handle says little about whether an artifact operation was intentionally given chunk-store authority.

Path-taking compatibility functions belong to bootstrap shells: they acquire a matching typed root and immediately delegate to root-bearing logic. The contract permits separately reviewed explicit output shells, such as caller-selected export destinations. Export permission and store authority are separate inputs; being able to read a store is not implicit permission to write anywhere on the host.

## Locators carry names, not ambient authority

`LocalStorePath` is a validated logical relative path. Its [implementation](../../../crates/molten-node-host/src/local_store/parts/mod/p000/body.rs) rejects parent traversal and rooted or platform-prefixed components, requires a nonempty result, and limits component count. Joining two validated paths checks their combined count. The governing contract additionally specifies denial of remote tickets, URLs, and content references as local locators.

The parser's behavior should not be conflated with the stricter materialization parser. The inspected local parser ignores `CurDir` components during normalization. The [materialization contract](../../filesystem-materialization-authority.md) explicitly rejects dot components and additional aliases. Both are relative-path boundaries, but they serve different admission formats; copying an assertion about one parser into the other would overstate equivalence.

Enumeration returns a bounded, sorted collection of names, relative paths, and file kinds. Consumers still reopen through their supplied capability. In particular, `LocalStoreEntry` contains no originating-root token in the inspected definition. This differs from node-state entries, whose namespace methods validate explicit root provenance. Do not infer node-state-style wrong-root rejection for an arbitrary local entry solely from its Rust type. Typed operation boundaries and the original-capability calling discipline remain significant.

## File acquisition and database handoff

The local read operation first checks the leaf kind, opens with no-follow options, checks the acquired handle's metadata, and reads from that handle. Writes reject unsuitable observed leaf kinds, open without following the leaf, recheck regular-file status, write, and flush. A successful flush here is not a crash-durability proof.

`open_database_file` follows the same crucial handoff pattern: it checks the database leaf, creates needed parents through the capability, opens through the directory, checks the acquired file, and returns its `std::fs::File` handle. The contract requires Redb construction from that handle rather than a reconstructed host pathname. Once acquired, the database handle does not ask Redb to re-resolve the store's original path.

Bounds are operation-specific. `LocalStorePath` bounds components and enumeration bounds entries; the inspected generic `LocalStoreRoot::read` uses `read_to_end` without a byte-limit parameter. Consequently, this article does not generalize node-state bounded reads to every generic local-store read. Adapters requiring finite payload budgets must establish their own bounds; the [content adapter contract](../../content-store-adapter.md) describes such a separate bounded streaming layer.

## Illustrative wrong-authority review

Consider an artifact adapter that derives the reviewed chunk relationship and then reads payload chunks. The reviewer should ask both whether the derivation is one of the supported relationships and whether every later effect uses the derived directory. Reconstructing `host_root/chunks` and opening it ambiently would discard the authority retained by derivation, even if its lexical path is unchanged.

Now imagine the host root is renamed after acquisition. A retained typed root continues referring to the acquired directory; a new ambient open at the old spelling can reach a replacement. This illustrates why reviewing only argument types or path validation misses the important effect boundary. It does not demonstrate that payload contents are canonical: chunk verification remains a different obligation.

## Verification guidance and limits

Suggested checks include missing-root denial for `open_existing`, symlink and non-regular database leaves, root replacement after acquisition, typed adapter workflows, and remote locators passed to local-path parsing. Inspect the shell-to-`*_with_root` transition and the eventual backend constructor, not merely the top-level signature. The documented ast-grep gate detects prohibited syntax in converted pages; it is not a semantic proof. No test execution is claimed here.

These roots constrain local filesystem access. They do not establish manifest truth, confidentiality, Merkle correctness, deletion admission, atomic publication, durability, or distributed consistency. Cross-store derivation is a reviewed authority relationship, not permission to invent generic public retagging or to infer stronger backend guarantees.

## Sources

- [Local filesystem authority](../../local-filesystem-authority.md)
- [Node-state filesystem authority](../../node-state-filesystem-authority.md)
- [Filesystem materialization authority](../../filesystem-materialization-authority.md)
- [Content-store adapter runtime](../../content-store-adapter.md)
- [Local path and entry definitions](../../../crates/molten-node-host/src/local_store/parts/mod/p000/body.rs)
- [Root operations, database handles, and typed wrappers](../../../crates/molten-node-host/src/local_store/parts/mod/p001/body.rs)
- [Technical companion](../README.md)
