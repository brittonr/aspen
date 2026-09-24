# Verification: Restore the Tracey requirement root

Base: `origin/molten` `4e31cee55167a38978961faac5c46476aca6f5ac`.

## Baseline

- Base guard: `requirements=0 referenced=0 uncovered=0 baseline_entries=1924 dangling=820`. It fails on dangling
  references.
- Guard at `06b622da4^`: `requirements=2508 referenced=584 uncovered=1924 dangling=0 verdict=pass`. Guard at
  `06b622da4`: `requirements=0 dangling=584`. The layout migration is the breaking commit.

## Tool self-tests

- `rustc --test tools/tracey/inherited_debt_guard.rs`: 5 passed, including the new
  `missing_requirement_root_fails_closed`.
- `rustc --test tools/tracey/inherited_debt_classifier.rs`: 4 passed.

## Guard over the branch tree

- After the root fix, before new markers: `requirements=2777 referenced=787 uncovered=1990 dangling=33`,
  `unexpected_missing=66`, `stale_baseline=0`.
- After the markers: `requirements=2777 referenced=850 uncovered=1927 baseline_entries=1924 dangling=33`,
  `unexpected_missing=3`, `stale_baseline=0`.
- `nix build .#checks.x86_64-linux.inherited-tracey-debt`: exit 1 (`/home/brittonr/git/OnixResearch/target/aspen-gate-blockers/b-inherited-tracey-debt.txt`). The self-tests
  pass, and the guard stops on the 33 dangling references listed below.

## Remaining residue (not resolvable on this branch)

- 4 heading-inline CAS markers: `aspen.cas.{contract,decision,boundary,verification}`. The MODIFIED delta in this
  change restates them in standalone form. `cairn sync` preview reports one `sync_delta_spec` action and is not
  blocked. They resolve when this change syncs.
- 28 references to requirements that exist only in active change deltas. They resolve when these changes sync:
  `declare-wasm-component-import-admission` (6), `fix-node-shutdown-admission` (4),
  `resume-blocked-scheduler-runnables` (4), `enforce-scheduler-queue-bounds-on-yield` (4),
  `saturate-exponential-retry-arithmetic` (4), `exercise-simulation-faults-causally` (3),
  `add-chaoscontrol-consensus-conformance` (2), `own-remote-assertions-per-session` (1).
- 1 undefined id, `molten.fabric_simulation.causal_acknowledgment` (`src/fabric_simulation/composition.rs`). Its
  owner must decide.
- 3 accepted requirements with no implementing evidence: `molten.authority.nominal_references.octet.guard` (no Octet
  nominal-domain check exists), and `molten.modularity.fabric_boundary.compatibility` and `.compatibility.fixtures`
  (no pre-migration canonical fixture is compared). The baseline was not grown.

## Evidence regeneration

- The classifier over the branch tree gives `verdict=pass`. The TSV and summary differ from the base only by the
  `cairn/specs/` → `.cairn/specs/` prefix. The new BLAKE3 values are TSV `a6c018160bf3586a3097bbac3f337e204f96efc7ef0f0231a3f99135af744d61`
  and summary `9747a3cf303ba2b4935d4f12956314b51d37a62ce2f2e6f42d90fdd36503a4c3`.
- `evidence/tracey/*.json` were re-exported with the pinned Nickel from the updated `.ncl` metadata.
- `cairn-policy/generated/cairn-policy.json` is `nickel export cairn-policy/default.ncl`. Its diff is the five
  traceability `root` values.

## Markers

63 requirements received direct `impl` or `verify` markers. The change adds comment lines only. Each marker's
file, line, and justification is recorded in `evidence/marker-justifications.json`, which covers the
dataspace-access-cache, dev-function-profiling, authority-identity-revocation, project, content-replication, dag-sync,
nickel-toolchain, and world-commit specs.
