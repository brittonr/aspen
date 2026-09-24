# Proposal: Restore the Tracey requirement root

## Why

`checks.x86_64-linux.inherited-tracey-debt` fails on `origin/molten` (`4e31cee55`) with `requirements=0`,
`dangling=820`. The lifecycle layout migration in `06b622da4` (2026-08-19) moved accepted specifications from
`cairn/specs` to `.cairn/specs`. `tools/tracey/inherited_debt_guard.rs` and `inherited_debt_classifier.rs` still read
`cairn/specs`, and the guard treated a missing root as an empty requirement set. From that commit on, the guard saw no
requirements and flagged every evidence reference as dangling. It passed at `06b622da4^`.

With the root restored, the guard shows debt that accumulated while it was blind. Thirty-three references are dangling.
Four are CAS requirements whose markers are written inline in headings, which the guard does not discover by design.
Twenty-eight reference requirements that exist only in active change deltas. One references an id defined nowhere.
Sixty-six requirements accepted between 2026-08-24 and 2026-08-27 have no evidence reference at all.

Other traceability paths still use the old layout: `molten test traceability scan` (`src/cli/ops/traceability.rs`),
its Nix gate fixture, the vendored Cairn traceability profile roots, and README.

## What Changes

- Point the guard and the classifier at `.cairn/specs`. Make both fail closed when the requirement root is missing,
  and make the guard also fail when the root contains no requirements.
- Regenerate the classification inventory and summary. The only change is the `.cairn/specs/` prefix. Update the
  bound digests and path metadata in `evidence/tracey/*.ncl` and `*.json`.
- Add direct evidence markers for 63 of the 66 unmarked requirements, at the code, test, document, or Nix check that
  implements or verifies each one.
- Repair the four `aspen.cas.*` heading-inline markers to the standalone accepted form through a MODIFIED delta,
  without changing semantics, as `molten.project.inherited_tracey_debt.marker_repair` requires.
- Point `molten test traceability scan`, the `requirement-traceability-gate` fixture, the vendored `cairn-default`
  and local traceability profile roots (with `UPSTREAM.md` and the regenerated policy JSON), and README at the
  `.cairn/` layout.

## Impact

- **Files**: `tools/tracey/*.rs`, `evidence/tracey/*`, `src/cli/ops/traceability.rs`, `flake.nix`,
  `cairn-policy/{default.ncl,UPSTREAM.md,generated/cairn-policy.json}`, `README.md`, and comment-only marker lines in
  source, test, document, and Nix files.
- **Testing**: guard and classifier self-tests, including a missing-root negative; the guard over the tree;
  `inherited-tracey-debt`, `requirement-traceability-gate`, and `contract-export-drift-gate`.

## Out of Scope

- Syncing other active changes. Twenty-eight dangling references resolve when their owning changes sync:
  `declare-wasm-component-import-admission`, `exercise-simulation-faults-causally`,
  `own-remote-assertions-per-session`, `fix-node-shutdown-admission`, `saturate-exponential-retry-arithmetic`,
  `resume-blocked-scheduler-runnables`, `enforce-scheduler-queue-bounds-on-yield`, and
  `add-chaoscontrol-consensus-conformance`.
- `molten.fabric_simulation.causal_acknowledgment` is defined nowhere. Its owner must add it to a delta or retarget
  the marker.
- `molten.authority.nominal_references.octet.guard`, `molten.modularity.fabric_boundary.compatibility`, and
  `molten.modularity.fabric_boundary.compatibility.fixtures` have no implementing check or test. The baseline must not
  grow, so each needs implementation or an owner decision.
