## Context

Aspen's extraction workflow now has a working contract: `docs/crate-extraction.md`, typed policy in `docs/crate-extraction/policy.ncl`, deterministic readiness checking, manifests, downstream fixtures, negative boundary evidence, and compatibility rails. That workflow has been applied to the Redb Raft KV stack and coordination, with adjacent archived changes covering protocol/wire, transport/RPC, blob/castore/cache, and KV branch/commit DAG families.

The remaining decomposition work is still large. Some candidates are prerequisites for many others, while others directly support Aspen's self-hosting goal. This change selects the next wave and defines what this active change must prove before implementation can mark readiness tasks complete.

## Goals / Non-Goals

**Goals:**

- Select the next five target families in an explicit order.
- Define the first bounded slice for each target so future implementation does not start with broad rewrites.
- Require full manifests, policy/checker entries, downstream fixtures, negative boundary tests, and compatibility evidence for each selected family.
- Preserve the existing rule that publication and repository split remain blocked until human license/publication policy is decided.

**Non-Goals:**

- Do not attempt a full extraction of every selected family in one broad rewrite; each family starts with its first blocker and stops at the manifest-defined evidence gate.
- Do not mark any selected family extraction-ready until its manifest, policy/checker, downstream fixture, negative boundary proof, positive/negative tests, and compatibility evidence are complete.
- Do not publish crates, split repositories, or rename crates for external branding.
- Do not revisit the archived Redb Raft KV, coordination, protocol/wire, transport/RPC, blob/castore/cache, or KV branch/commit-DAG extraction work except to consume their established patterns.

## Decisions

### 1. Select prerequisite and self-hosting targets first

**Choice:** The next wave is:

1. `foundational-types`
2. `auth-ticket`
3. `jobs-ci-core`
4. `trust-crypto-secrets`
5. `testing-harness`

**Rationale:** Foundational type/trait seams unblock other families. Auth/ticket portability protects client and wire surfaces. Jobs/CI core is central to Aspen's self-hosted CI goal. Trust/crypto/secrets contains reusable pure state-machine and cryptographic logic that should not depend on runtime shells. Testing harness decomposition improves the evidence loop for all later extraction work.

**Alternative:** Choose config/plugin and binary shells in this wave. Rejected for this wave because plugin/config has a narrower dependency blast radius, and binary shells should be audited continuously as final consumers rather than treated as reusable extraction targets.

### 2. Treat existing stubs as incomplete manifests

**Choice:** `foundational-types.md` and `auth-ticket.md` remain useful starting points, but future readiness work must upgrade them to the same full manifest format used by the completed service families.

**Rationale:** The stubs document first blockers, but they do not yet carry enough owner, compatibility, downstream, and negative-boundary evidence to justify readiness changes.

**Alternative:** Create brand-new manifest names for narrower subfamilies first. Rejected because the inventory already established family-level rows; narrower manifests can be added later if the family manifest identifies separable slices.

### 3. Make jobs and CI one target family

**Choice:** Combine jobs protocol/executor and CI scheduler/orchestrator/config work under `jobs-ci-core`.

**Rationale:** Aspen CI consumes jobs, worker readiness, executors, Nickel config, artifacts, and orchestration state as one self-hosting stack. Splitting them too early would hide cross-crate runtime leaks between scheduler logic and executor shells. `aspen-jobs-protocol` remains covered by the archived protocol/wire contract for generic serialization stability; this wave includes it only to track domain-schema compatibility while jobs/CI core seams move.

**Alternative:** Create separate `jobs-core` and `ci-core` targets now. Rejected because the first blocker is the seam between shared run-state/config/scheduler logic and runtime worker/executor shells; proving that seam once gives clearer evidence.

### 4. Keep trust crypto separate from runtime secrets service

**Choice:** `trust-crypto-secrets` must separate pure cryptographic/state-machine logic from Raft log application, Iroh share exchange, startup probing, and secrets service storage/runtime shells.

