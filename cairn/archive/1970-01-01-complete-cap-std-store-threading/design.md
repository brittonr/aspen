## Context

The archived capability-store change added `LocalStorePath`, `LocalStoreRoot`, typed aliases, and focused wrapper tests. Current production operations still use ambient paths in `src/chunk/parts/store`, `src/retention/parts`, `src/remote/parts/dataspace`, and `src/iroh/parts/exchange`; the typed roots are opened by helper functions but are not threaded into those operations. `src/artifacts` and derived Redb indexes have the same split boundary.

The `cap-std` reference guarantees containment only when child operations are invoked through `cap_std::fs::Dir`. A validated string followed by `std::fs::read(root.join(name))` does not retain that guarantee.

## Decisions

### 1. Operational authority, not aliases, defines completion

**Choice:** A converted effectful API accepts a borrowed typed capability root (or a narrow port backed by one), and all child filesystem operations use that authority. Merely exposing an alias or constructor does not satisfy the boundary.

**Rationale:** Authority must be present at the operation that opens, lists, mutates, or removes a filesystem object.

### 2. Relative locator validation remains a pure core

**Choice:** Parsing and validating logical store locators remains deterministic over in-memory strings and components. It returns an opaque relative locator that cannot expose or reconstruct the ambient root. Capability wrappers own I/O and error translation.

**Rationale:** Locator policy is straightforward to test without a filesystem, while containment and operating-system errors belong to the imperative shell.

### 3. Ambient authority is acquired once at an outer boundary

**Choice:** CLI and runtime adapters may create or open an explicit operator-selected root using ambient authority. They then pass the typed root inward. Store modules must not call `open_ambient_dir`, `std::fs`, `Path::canonicalize`, or root-relative `PathBuf::join` for child I/O.

**Rationale:** Aspen must retain a deliberate bootstrap point, but ambient authority should not be reacquired throughout the call graph.

### 4. Directory traversal returns logical entries

**Choice:** Enumeration helpers return bounded, sorted relative names or typed entry records. Callers reopen entries through the same capability root rather than consuming `DirEntry::path()` host paths.

**Rationale:** Returning ambient paths from a capability-backed scan would leak the namespace representation and invite later ambient reopens.

### 5. Redb receives a capability-acquired file handle

**Choice:** The store root opens the fixed Redb leaf with capability-relative open options, then bridges the already-open file handle to `redb::Builder::create_file` (or the equivalent file/backend API). The database filename is a reviewed constant or validated relative locator, never an untrusted ambient path.

**Rationale:** Redb supports an already-open file, so its storage engine need not weaken the authority boundary.

### 6. Compatibility entry points are thin shells

**Choice:** Existing path-taking CLI-facing functions may be retained during migration only when they open the corresponding typed root and immediately delegate. New internal APIs and tests use capability roots directly. Compatibility shells are explicitly included in the authority audit allowlist; store modules are not.

**Rationale:** This limits migration churn without preserving ambient authority in reusable logic.

### 7. Structural enforcement is scope-aware

**Choice:** Add positive and negative ast-grep fixtures and promote a scoped rule to blocking for converted adapter paths. The rule permits reviewed bootstrap calls in named outer-shell modules and test-only adversarial setup, but rejects direct ambient filesystem operations in store adapters.

**Rationale:** The repository already inventories ambient calls. A narrowly scoped blocking posture prevents the alias-only state from recurring without falsely banning legitimate explicit CLI input reads.

## Functional core / imperative shell

- **Pure core:** locator parsing, component and entry bounds, logical path construction, content-ref-to-leaf mapping, mutation plans, deterministic ordering, and diagnostics.
- **Imperative shell:** explicit root creation/opening, `cap_std::fs::Dir` operations, file-handle bridging to Redb, OS error mapping, and CLI compatibility adapters.

## Migration order

1. Strengthen the reusable root and relative-locator APIs.
2. Convert artifact and chunk stores, including the Redb index handle.
3. Convert retention stores and destructive operations.
4. Convert local dataspace and local exchange stores.
5. Remove obsolete ambient helper modules and enable the scoped structural gate.

## Non-goals

- Do not replace Iroh networking with `cap_std::net::Pool`; Iroh owns a different asynchronous transport boundary.
- Do not introduce ambient `cap-std` clock or randomness into deterministic cores.
- Do not claim atomic writes, crash durability, confidentiality, or untrusted-Rust sandboxing merely because paths are capability-relative.

## Risks / Trade-offs

- Threading roots changes many signatures. Compatibility shells keep command behavior stable while the internal authority contract changes.
- Capability-relative enumeration has different entry types from `std::fs`; adapt once in the filesystem shell rather than leaking both APIs.
- Tests that intentionally tamper with files need a separate adversarial setup authority; production code must never receive that authority.
