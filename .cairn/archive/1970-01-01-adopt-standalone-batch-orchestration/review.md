# Implementation review: standalone batch orchestration adoption

## Disposition

No outstanding finding in the standalone adoption scope. This is an implementation
review, not an independent security audit, system-extension admission or production
readiness approval. Execution evidence is in
[evidence/2026-10-25-verification.md](evidence/2026-10-25-verification.md).

## Reviewed invariants

- **Provenance and license:** the 57-file supplied overlay is identified by its
  archive BLAKE3 and member prefix; source headers and inherited package licenses
  are AGPL-3.0-or-later. The resolved standalone lockfile and pinned toolchain are
  separate from root dependency ownership. The repair record includes the three
  original findings and the two additional strict-Clippy defects found in-tree.
- **Filesystem authority and durability:** directory synchronization opens `.`
  beneath the held cap-std directory with read flags and no final symlink following.
  It neither syncs an `O_PATH` descriptor nor reconstructs an ambient path. The
  regression exercises create, replace, held-root relocation and create-only
  refusal; existing tests cover parent traversal and symlinked database denial.
- **Durability setup:** all four fallible immediate-durability selections propagate
  before table mutation or commit. The repair does not suppress warnings or
  replace errors with success.
- **Pure scheduling:** `checked_add` preserves overflow checks. A schedule command
  borrows its transient request, while committed `Attempt` state owns its cloned
  request. The borrow is not persisted or held across the later worker effect,
  and no heap box is introduced. Existing positive/deny tests exercise capacity,
  cancellation, fencing, uncertainty and restart invariants.
- **Executable path:** all 62 standalone tests, strict Clippy and the release build
  pass. The actual release CLI's controller/worker DAG fetches exactly `ok`.
  A second, two-worker test covers real process restart, idempotency and peer denial.
- **Reference safety:** the unchanged installer was checked/applied only at its
  exact historical base; 29 selected tests passed there. The current checkout was
  denied. Current mainline's assignment-token completion API is preserved rather
  than overwritten by an older command shape. Default dry-run is documented
  accurately because the delivered installer has no `--check` option.
- **Claim boundary:** locked runtime metadata shows the independent seven-crate
  workspace and the unchanged four-package root workspace. There is no dispatcher,
  root ALPN/fabric-port, system-tier manifest or production-admission rewiring.
  The integration plan names each owner, integration law and follow-on gate.

## Residual limits

Same-host tests do not qualify WAN/relay or cross-host behavior, controller HA,
production isolation, valuable workloads, exactly-once external effects or any
whole-system correctness claim. In-process component recovery is not native/VM
process cleanup evidence. The private demo directory is retained locally; only
nonsecret logs, job and status projections are committed as execution evidence.
