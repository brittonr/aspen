# Feature Matrix Evidence (partial)

Status: partial. This artifact records the reusable/default and impacted foundational compile slice completed while preparing I9, plus prepared Aspen compatibility consumer rails for `V7`, `V8`, and `V9`. `V7`-`V9` remain unchecked because they are post-migration verification rails for `I10`-`I12`; `V1` remains unchecked until future named reusable storage/facade feature sets from `I10`-`I12` exist and are added.

## Reusable/default compile slice

| Package | Feature set | Command | Status | Notes |
| --- | --- | --- | --- | --- |
| `aspen-kv-types` | no default features | `cargo check -p aspen-kv-types --no-default-features` | pass | Reusable KV operation/response type default. |
| `aspen-redb-storage` | no default features | `cargo check -p aspen-redb-storage --no-default-features` | pass | Storage pure-helper default; direct deps now leaf constants, BLAKE3, hex, serde. |
| `aspen-raft-kv-types` | no default features | `cargo check -p aspen-raft-kv-types --no-default-features` | pass | Reusable OpenRaft app types without Aspen app/runtime defaults; legacy `aspen-raft-types` package/API transition now documented in the manifest. |
| `aspen-raft-kv` | no default features | `cargo check -p aspen-raft-kv --no-default-features` | pass | Facade config/trait operation surface without concrete Redb storage or iroh endpoint construction. |
| `aspen-raft-network` | no default features | `cargo check -p aspen-raft-network --no-default-features` | pass | Explicit iroh/IRPC adapter crate; transport dependencies are allowed here, not in storage/facade defaults. |
| `aspen-traits` | default | `cargo check -p aspen-traits` | pass | Impacted foundational trait rail. |
| `aspen-core` | no default features | `cargo check -p aspen-core --no-default-features` | pass | Impacted core no-std/default-boundary rail. |
| `aspen-core-no-std-smoke` | default | `cargo check -p aspen-core-no-std-smoke` | pass | No-std smoke rail. |
| `aspen-core-shell` | layer/global-discovery/sql | `cargo check -p aspen-core-shell --features layer,global-discovery,sql` | pass | Runtime shell compatibility rail for existing `aspen_core::*` import alias consumers. |

Transcript: `openspec/changes/prepare-crate-extraction/evidence/feature-matrix-core-slice.txt`.

## Aspen compatibility consumer rails

| Task | Artifact | Status | Notes |
| --- | --- | --- | --- |
| `V7` | `openspec/changes/prepare-crate-extraction/evidence/compat-node-cluster.md` | prepared/pass; unchecked | Node-runtime root package, cluster library, and RPC handler aggregate compile after the reusable KV boundary changes. |
| `V8` | `openspec/changes/prepare-crate-extraction/evidence/compat-cli-dogfood-handlers.md` | prepared/pass; unchecked | CLI, dogfood, aggregate handlers, and exact handler package list compile. This rail exposed stale worker dependencies on the alloc-only `aspen-core`; worker crates now alias `aspen-core-shell` where Redb table definitions are required. |
| `V9` | `openspec/changes/prepare-crate-extraction/evidence/compat-bridges-web-tui.md` | prepared/pass; unchecked | Bridge/gateway/web/TUI package list compiles. This rail exposed stale SNIX circuit-breaker callers that still passed `Instant`; SNIX services now pass explicit wall-clock milliseconds to the pure circuit-breaker core. |

## Remaining before `V1` can be checked

- Add the core UI no-std regression rail now required by `V1`: `cargo test -p aspen-core --test ui`.
- Add/refresh feature-topology evidence for named reusable feature sets introduced by `I10`-`I12` (for example future `aspen-redb-storage/raft-storage`).
- Re-run dependency-boundary checker once owner-needed exceptions are either assigned or intentionally left as expected failures in a verification task.
