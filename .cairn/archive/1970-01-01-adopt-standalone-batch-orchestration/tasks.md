# Tasks

## Phase 1: Vendor with provenance

- [x] [serial] Vendor the reviewed overlay under `pilots/orchestration` with the provenance record (bundle origin, base commit `bb6f3830ee7327da9875ea85a8c8e25697eddc35`, review date, repair list) and AGPL-3.0-or-later headers. r[molten.batch_orchestration.vendor]
- [x] [serial] Resolve, review, and commit the workspace `Cargo.lock` for the vendored crates. r[molten.batch_orchestration.vendor]

## Phase 2: Land the review repairs

- [x] [serial] Apply the `redb::ReadableDatabase` read-transaction import repair in `adapters/src/storage.rs`. r[molten.batch_orchestration.repairs]
- [x] [serial] Replace the `O_PATH` directory `fsync` in `CapabilityRoot::sync` with a real-flags directory sync and add a regression fixture that fails with `EBADF` under the previous implementation. r[molten.batch_orchestration.repairs]
- [x] [parallel] Resolve strict-Clippy `should_implement_trait` findings on `Resources::add` in `batch-core` without weakening capacity checks. r[molten.batch_orchestration.repairs]

## Phase 3: In-tree validation

- [x] [serial] Run `scripts/verify.sh` with `CLIPPY_STRICT=1` from the vendored tree; resolve any failure as a defect and retain the log, toolchain versions, and source revision. r[molten.batch_orchestration.validation]
- [x] [serial] Build the release CLI and run `scripts/demo.sh`; retain the run directory and demo evidence. r[molten.batch_orchestration.validation]

## Phase 4: Reference scheduler conformance

- [x] [serial] In a clean worktree at the pinned base, run the unchanged `apply.py` default dry-run, `git apply --check`, `git apply`, then `scripts/reference-tests.sh` with repository dependencies; record results. r[molten.batch_orchestration.reference]
- [x] [serial] Reconcile explicitly or record a blocked disposition if the checkout no longer matches the pinned base; never remove installer guards. r[molten.batch_orchestration.reference]
      Historical patch: 29 passing tests at the exact base. Current mainline keeps its job/owner/epoch-bound assignment-token API; the moved-checkout installer refusal is recorded in `evidence/moved-mainline.log`. The delivered installer has no `--check` flag; default dry-run is its guarded check mode.

## Phase 5: Boundary and integration plan

- [x] [serial] Confirm the diff leaves `system_extension::Dispatcher`, root Cargo workspace, ALPN registries, system-tier manifests, and Basalt/Cairn admission unchanged, and record the standalone deployment profile naming. r[molten.batch_orchestration.boundary]
- [x] [serial] Review the vendored `docs/MOLTEN-INTEGRATION.md` port mapping as the follow-on integration plan with per-port gates and non-claims; no port rewiring in this change. r[molten.batch_orchestration.integration] r[molten.batch_orchestration.boundary]

## Executed evidence

See `evidence/2026-10-25-verification.md` and `review.md`: 62 standalone tests,
warnings-denied Clippy, release build, exact-output real-process demo, historical
reference conformance, unchanged root membership, and retained failed attempts.
The in-tree strict run also repaired an oversized command variant without boxing
and propagated four fallible Redb durability-configuration results.