**Rationale:** The reusable core should be deterministic, testable without networking, and suitable for Verus/property testing. Runtime shells remain Aspen-specific because they depend on membership, transport, storage, and bootstrap behavior.

**Alternative:** Extract all of `aspen-secrets` with trust. Rejected because secrets service behavior includes runtime persistence and cluster orchestration that should stay behind adapters until the pure core is proven.

### 5. Make testing harness a product of decomposition, not only support work

**Choice:** Treat `testing-harness` as a selected target with its own manifest and policy entry.

**Rationale:** Many future extractions need downstream fixtures, madsim, network, patchbay, and failure-injection helpers. If these helpers remain tied to Aspen cluster bootstrap, evidence for reusable crates will keep carrying app-runtime assumptions.

**Alternative:** Leave testing utilities as internal support code. Rejected because the extraction process itself depends on reusable, bounded, deterministic test harnesses.

## Target Slices

| Family | First slice | Verification emphasis |
| --- | --- | --- |
| `foundational-types` | Move `SM_KV_TABLE` / `redb::TableDefinition` out of portable storage types; split `aspen-traits` into narrower capability traits with compatibility composite. | no-default/default compile, cargo tree leak checks, representative consumer feature-unification checks. |
| `auth-ticket` | Migrate portable consumers to `aspen-auth-core` / `aspen-hooks-ticket`; pin token/ticket serialization compatibility. | encode/decode goldens, negative runtime API availability checks, runtime-shell compatibility. |
| `jobs-ci-core` | Separate scheduler/config/run-state logic from worker/executor runtime shells and concrete process/Nix execution. | downstream scheduler fixture, Nickel config fixtures, executor-shell negative boundary checks, CI/jobs consumer compatibility. |
| `trust-crypto-secrets` | Extract pure Shamir/GF/HKDF/share-chain/reconfiguration state logic from Raft/Iroh/secrets service shells. | property/Verus-ready pure tests, malformed share/key negative tests, runtime trust/secrets compatibility. |
| `testing-harness` | Split reusable simulation/workload/assertion helpers from Aspen cluster boot helpers. | fixture self-tests, negative app-bootstrap dependency checks, compatibility with existing madsim/network/patchbay suites. |

## Risks / Trade-offs

- **[Risk] Five targets create too much scope for one implementation change** → Mitigation: this OpenSpec only selects and constrains the wave; each family can still become its own implementation OpenSpec or task slice.
- **[Risk] Foundational trait changes ripple through many crates** → Mitigation: require compatibility composites, representative consumer checks, and first-slice evidence before readiness changes.
- **[Risk] Jobs/CI merges too many concerns** → Mitigation: manifest must name sub-surfaces and first blocker; implementation can split follow-up tasks once the scheduler/runtime seam is explicit.
- **[Risk] Security-sensitive trust/auth extraction loses invariants** → Mitigation: require positive and negative serialization/crypto tests plus pure-core verification rails before runtime integration changes.
- **[Risk] Testing harness extraction hides app dependencies in fixtures** → Mitigation: downstream metadata and checker mutations must reject cluster bootstrap and root-app dependencies in reusable harness defaults.

## Migration Plan

1. Land the OpenSpec artifacts so the active change has a reviewable contract.
2. For each selected family, create or upgrade the manifest and policy entry before moving code.
3. Capture baseline dependency and source-import evidence for that family.
4. Implement the first blocker only, with compatibility paths for existing Aspen consumers.
5. Save compile, cargo tree, downstream fixture, negative boundary, positive/negative test, and compatibility evidence before checking tasks complete.

## Open Questions

- Whether `config/plugin` should be the sixth wave or folded into `jobs-ci-core` through Nickel CI config evidence.
- Whether `testing-harness` should split into separate `simulation-harness` and `integration-fixtures` families after the baseline source inventory is captured.
