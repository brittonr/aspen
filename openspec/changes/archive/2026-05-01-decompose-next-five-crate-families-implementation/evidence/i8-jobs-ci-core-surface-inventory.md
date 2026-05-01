# I8 Jobs/CI Core Surface Inventory

## Reusable defaults already suitable for extraction

- `aspen-ci-core`: CI schema/config/log chunk types and pure validation/timeout/resource/trigger helpers. Current normal tree is limited to serde/schemars/snafu/uuid/chrono utilities; no Aspen root app, handler, process-spawn, VM, Nix, Iroh, or worker runtime dependencies were observed in the I8 cargo tree evidence.
- `aspen-jobs-protocol`: alloc/no-std job response and worker/status protocol DTOs. Current `--no-default-features` tree is serde only.

## Runtime/adapter shells that must stay out of reusable defaults

- `aspen-jobs`: queue manager, worker, scheduler, durable executor, Redb storage, Tokio runtime, Iroh, `aspen-core`, `aspen-coordination`, blob/KV runtime integration, VM/plugin features.
- `aspen-jobs-worker-*`: concrete worker adapters for blob, maintenance, replication, shell, SQL.
- `aspen-ci`: CI orchestration/runtime agent.
- `aspen-ci-handler` and `aspen-job-handler`: RPC/handler shells.
- `aspen-ci-executor-shell`: shell/process execution adapter.
- `aspen-ci-executor-vm`: VM executor adapter.
- `aspen-ci-executor-nix`: Nix/SNIX build/evaluation adapter.

## Boundary decision for the next implementation slice

I8 should keep `aspen-ci-core` and `aspen-jobs-protocol` as the positive reusable defaults for the first jobs/CI wave. I9 should add positive fixture metadata around those crates and negative fixtures/checks that reject root app, handler, process-spawn, VM, Nix/SNIX executor, and concrete worker imports from reusable defaults.

Primary evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-inventory.txt`.
