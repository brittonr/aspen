# Design: Avoid VMCI Nix Store FD Pressure

## Context

The live VMCI debug ladder has narrowed the issue. Recent medium receipts prove:

- VMCI local startup/readiness works.
- Forge source push and CI auto-trigger work.
- The bounded source snapshot overlays tracked WIP.
- Shell/Nix payloads carry `source_hash` and materialize the workspace.
- VM worker reaches `job_spec_parse_done`, `nix_payload_transform_done`, `executor_enter`, `workspace_materialization_done`, and `command_started`.
- Preserving lock `original` fixed GitHub `HEAD` refresh.
- Selectively rewriting `tigerstyle` fixed the broad private-input intent but did not remove the FD-pressure failure.

The remaining failure happens inside guest Nix after command start while it copies/unpacks/chmods large source inputs, especially `nixpkgs`, with `Too many open files in system`.

## Goals / Non-Goals

**Goals:**

- Stop guest VMCI Nix from traversing/copying giant public source inputs through host-backed virtiofs `/nix/store`.
- Keep private/offline input support for `tigerstyle`/Octet without reintroducing GitHub `HEAD` refresh or `narHash` mismatch.
- Produce deterministic diagnostics that distinguish this boundary from startup, route, source blob, workspace, and timeout failures.
- Prove with `dogfood-local-vmci-medium` before clippy/full.

**Non-Goals:**

- Blanket FD-limit increases as the primary fix.
- Making VMCI builds depend on host checkout access.
- Solving every Nix cache optimization in one change.
- Exposing secrets or unbounded command/env data in diagnostics.

## Decisions

### 1. Treat large public inputs as cache-native/guest-local, not host path inputs

**Choice:** VMCI input preparation will classify inputs. Only explicit private/offline inputs can be rewritten to `path`; large public inputs such as `nixpkgs` must stay on locked fetchers or be made available through a guest-local/cache-native mechanism that avoids host virtiofs tree traversal.

**Rationale:** The failed medium showed that even after only `tigerstyle` rewrite, guest Nix still copied/unpacked a large `nixpkgs` source path and hit system-wide FD pressure. Broad host path exposure and host-store traversal are the wrong boundary for giant source trees.

**Alternative rejected:** Raise host `virtiofsd`/guest nofile again. That improves headroom but does not remove the unbounded traversal shape and has already failed to clear the proof rail.

**Implementation notes:**

- Preserve existing selective rewrite tests.
- Add input classification helpers with explicit names and bounded default lists.
- Ensure Nix command/config does not implicitly force substituter source copies through host store mounts for public source inputs when VMCI profile is active.
- Evaluate implementation options in this order:
  1. guest-local Nix cache/store for fetched source inputs;
  2. host cache proxy/substituter path that streams NARs without virtiofs tree walk;
  3. Aspen/snix/castore import to guest-local store with bounded concurrency.

### 2. Add a diagnostic class for source/store FD pressure

**Choice:** Dogfood/diagnostics classify `Too many open files in system` with Nix source copy/unpack/chmod/read context as `vmci_nix_source_fd_pressure` or equivalent.

**Rationale:** The operator should not repeat VMCI route/source/workspace fixes once markers prove command start. The current evidence needs an unambiguous boundary label.

**Implementation notes:** Match bounded stderr tails for:

- `Too many open files in system`
- nearby `copying path '/nix/store/...-source'`
- nearby `unpacking 'github:...'`
- nearby `chmod` or `reading directory` under `/nix/store/...-source`

Retain only safe basename/hash snippets and marker/job ids.

### 3. Medium rail remains the gate

**Choice:** Do not escalate to clippy/full until medium either passes or produces a non-FD classified failure.

**Rationale:** Medium exercises the same VMCI Forge/source/CI/job path with lower cost and has already reproduced the root failure.

## Risks / Trade-offs

**Network/cache flakiness** → Prefer cache-native paths that honor lock/narHash and keep retry/timeout bounds. If network is used, diagnostics must distinguish network failure from FD pressure.

**Private input regression** → Keep `tigerstyle` selective path rewrite tests and the synthetic lock-original proof.

**False pass by skipping the real path** → Medium must still run real `build-cli` in VMCI, not a synthetic no-op.

**Overbroad diagnostics** → Classifier must require both `Too many open files` and Nix source/store context so unrelated FD exhaustion is not mislabeled.

## Verification Plan

1. Unit tests for input classification and lock rewrite: public inputs are not rewritten to path; private/offline inputs can be.
2. Unit tests or fixture tests for VMCI Nix command/store strategy showing large public inputs are resolved guest-local/cache-native without host path rewrite.
3. Diagnostic tests for the exact latest stderr shape, verifying classification and redaction.
4. Existing VMCI dogfood tests for rail selection and source payload propagation.
5. Live `nix run .#dogfood-local-vmci-medium` receipt. Only after it clears the FD-pressure signature should clippy/full be considered.
