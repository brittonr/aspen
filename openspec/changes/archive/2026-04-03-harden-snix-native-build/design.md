## Context

The snix native build pipeline in `aspen-ci-executor-nix` executes Nix builds in-process via snix-build's `BuildService` (bubblewrap sandbox), replacing the `nix build` subprocess. The pipeline has four stages:

1. **Eval** — Resolve flake/npins to a `Derivation` (snix-eval or `nix eval` subprocess)
2. **Materialize** — Ensure all input store paths exist on disk (PathInfoService → disk, upstream cache → disk, or `nix-store --realise`)
3. **Build** — Execute via `BuildService::do_build()` in a bwrap sandbox
4. **Upload** — Store outputs in PathInfoService/BlobService for cache serving

The pipeline works for trivial derivations (VM tests pass) but has gaps that surface with larger dependency closures: phantom placeholder nodes, sequential upstream fetches, and silent input skipping.

## Goals / Non-Goals

**Goals:**

- Correct Node metadata for all resolved inputs (real digests, real sizes)
- Concurrent upstream cache fetches bounded by `MAX_CONCURRENT_FETCHES`
- Fail-fast with actionable errors when inputs are missing after materialization
- Test coverage for the input resolution → verification → build error paths

**Non-Goals:**

- Changing the eval stage (snix-eval / flake-compat / subprocess fallback cascade)
- Adding new materialization backends (e.g., FUSE)
- Modifying the bwrap sandbox execution itself
- Changing the upload/PathInfoService storage path

## Decisions

### 1. Local ingestion via snix-castore's `ingest_path`

**Decision:** Replace placeholder nodes with real ingestion using `snix_castore::import::ingest_path` which walks a filesystem path, ingests blobs/directories, and returns a proper `Node`.

**Rationale:** snix-castore already provides this — it's the standard way to import local filesystem content into the castore model. The ingested Node has a real B3Digest, real size, correct executable bits. No custom tree-walking needed.

**Alternative considered:** Computing B3 hashes manually without storing in BlobService. Rejected because the Node digest must match what BlobService can serve — a hash without backing storage is useless for downstream consumers (cache gateway, replication).

**Scope:** Only triggered when PathInfoService lookup returns `None` and the path exists locally. PathInfoService hits continue to use the cached Node directly.

### 2. Concurrent BFS with `JoinSet` for upstream fetches

**Decision:** Restructure `populate_closure` to spawn fetch tasks into a `JoinSet`, processing up to `MAX_CONCURRENT_FETCHES` at a time. New references discovered from completed fetches are added to the BFS queue immediately.

**Rationale:** The current code acquires a semaphore permit but processes sequentially — the semaphore does nothing. A `JoinSet` with a `Semaphore` provides actual bounded parallelism. For a closure of 500 paths, this reduces fetch time from ~500 × RTT to ~500/8 × RTT.

**Alternative considered:** `FuturesUnordered` with manual stream polling. Rejected because `JoinSet` is simpler, handles cancellation, and is the idiomatic Tokio approach.

### 3. Pre-build verification as a distinct step with structured errors

**Decision:** Add `verify_inputs_present(drv: &Derivation) -> Result<(), InputVerificationError>` called between materialization and `build_derivation`. It checks every required path exists on disk and returns a structured error with all missing paths categorized by source.

**Rationale:** Currently, missing inputs cause opaque bwrap sandbox errors ("No such file or directory" inside the sandbox, without identifying which path). A dedicated check catches this earlier with actionable output.

**Placement:** In `try_native_build` and `try_npins_native_build`, after the materialization block and before `service.build_derivation()`.

### 4. `collect_input_store_paths` returns a structured report

**Decision:** Change `collect_input_store_paths` from `-> Vec<StorePath<String>>` to `-> InputCollectionReport` containing both resolved paths and a list of unresolved/unparseable derivation paths. Log unresolved at `warn` instead of `debug`.

**Rationale:** The caller (`resolve_build_inputs`) needs to know if inputs are incomplete. A silent skip at `debug` level means production builds can proceed with missing inputs and fail later with unrelated errors.

### 5. Retry with exponential backoff for transient HTTP errors

**Decision:** Wrap `fetch_and_ingest_nar` calls in a retry loop (max 2 retries, 1s/2s backoff) for HTTP 5xx, timeouts, and connection resets. 4xx errors and hash mismatches fail immediately.

**Rationale:** Upstream caches occasionally return 503s under load. A simple retry prevents these from becoming build failures.

## Risks / Trade-offs

- **[Risk] Local ingestion adds latency to `resolve_single_input`** → Mitigated: only triggered on PathInfoService miss for locally-present paths. These are typically small inputs (build tools, sources). Large closures are already in PathInfoService from upstream cache population.

- **[Risk] Concurrent fetches increase memory pressure** → Mitigated: bounded by `MAX_CONCURRENT_FETCHES=8`, and `MAX_NAR_FETCH_SIZE=2GB` per NAR. Peak memory for concurrent fetches is 8 × avg NAR size (typically 1-50MB each).

- **[Risk] `ingest_path` writes to BlobService/DirectoryService, increasing storage** → Acceptable: these paths are build inputs that would be stored anyway if they were fetched through the normal upload path. The storage overhead is the correct behavior.
