## Why

Aspen has already proven the crate-extraction workflow on the Redb Raft KV stack, coordination, and several adjacent runtime/service families. The next decomposition wave needs an explicit OpenSpec so the work continues in dependency order instead of picking large crates ad hoc.

## What Changes

- Select the next five crate-decomposition target families and record why they are next.
- Require each selected family to get a complete extraction manifest, policy entry, standalone verification rails, downstream-style fixture, negative boundary proof, and compatibility evidence before any readiness state is raised.
- Add ordering rules so prerequisite seams, especially foundational type and trait boundaries, land before higher-level jobs, CI, trust, and test-harness work depends on them.
- Keep publication and repository split out of scope until human license/publication policy is decided; selected families may not advance beyond `workspace-internal` or `extraction-ready-in-workspace` readiness labels in this change.

Selected next wave:

1. **Foundational types/helpers**: `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, `aspen-constants`.
2. **Auth and tickets**: `aspen-auth-core`, runtime shell `aspen-auth`, `aspen-ticket`, `aspen-hooks-ticket`.
3. **Jobs and CI core**: `aspen-jobs`, `aspen-jobs-protocol`, `aspen-jobs-guest`, `aspen-jobs-worker-blob`, `aspen-jobs-worker-maintenance`, `aspen-jobs-worker-replication`, `aspen-jobs-worker-shell`, `aspen-jobs-worker-sql`, `aspen-ci-core`, `aspen-ci`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`.
4. **Trust, crypto, and secrets**: `aspen-trust`, `aspen-crypto`, `aspen-secrets` reusable pure/state-machine surfaces, with `aspen-secrets-handler` retained as a runtime consumer.
5. **Testing harness**: `aspen-testing-core`, `aspen-testing`, `aspen-testing-fixtures`, `aspen-testing-madsim`, `aspen-testing-network`, `aspen-testing-patchbay`.

Deferred candidates:

- **Config/plugin APIs**: `aspen-nickel` and `aspen-plugin-api` remain important, but this wave touches Nickel through CI config first and defers standalone plugin/config extraction until the jobs/CI seam identifies the stable config/plugin boundary.
- **Binary shells**: `aspen-cli`, `aspen-tui`, node binaries, bridges, gateways, web, and dogfood remain final consumers. This wave verifies they stay thin and compatible, but does not treat them as reusable extraction targets.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `architecture-modularity`: Adds requirements for the next five decomposition targets, their ordering, manifest completeness, policy/checker coverage, and evidence gates.

## Impact

- **Docs/specs**: Extends the crate-extraction roadmap and architecture-modularity requirements.
- **Code later**: Implementation under this change will touch the selected crate families and `docs/crate-extraction/policy.ncl` in bounded first-blocker slices.
- **Dependencies later**: Reusable defaults must avoid root `aspen`, handlers, binary shells, concrete runtime adapters, trust/secrets/SQL/coordination dependencies unless the selected manifest documents a named adapter feature or backend-purpose exception.
- **Testing later**: Every selected family needs positive compile/usage evidence plus negative tests or checker mutations proving forbidden app/runtime dependencies are rejected.

## Task Traceability

- `R1` / `R2`: Capture selection and per-family baseline evidence for `architecture.modularity.next-decomposition-wave-is-selected` and `architecture.modularity.next-decomposition-first-blockers-are-explicit`.
- `I1` / `I2`: Update roadmap and full manifests for `architecture.modularity.next-decomposition-manifests-are-complete`.
- `I3` / `V1`: Extend policy/checker coverage and negative mutations for `architecture.modularity.next-decomposition-policy-covers-wave`.
- `I4` through `I17` plus `V2` through `V4`: Carry first-blocker implementation, downstream fixtures, compatibility rails, and positive/negative tests for `architecture.modularity.next-decomposition-standalone-and-compatibility-proof` during implementation of this active change.
- `V5`: Enforce verification.md, evidence paths, preflight, and gate transcript discipline before any task is checked.
