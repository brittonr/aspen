## Why

ChaosControl and the conformance guest cohort need a guest-runnable Molten
consensus replica. The current `raft::live_process` three-process harness is
`cfg(test)`: it uses test reference generators, a shared signal directory,
hard-coded secret bytes, and workspace-relative paths. None of that is a
durable product contract or guest input.

The adapter layer now projects real committed applies, but there is no
production shell that starts the admitted consensus service from explicit
configuration. A bounded replica command is the missing bridge between the
tested service and a packaged multi-guest cohort.

## What Changes

- Add a production `molten cluster replica` shell with an explicit canonical
  configuration and listener-key file.
- Bind the exact consistency group, membership, endpoints, durability root,
  application manifest/handler, timer/entropy/supervision profiles, and finite
  resource bounds. Reject test references, workspace-relative paths, ambient
  listeners, and implicit defaults.
- Start or recover one `ScopedLiveReplica` through the existing admitted ports,
  run a bounded ingress/event loop, publish readiness, accept stop, and emit
  canonical startup and terminal receipts.
- Expose status/readback and deterministic shutdown without granting authority,
  durability, deployment, or release claims.
- Add a native three-process test that launches the command, elects, commits,
  reads, snapshots, restarts, and fences the stale leader.
- Document the exact guest handoff inputs. This change does not package or run a
  guest image and does not claim KVM, multi-VM, campaign, or production
  admission.

## Impact

- **Files**: `src/fabric_consistency/raft/replica_shell.rs`,
  `src/fabric_consistency/raft/live_process` (shared shell input types),
  `src/cli/ops/cluster/` command wiring, config schema fixtures, focused
  integration tests, and `docs/` guest handoff notes.
- **Testing**: config admission and denial cases, three-process native
  lifecycle, stale-generation fencing, durable restart, bounded event/ingress
  behavior, status readback, clean teardown, Clippy, Octet, and Cairn gates.
- **Non-goals**: no guest image, KVM campaign, ChaosControl workload
  qualification, new consensus semantics, ambient listener discovery, host
  signing keys, or release eligibility.
