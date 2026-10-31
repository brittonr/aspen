# Baseline boundaries, dependency admission and QM reference (tasks 1.1–1.3)

Recorded 2026-10-31 in worktree `/home/brittonr/.cargo-target/aspen-collab-20261031`
(branch `cairn/collaboration-scope-closeout-20261031`, base `c267d062c`).

## How this record was produced

Task 1.1 asks for a record taken *before* implementation. No such record was written at the
time: the proposal was authored on 2026-08-15 (`06b622da4`) and the implementation arrived in
integration commit `ec1f3d7f4` (2026-10-30). This record is reconstructed afterwards from the
last mainline commit without any collaboration source, `d7e435e711f7f58167900ce7419121390b0842d3`
(`git ls-tree d7e435e71 src/collaborationscope crates/molten-core/src/collaborationscope` is
empty; the same command at `ec1f3d7f4` lists both directories). It describes that baseline; it is
not a contemporaneous pre-implementation snapshot.

## 1.1 Boundaries at the baseline

Each boundary existed at `d7e435e71` under the listed owner and accepted specification, and its
archived introduction precedes this change. The extension consumes these boundaries; it owns
none of them.

| Boundary | Owner at baseline | Accepted spec / archive |
|---|---|---|
| Fabric node membership, placement, fencing | `src/fabric_membership` | `.cairn/specs/fabric-membership-placement`; `2026-07-12-fabric-membership-placement-runtime` |
| Authority (identity, revocation) | `src/authority` | `.cairn/specs/authority-identity-revocation` |
| Resources and backpressure | `src/resources` | `.cairn/specs/resource-governance-backpressure` |
| Consistency service | `src/fabric_consistency` | `2026-07-24-fabric-consistency-service-runtime` |
| Durable state | `src/fabric_durability` | `.cairn/specs/durable-state-ports`; `2026-07-12-fabric-durable-state-ports` |
| Logical time | `src/fabric_time` | `.cairn/specs/fabric-time-scheduling`; `2026-07-11-fabric-time-scheduler-runtime` |
| System extensions | `src/system_extension` | `.cairn/specs/system-extension-runtime`; `2026-07-11-system-extension-service-runtime` |
| Whole-system simulation | `src/fabric_simulation` | `.cairn/specs/fabric-simulation`; `2026-08-01-fabric-whole-system-simulation` |

Fabric membership is unchanged by this change: `git diff --stat d7e435e71 c267d062c --
src/fabric_membership` prints nothing. Collaboration membership lives only in the extension's
own records (`molten_core::collaboration_scope::{CollaborationScope, Membership}`).

## 1.2 Admitted dependencies

| Dependency | Identity | Where |
|---|---|---|
| Nominal references | `molten_core::nominal` (`PolicyRef`, `OperationRef`, `ResourceRef`, `RevocationRef`, `EvidenceRef`) | archive `2026-08-24-type-authority-and-artifact-references`, `1970-01-01-adopt-octet-nominal-domain-enforcement` |
| Installed-extension simulation | `fabric_simulation` installed-workload admission | archive `1970-01-01-admit-installed-extension-simulation` (archived in `ec1f3d7f4`) |
| Basalt | crate `basalt 0.1.0`, `git+ssh://git@github.com/OnixResearch/basalt.git?rev=89675cd4f585f837323c049e4a25f7b94c903038` | `Cargo.toml`, `Cargo.lock`; flake input `basalt-src` (`narHash sha256-lH3qGZ/MssqWR9cIJeyv8xcSO42IfVkvOHsn+O/ddwg=`) |
| UCAN | crates `ucan 0.1.0`, `ucan-core 0.1.0` from the same Basalt revision `89675cd4…` | `Cargo.lock` |

Both external consumer identities are exact Git revisions recorded in `Cargo.lock`; neither is a
workspace-relative path. They are verification mechanisms only: Basalt and UCAN correctness is a
non-claim of this change.

## 1.3 QM reference

- Source: `https://github.com/yc-software/qm` at commit
  `7f2c916360f1797a8ff2a77ce2ce40c5fabab087`. Fetched on 2026-10-31 with
  `curl -fsSL https://raw.githubusercontent.com/yc-software/qm/7f2c916360f1797a8ff2a77ce2ce40c5fabab087/<file>`:

  | File | Bytes | sha256 |
  |---|---|---|
  | `LICENSE` | 1072 | `586fe8965302b57c7a490a42911f847f0dba54e60d95df53f3ad53ed9eed6b83` |
  | `README.md` | 8534 | `9588226b6b19b354f125d805b18b3a585ccbe76ae04800d719f978dc5cce51ca` |
  | `SECURITY.md` | 9887 | `d2c00b7d5977c16172fa6a2a1af2f48117cbe71bf3eb193193ca00d303d763ed` |

- License: `LICENSE` is the MIT License, "Copyright (c) 2026 QM contributors".
- Reviewed mechanisms (requirements only): personal and shared scopes with per-scope isolation
  (README "Personal and shared scopes"; SECURITY "Scope"), an org-wide security posture that
  narrower scopes may only tighten (README), scope-owned resources shareable only by explicit
  grant (README "Shared skills"), and the goal of preventing unauthorized cross-scope reads,
  writes and deliveries (SECURITY).
- Rejected mechanisms: QM's Postgres persistence layout, its TypeScript implementation,
  capability links that authorize outside visitors (SECURITY "Scope"), credential
  materialization inside per-scope sandboxes, and its classifier-based content screening.
- Non-transfer boundary: no QM code, test or fixture was copied; no QM test was run. QM's
  security goals and behavior are not evidence for Molten; Molten's claims rest only on its own
  executed checks.
