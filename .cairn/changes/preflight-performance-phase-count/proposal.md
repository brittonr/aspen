# Preflight reviewed benchmark phase group cardinality

## Why

The public `build_benchmark_run` checks a suite whose reviewed phase
inventory is exactly compilation, instantiation, execution, but sorts
and validates arbitrary caller-built `input.phases` before that count
is enforced. Public `validate_benchmark_run` also scans every forged
`run.phases` entry and formats its hash before checking a phase bound.
More than three groups can grow diagnostics and work contrary to the
existing fixed suite contract.

## What Changes

- Reject more phase groups than the reviewed suite's three phases
  before either public path normalizes, scans, or formats those groups.
- Keep the producer's existing profile/suite/input validation first;
  keep existing normalization order, missing/duplicate, malformed
  phase, sample and run-identity errors for admitted inputs.
- Reuse the suite's reviewed phase count, not a new arbitrary limit;
  zero through three entries retain their existing shape validation.

## Impact

One isolated performance owner from clean canonical Aspen, two public
functions, focused failing-before/passing-after regressions, compiled
`molten::wasm_performance` caller, handoff, and a change-local Cairn
delta. Compose only this disjoint slice above 66 existing packages.
No production CLI path constructs benchmark runs internally. No
accepted spec, waiver, policy, baseline, pin or retry of the known
failing full strict Octet gate is requested.
