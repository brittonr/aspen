# Feature Matrix Evidence (partial)

Status: partial. This artifact records the reusable/default and impacted foundational compile slice completed while preparing I9. `V1` remains unchecked until Aspen compatibility consumers and all named reusable feature sets are added.

## Reusable/default compile slice

| Package | Feature set | Command | Status | Notes |
| --- | --- | --- | --- | --- |
| `aspen-kv-types` | no default features | `cargo check -p aspen-kv-types --no-default-features` | pass | Reusable KV operation/response type default. |
| `aspen-redb-storage` | no default features | `cargo check -p aspen-redb-storage --no-default-features` | pass | Storage pure-helper default; direct deps now leaf constants, BLAKE3, hex, serde. |
| `aspen-raft-kv-types` | no default features | `cargo check -p aspen-raft-kv-types --no-default-features` | pass | Reusable OpenRaft app types without Aspen app/runtime defaults. |
| `aspen-raft-kv` | no default features | `cargo check -p aspen-raft-kv --no-default-features` | pass | Facade config/trait surface without concrete iroh endpoint construction. |
| `aspen-raft-network` | no default features | `cargo check -p aspen-raft-network --no-default-features` | pass | Explicit iroh/IRPC adapter crate; transport dependencies are allowed here, not in storage/facade defaults. |
| `aspen-traits` | default | `cargo check -p aspen-traits` | pass | Impacted foundational trait rail. |
| `aspen-core` | no default features | `cargo check -p aspen-core --no-default-features` | pass | Impacted core no-std/default-boundary rail. |
| `aspen-core-no-std-smoke` | default | `cargo check -p aspen-core-no-std-smoke` | pass | No-std smoke rail. |
| `aspen-core-shell` | layer/global-discovery/sql | `cargo check -p aspen-core-shell --features layer,global-discovery,sql` | pass | Runtime shell compatibility rail for existing `aspen_core::*` import alias consumers. |

Transcript: `openspec/changes/prepare-crate-extraction/evidence/feature-matrix-core-slice.txt`.

## Remaining before `V1` can be checked

- Add compile evidence for Aspen compatibility consumers named by `V7`, `V8`, and `V9`.
- Add/refresh feature-topology evidence for any named reusable feature sets introduced by `I10`-`I12` (for example future `aspen-redb-storage/raft-storage`).
- Re-run dependency-boundary checker once owner-needed exceptions are either assigned or intentionally left as expected failures in a verification task.
