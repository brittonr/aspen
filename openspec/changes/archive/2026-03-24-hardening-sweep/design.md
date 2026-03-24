## Context

Aspen's last 30 days (606 commits) focused on operational hardening: tiger style cleanup, write forwarding, multi-node dogfood, cluster discovery with file-based peer recovery. Unit tests pass (683/683 quick profile). But the three validation layers above unit tests — dogfood pipeline, NixOS VM tests, and snix native build completeness — haven't been systematically verified since these changes landed.

The dogfood pipeline (`scripts/dogfood-local.sh`) exercises Forge push → CI trigger → nix build → deploy → verify. It last ran successfully for multi-node (3-node full-loop) but that was before the cluster discovery and connection pool retry changes.

63 NixOS VM tests exist in `nix/tests/`. The napkin documents many past failures (plugin count assertions, sops-transit, deploy bricking, snix-bridge FUSE issues). Some were fixed, some may have regressed.

The snix native build path in `aspen-ci-executor-nix` has two known hardcoded values: system is always `x86_64-linux` and submodule fetching is always `false`.

## Goals / Non-Goals

**Goals:**

- Validate the dogfood pipeline works end-to-end after recent hardening changes
- Know the pass/fail status of every NixOS VM test and fix regressions
- Remove hardcoded assumptions from snix eval (system detection, submodule parsing)
- All fixes covered by tests

**Non-Goals:**

- Adding new features or capabilities
- Rewriting the dogfood script
- Fixing VM tests that were already broken before this sprint (document, don't fix)
- Multi-arch CI (just wire the plumbing; cross-compilation is future work)
- aarch64 testing (no hardware available in CI)

## Decisions

### 1. Dogfood: run locally, fix forward

Run `nix run .#dogfood-local -- full-loop` on the current codebase. If it fails, fix the immediate cause rather than refactoring the pipeline. The goal is validation, not redesign.

*Alternative*: Run dogfood in a NixOS VM test. Rejected — `ci-dogfood-full-loop-test` already exists as a VM test, and we want to exercise the bare-metal path too.

### 2. VM tests: triage into pass/fail/skip buckets

Build each VM test individually via `nix build .#checks.x86_64-linux.<name>`. Categorize:

- **Pass**: no action needed
- **Fail (regression)**: fix in this change
- **Fail (pre-existing)**: document in a tracking list, don't fix now
- **Skip (infra)**: tests requiring KVM nested virt, external network, or excessive resources — document why

Run tests in parallel where possible (independent VM tests don't share state).

*Alternative*: Run all via `nix flake check`. Rejected — one failure blocks the rest, and we need per-test granularity.

### 3. snix system detection: read from build payload, default from host

The `NixBuildPayload` already has a `system` field but `evaluate_flake_derivation` ignores it and hardcodes `x86_64-linux`. Fix: read `payload.system`, fall back to `std::env::consts::ARCH` + `std::env::consts::OS` mapped to nix system strings (`x86_64-linux`, `aarch64-linux`, `x86_64-darwin`, `aarch64-darwin`).

*Alternative*: Always require explicit system in payload. Rejected — breaks existing callers that omit it.

### 4. submodule parsing: read `submodules` from flake.lock locked input

Nix flake.lock git inputs can include `"submodules": true` in the locked node. Parse this field when present, default to `false` when absent (preserving current behavior for locks without the field).

## Risks / Trade-offs

- **[Dogfood timing]** Full-loop takes 5-15 minutes depending on nix cache state. If it fails deep in the pipeline, debugging is slow. → Mitigation: run the phases individually (`start`, `push`, `build`, `deploy`, `verify`) to isolate failures.

- **[VM test count]** 63 tests × ~2-5 min each = up to 5 hours sequential. → Mitigation: run in parallel via pueue, group by independence. Many will be cached from prior builds.

- **[System detection edge cases]** `std::env::consts` gives `linux`/`macos` and `x86_64`/`aarch64`, need to map `macos` → `darwin` for nix. → Mitigation: exhaustive match with compile-time error for unknown combos.

- **[Submodule flag has no test coverage]** No existing flake.lock fixtures include `"submodules": true`. → Mitigation: add a unit test with a synthetic flake.lock containing the field.
