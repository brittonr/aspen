## Context

Aspen currently uses three snix crates (`snix-castore`, `snix-store`, `nix-compat`) to implement distributed Nix storage backed by iroh-blobs and Raft KV. The CI executor shells out to the `nix` binary for builds. The cache gateway is a 523-line hand-rolled hyper HTTP server. The snix-bridge exposes gRPC for `snix-store` CLI access.

The snix project provides seven additional crates that overlap with or replace Aspen-maintained code:

- `snix-build`: Sandboxed build execution (bubblewrap/OCI) via `BuildService` trait
- `snix-eval` + `snix-glue`: Pure-Rust Nix evaluator with store-aware builtins and fetchers
- `snix-serde`: Nix → Rust serde deserialization
- `nix-daemon`: `NixDaemonIO` implementation wiring `BlobService`/`DirectoryService`/`PathInfoService`
- `nar-bridge`: axum-based HTTP binary cache server with upload support
- `snix-castore-http`: HTTP directory/file browser for castore contents
- `snix-tracing`: Opinionated tracing setup with OpenTelemetry

Aspen already implements the core snix traits (`BlobService`, `DirectoryService`, `PathInfoService`). The remaining crates consume these traits — they plug directly into Aspen's existing implementations.

## Goals / Non-Goals

**Goals:**

- Eliminate `nix` CLI dependency for CI builds — builds execute in-process via snix-build
- Replace hand-rolled HTTP cache gateway with nar-bridge (less code, upload support, range requests)
- Enable nix-daemon protocol so Nix clients can use Aspen as a native store
- Add in-process Nix evaluation for flake introspection and pre-flight validation
- Structured flake ref parsing and derivation analysis for smarter CI caching
- Reduce maintenance surface by adopting upstream implementations over custom code
- Expose castore HTTP browser for debugging store contents

**Non-Goals:**

