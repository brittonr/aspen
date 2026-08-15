## Context

Repro export and unpack create an output directory and write fixed members through `std::fs`. Retention review bundles build and scan nested artifact groups through ambient directory entries. Dogfood release export reads declared members from an output tree and writes a tar archive through ambient files. These surfaces share logical path, count, size, duplicate, staging, and verification concerns but do not share an authority-carrying output abstraction.

This change covers filesystem materialization, not parsing or semantic validation of the Preserves artifacts themselves.

## Decisions

### 1. Materialization planning is pure

**Choice:** A pure core accepts logical member declarations and a policy profile, validates relative path components, reserved names, duplicates, type, size, count, replacement policy, and expected BLAKE3 content refs, then emits a deterministically ordered `MaterializationPlan` with a BLAKE3 plan identity.

**Rationale:** Path-policy and manifest decisions can be exhaustively tested without touching a host filesystem.

### 2. One explicit destination capability owns writes

**Choice:** The CLI shell creates or opens the requested destination and constructs a `MaterializationRoot`. All descendant directory creation, file create/readback, rename, and cleanup use that capability. Callers submit logical relative members, not joined host paths.

**Rationale:** `cap_std::fs::Dir` protects descendant operations from parent traversal, absolute paths, and symlink escapes when it remains the operation authority.

### 3. Staging and publication stay inside the root

**Choice:** A plan stages members under a reserved in-root leaf derived from the plan hash and opened with create-new semantics. After all bytes and refs verify, the shell publishes according to an explicit no-replace or reviewed-replace policy using in-root operations. Failed or stale plans do not emit a passing materialization receipt; partial staging is cleaned or quarantined through the same root.

**Rationale:** Capability containment alone does not address partial output or stale-plan confusion.

### 4. Inputs use explicit read authority

**Choice:** A command may read a separately supplied source artifact or archive in its outer shell. When it reads a directory of source members, it opens that directory as a read capability and enumerates logical names. It does not reuse the output capability as ambient read authority.

**Rationale:** Source and destination authority should be independently reviewable.

### 5. Archive member policy is canonical before extraction exists

**Choice:** Tar readers and writers normalize and validate member names with the same pure logical-path core. Absolute paths, parent components, platform prefixes, empty components, separator ambiguity, duplicate normalized names, links, devices, and unsupported entry kinds deny. Verification-only readers retain bytes in memory or bounded temporary storage and never call generic archive unpack.

**Rationale:** Archive names already contribute to manifest identity and diagnostics; accepting unsafe or ambiguous names now would create compatibility debt for future materialization.

### 6. Receipts bind logical paths and content, not host locations

**Choice:** Materialization receipts bind schema, profile, plan hash, logical member paths, content refs, byte/count summaries, decision, diagnostics, and non-claims. Absolute destination and staging paths remain display-only and do not affect canonical identity.

**Rationale:** Evidence should be portable across checkout, VM, Nix store, and temporary roots.

### 7. Shared shell does not absorb semantic cores

**Choice:** Repro reveal admission, retention policy, release member closure, signatures, and artifact-kind validation remain in their existing pure cores. They supply an admitted member plan to the materialization shell.

**Rationale:** Filesystem authority must not become a second semantic policy engine.

## Functional core / imperative shell

- **Pure core:** logical path parsing, normalization, duplicate detection, member bounds, replacement planning, deterministic ordering, plan hashing, expected-ref comparison, and receipt payload construction.
- **Imperative shell:** explicit source/destination open, capability-relative staging and writes, streaming hashes, readback, in-root publication, cleanup/quarantine, and display output.

## Migration order

1. Add the pure member/path plan and capability-rooted writer.
2. Convert repro export and unpack fixed-member workflows.
3. Convert retention review bundle output and verification scans.
4. Convert dogfood release directories and tar archive source/destination handles.
5. Apply scoped structural enforcement to converted materializers.

## Risks / Trade-offs

- In-root staged publication may not be crash-atomic across every platform or multi-file tree. Receipts state the observed boundary and do not overclaim.
- Existing commands may allow writing into a non-empty output directory. Preserve behavior only through an explicit reviewed replacement policy; default new surfaces fail closed.
- Archive verification must keep byte and member bounds explicit to avoid replacing path traversal risk with memory or disk exhaustion.
