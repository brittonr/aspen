## Context

The prior VM-CI change fixed bootstrap health, bridge-scoped guest tickets, relay-disabled guest endpoints, unique guest IPs, and NixOS firewall-chain ingress. A live retry then reached the next boundary: VM workers connected to the host and a guest received `ci_nix_build`, but the full dogfood run did not complete before manual cleanup. The next debugging target is therefore the VM CI workspace/blob/job execution path, not the already-repaired Iroh bridge path.

## Goals / Non-Goals

**Goals:**
- Make VM-CI post-registration stalls diagnosable from receipts/logs without ad-hoc greps.
- Preserve evidence before cleanup so runs can be archived and compared.
- Add tests around classification and progress evidence so future regressions fail deterministically.

**Non-Goals:**
- Rewriting TAP/helper/firewall/ticket scoping unless diagnostics prove a regression.
- Requiring sudo-only packet capture as the primary evidence path.
- Running the full dogfood loop repeatedly without a new diagnostic signal or code change.

## Decisions

### 1. Classify by highest reached VM-CI boundary

**Choice:** Diagnostics should classify the highest proven boundary: setup/preflight, guest ticket scoped, worker registered, job assigned, workspace materialized, executor started, job result published.

**Rationale:** The recent run proved worker registration/job assignment, so future reports must not collapse that into the old connectivity blocker.

**Alternative:** Continue reporting only top-level timeout/failure. Rejected because it hides progress and encourages repeated expensive reruns.

### 2. Preserve host and guest logs before cleanup

**Choice:** Dogfood cleanup paths should preserve logs/receipts for failed or timed-out VM-CI runs that reached worker registration or job assignment.

**Rationale:** `/tmp/aspen-dogfood` is transient and `stop`/manual cleanup can destroy the only evidence.

**Alternative:** Rely on operators to manually copy logs. Rejected because it is fragile and misses timeout paths.

### 3. Use redacted structured summaries over raw secret-bearing output

**Choice:** Evidence summaries may include tickets/blob IDs only when redacted or bounded; raw secrets and long opaque credentials must not be committed or printed in final summaries.

**Rationale:** Runtime commands can include Iroh secret keys and tickets.

**Alternative:** Keep full raw process lines. Rejected for credential safety.

## Risks / Trade-offs

**Diagnostic code masks product failure** → Keep diagnostics read-only and add negative tests that failure classification does not mark acceptance success.

**Evidence artifacts become too large** → Preserve bounded tail/snippet summaries plus stable paths; only commit compact redacted evidence when required for archive.

**False classification due sparse logs** → Classification should include an `unknown/post-registration` fallback with the highest observed marker rather than guessing workspace/blob root cause.

## Validation Plan

1. Add or update pure classifier tests covering connectivity regression, worker registered, job assigned, workspace materialization timeout, executor-started failure, and unknown post-registration stalls.
2. Add focused VM executor or dogfood tests that preserve evidence handles and redact long tokens.
3. Run focused tests for `aspen-dogfood` and `aspen-ci-executor-vm`.
4. Run `openspec validate debug-vmci-ci-workspace-blob-stall --strict --json` and `openspec validate --all --strict --json`.
5. Run one live `nix run .#dogfood-local-vmci -- full` after cleanup; accept either a successful receipt or a classified evidence bundle that identifies the next product boundary.
