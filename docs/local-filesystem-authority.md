# Local filesystem authority boundary

Molten opens operator-declared local roots at reviewed compatibility or runtime shells, then carries typed capability authority through artifact, chunk, retention, local dataspace, and local exchange operations.

## Authority flow

r[impl molten.chunk_store.cap_std_ambient_boundary] Root acquisition is the only ambient step. A path-taking compatibility function opens the matching `ArtifactStoreRoot`, `ChunkStoreRoot`, `RetentionStoreRoot`, `DataspaceStoreRoot`, or `ExchangeStoreRoot` once and immediately delegates to a `*_with_root` operation. Reusable store logic borrows that typed root; it does not recover the host path, call `open_ambient_dir`, canonicalize descendants, or reopen children through `std::fs`.

r[impl molten.chunk_store.cap_std_operational_roots] Operational containment comes from invoking reads, writes, removals, existence checks, and directory scans through the supplied `cap_std::fs::Dir`. A type alias, validated string, or `root.join(child)` followed by ambient I/O is not capability adoption.

The reviewed ambient classes are:

| Class | Allowed use |
| --- | --- |
| Bootstrap shell | Create/open an explicit operator-selected top-level root, then delegate immediately. |
| Explicit output shell | Materialize a caller-selected output outside store state; this authority is separate from store authority. |
| Adversarial test setup | Create corruption, symlinks, replacement races, and missing-authority fixtures under `#[cfg(test)]` or excluded test fixture paths. |
| Converted operation | No ambient child I/O; use a borrowed typed root and validated relative locator. |

### Audited inventory

| Location | Classification | Disposition |
| --- | --- | --- |
| `LocalStoreRoot::open` and `LocalStoreRoot::open_existing` | bootstrap shell | Retained as the single reviewed `create_dir_all`/`open_ambient_dir` acquisition point. |
| Public artifact, chunk, retention, dataspace, and exchange path APIs | bootstrap shell | Retained only when they acquire one typed root and immediately delegate to `*_with_root`. |
| `src/iroh/exchange.rs::write_explicit_output` | explicit output shell | Retained outside reusable exchange store adapters for caller-selected export output. |
| `src/local_store.rs` test module and adapter `tests/**` files | adversarial test setup | Retained to construct tampering, symlinks, root replacement, corruption, and missing-file cases. |
| `src/artifacts/parts/mod/p005/body.rs` | adversarial test setup | Retained because this generated include page contains only the inline `#[cfg(test)]` module. |
| Converted production adapter pages | converted operation | No remaining direct `std::fs`/`fs` child calls, `open_ambient_dir`, descendant canonicalization, or path reconstruction followed by ambient I/O. |

## Relative locators and enumeration

r[impl molten.chunk_store.cap_std_relative_enumeration] `LocalStorePath` accepts only bounded relative components. It rejects parent traversal, absolute or platform-prefixed paths, URLs, Iroh tickets, and content refs. Directory enumeration returns a bounded, sorted `LocalStoreEntry` list containing logical relative paths and explicit file kinds. Consumers reopen those entries through the original capability and reject symlinks or non-regular leaves where regular files are required.

Typed derivation methods encode the few reviewed cross-store relationships: artifact payload chunks, chunk-GC retention evidence, and fixture-state subroots. Generic public root retagging is not exposed, so callers cannot substitute an arbitrary store authority for another typed operation.

## Backend handles

r[impl molten.chunk_store.cap_std_backend_handles] Redb database leaves are fixed relative locators. `LocalStoreRoot::open_database_file` rejects symlink and non-regular leaves, opens the file through `cap_std`, and passes only the acquired `std::fs::File` handle to `redb::Builder::create_file`. Redb never receives a reconstructed ambient database path.

## Structural gate

r[impl molten.chunk_store.cap_std_regression_gate] `store-ambient-filesystem-call.yml` is a blocking ast-grep rule scoped to converted adapter pages. Positive fixtures contain prohibited ambient calls. Negative fixtures cover typed bootstrap/delegation and separately scoped adversarial test setup. The gate is syntax-level regression evidence only; clean output does not prove semantic containment by itself.

## Validation coverage

r[impl molten.chunk_store.cap_std_conversion_validation] Positive tests execute artifact, chunk, retention, dataspace, exchange, enumeration, and Redb workflows through typed roots. Negative tests reject traversal, absolute and platform-prefixed paths, remote/content locators, symlink leaves and intermediates, root-replacement races, wrong-root substitution, non-regular database leaves, and opening a missing authority root.

## Non-claims

Capability roots bound local filesystem authority only. They do not prove durability, atomicity, crash consistency, artifact truth, confidentiality, remote transport trust, Merkle correctness, policy admission, deployment safety, or distributed runtime correctness. Symlink denial and typed-root tests establish only the exercised platform and operation boundaries. Existing manifests, content refs, receipts, policy evidence, provenance evidence, and runtime gates remain responsible for their own claims.
