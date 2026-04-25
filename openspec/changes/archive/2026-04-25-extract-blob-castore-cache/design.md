## Context

`aspen-blob` wraps iroh-blobs and currently depends on `aspen-core` and `aspen-client-api` for replication RPC concerns. `aspen-castore` connects snix castore traits to Aspen blob storage and currently imports `aspen-core` for circuit-breaker behavior. `aspen-cache` owns Nix binary cache metadata/signing and stores artifacts through Aspen blob/KV paths while dev tests pull broader Aspen testing/runtime crates.

Unlike foundational type crates, this family is intentionally runtime-capable: iroh/iroh-blobs, snix, nix-compat, async runtime, and signing dependencies are expected. The extraction goal is not no-std; it is clear separation between reusable backend libraries and Aspen node/app integration.

## Goals / Non-Goals

**Goals:**

- Make reusable blob/castore/cache default graphs avoid root Aspen app/runtime shells and handler registries.
- Keep backend-purpose dependencies explicit and documented.
- Move Aspen replication/RPC/client-context integration behind named features or shell crates.
- Prove downstream fixtures can use the reusable APIs directly.
- Verify Aspen compatibility consumers still compile and pass focused tests.

**Non-Goals:**

- Removing iroh/iroh-blobs from `aspen-blob`.
- Removing snix/nix-compat from castore/cache crates.
- Rewriting blob protocols or Nix cache formats.
- Publishing crates or splitting repositories.
- Extracting full snix bridge/gateway binaries; those remain binary/runtime shells.

## Decisions

### Decision 1: Backend dependencies are allowed when they are the crate purpose

**Choice:** Allow `iroh` and `iroh-blobs` in reusable `aspen-blob`; allow snix/nix-compat dependencies in `aspen-castore` and `aspen-cache` where they are the domain contract.

**Rationale:** A blob backend without iroh-blobs or a snix castore adapter without snix traits is not the valuable reusable unit. The boundary is Aspen app/runtime coupling, not all runtime libraries.

**Alternative:** Force a pure trait-only layer first. Rejected because it would delay the highest-value extraction and create abstractions without an immediate consumer.

### Decision 2: Aspen replication and client RPC become adapter concerns

**Choice:** Any dependency on `aspen-client-api`, handler registries, root `aspen`, or node/client context belongs behind named adapter features or companion shell crates.

**Rationale:** External consumers can use blob/cache primitives without Aspen's client RPC surface, while Aspen still gets its integration path explicitly.

**Alternative:** Keep replication RPC in defaults. Rejected because it makes every blob user also a client-RPC user.

### Decision 3: Circuit breaker dependency must be reusable or gated

**Choice:** If `aspen-castore` needs circuit-breaker behavior, provide a small reusable helper in the family or gate the existing `aspen-core-shell` circuit-breaker path behind an Aspen adapter feature.

**Rationale:** Pulling `aspen-core-shell` into a reusable snix adapter couples cache users to the app shell.

**Alternative:** Keep `aspen-core` dependency by alias. Rejected because this family is specifically trying to avoid shell leakage.

## Risks / Trade-offs

- **[Risk] iroh feature defaults pull broad networking extras** → Mitigate with documented feature sets and `cargo tree` evidence for default/minimal backends.
- **[Risk] Replication code becomes split across crates** → Mitigate with compatibility adapters and representative consumer checks.
- **[Risk] Cache tests accidentally require Aspen cluster fixtures** → Mitigate with separate reusable unit tests and shell integration tests.
- **[Trade-off] Runtime-capable extraction is less minimal than no-std extraction** → Accepted because the user value is independent content-addressed storage/cache, not alloc-only portability.