- Full Nix CLI replacement (we're replacing specific build/eval operations, not `nix develop`, `nix shell`, etc.)
- Running snix as a standalone daemon alongside Aspen — it's embedded, not forked
- Supporting non-Linux build sandboxes (bubblewrap is Linux-only; macOS remains subprocess-based)
- Replacing Aspen's existing tracing infrastructure wholesale — snix-tracing only applies to snix-integrated crate boundaries

## Decisions

### D1: Phased rollout in four tiers

**Tier 1 — Drop-in replacements (low risk, immediate maintenance reduction):**

- `nar-bridge` replaces `aspen-nix-cache-gateway` server code
- `nix-compat::flakeref` replaces string concatenation in `NixBuildPayload::flake_ref()`
- `snix-tracing` adopted for snix-dependent crate initialization
- `snix-castore-http` wired up behind a feature flag for debugging

**Tier 2 — Protocol additions (medium risk, new capability):**

- `nix-daemon` crate wired to Aspen's services via `aspen-snix-bridge`, exposing a Unix socket
- `nix-compat::derivation` parsing added to CI executor for build graph introspection

**Tier 3 — Evaluation integration (higher risk, significant new dependency):**

- `snix-eval` + `snix-glue` embedded for in-process `nix eval` equivalent
- `snix-serde` for config deserialization from Nix expressions

**Tier 4 — Build execution replacement (highest risk, largest payoff):**

- `snix-build` replaces `nix build` subprocess in CI executor
- Requires Tier 3 (evaluation produces derivations consumed by build service)
- Subprocess fallback retained behind `nix-cli-fallback` feature flag

**Rationale:** Each tier is independently valuable and shippable. Tier 1 reduces maintenance immediately. Tiers 2-4 build on each other but can be paused without blocking Tier 1 benefits.

### D2: nar-bridge replaces aspen-nix-cache-gateway

The cache gateway is 523 lines of hyper boilerplate implementing GET `/{hash}.narinfo` and GET `/nar/{hash}.nar`. nar-bridge does this in axum with PUT support, range requests, and an LRU root-node cache. Aspen provides the backing services; nar-bridge provides the HTTP layer.

The `aspen-nix-cache-gateway` binary becomes a thin wrapper: parse CLI args, build Aspen-backed services, call `nar_bridge::AppState::new()`, mount the router, serve.

**Alternative considered:** Keep the custom gateway. Rejected because nar-bridge handles edge cases (range requests, content-type negotiation, concurrent upload with LRU dedup) that the custom code doesn't, and every upstream fix is free.

### D3: nix-daemon socket alongside gRPC bridge

The snix-bridge already runs a gRPC Unix socket. Add a second socket for the nix-daemon protocol using the `nix-daemon` crate's `SnixDaemon` struct, backed by the same Aspen services. This gives Nix clients two access paths:

- gRPC socket → `snix-store` CLI tools
- nix-daemon socket → standard `nix` CLI (`nix build --store unix:///path/to/socket`)

Both share the same `BlobService`/`DirectoryService`/`PathInfoService` instances.

**Alternative considered:** Replace gRPC with nix-daemon only. Rejected because snix-store CLI uses gRPC natively and the bridge already works.

### D4: snix-eval integration via EvalIO trait

`snix-eval` uses an `EvalIO` trait for file access. `snix-glue` provides `SnixStoreIO` which implements `EvalIO` backed by `BlobService`/`DirectoryService`/`PathInfoService`. Since Aspen already implements these traits, evaluation "just works" — the evaluator reads files from the cluster's store.

The `snix-glue` crate also provides fetchers (fetchurl, fetchTarball, fetchGit) with configurable HTTP clients. For CI, these resolve flake inputs.

**Rationale:** snix-eval is the only pure-Rust Nix evaluator. The alternative (shelling out to `nix eval`) requires the Nix binary and can't introspect evaluation results programmatically.

### D5: snix-build integration requires derivation bridge

`snix-build` takes a `BuildRequest` (inputs as castore Nodes, command, env vars, outputs). This isn't a `.drv` file — it's a normalized, content-addressed representation. The flow is:

1. Evaluate flake with `snix-eval` → get derivation
2. Parse derivation with `nix-compat::derivation`
3. Convert derivation → `BuildRequest` (snix-glue handles this)
4. Execute via `BuildService::do_build(request)`

The bubblewrap sandbox runs on Linux only. For non-Linux or when bubblewrap isn't available, retain the `nix build` subprocess fallback.

### D6: Feature flag structure

```
snix           = [snix-castore, snix-store, nix-compat]        # existing
snix-daemon    = [snix, nix-daemon]                             # Tier 2
snix-eval      = [snix, snix-eval, snix-glue, snix-serde]      # Tier 3
snix-build     = [snix-eval, snix-build]                        # Tier 4
snix-http      = [snix, nar-bridge, snix-castore-http]          # Tier 1
```

Each tier maps to a feature flag. The `full` feature enables all of them. Individual features can be disabled on platforms where they don't apply (e.g., `snix-build` on macOS).

### D7: snix-tracing as optional init helper

`snix-tracing` provides a `TracingBuilder` with clap integration and optional OTLP export. Use it in binaries that are primarily snix-facing (`aspen-snix-bridge`) rather than retrofitting all of Aspen's tracing. This keeps it scoped and avoids conflicts with Aspen's existing tracing subscriber setup.

### D8: snix-castore-http behind debug feature flag

Wire `snix-castore-http`'s axum router into the snix-bridge binary behind a `--browse` flag. This gives operators a web UI to browse store path contents during debugging. Not exposed in production by default.

## Risks / Trade-offs

**[snix-eval is large and actively developed]** → Pin to same git rev as existing snix deps. Vendor if upstream changes break Aspen. The evaluator is the single largest new dependency (~30k lines).

**[bubblewrap sandbox is Linux-only]** → Keep `nix build` subprocess as fallback behind `nix-cli-fallback` feature. macOS CI workers continue using the subprocess path.

**[nar-bridge uses axum, Aspen avoids HTTP]** → The cache gateway is inherently HTTP (it serves Nix clients that speak HTTP). axum is already a transitive dependency via snix-castore's gRPC layer. This doesn't violate the "no HTTP API" rule — it's a compatibility bridge for Nix ecosystem tools, not an Aspen API.

**[snix-eval may not evaluate all nixpkgs]** → snix-eval has known gaps vs C++ Nix (some builtins, IFD). For Aspen's dogfood use case (evaluating its own flake), coverage is sufficient. Fall back to subprocess for exotic evaluations.

**[Adding 8 new dependencies increases build time]** → Feature-gated. Default build doesn't include any of them. `full` feature adds them all. CI builds with `full` will be slower but that's acceptable.

## Migration Plan

1. **Tier 1**: Add nar-bridge, flakeref, castore-http, snix-tracing behind feature flags. Wire up. Deprecate `aspen-nix-cache-gateway` internals. Ship.
2. **Tier 2**: Add nix-daemon socket to snix-bridge. Add derivation parsing. Ship.
3. **Tier 3**: Add snix-eval/snix-glue integration. Add snix-serde. Ship.
4. **Tier 4**: Add snix-build. Keep subprocess fallback. Run both paths in CI for a release cycle to validate parity. Remove subprocess path once verified. Ship.

Rollback: Each tier is a feature flag. Disable the flag to revert to previous behavior.

## Open Questions

- Should the nix-daemon socket live in `aspen-snix-bridge` or `aspen-node` directly? Bridge keeps it separate; node makes it always-available.
- What's the minimum snix-eval coverage needed for the dogfood pipeline? Need to test with the actual Aspen flake.
- Should nar-bridge serve on the same port as the existing cache gateway, or a new one? Same port is simpler for existing configs.
